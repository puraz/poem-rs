use std::time::Duration;

use anyhow::Result;
use iced::widget::{column, container, mouse_area, row, text};
use iced::{Element, Length, Size, Task, Theme, window};
use tracing_subscriber::EnvFilter;

use crate::config::app::AppPaths;
use crate::storage::{AppDatabase, StoredAiConfig};

use super::components::{
    ButtonKind, SurfaceKind, ToastTone, action_button, compact_button, modal_overlay, nav_button,
    nav_surface, page_shell, section_surface, surface, toast, toast_host,
};
use super::message::{ContentMode, Message, Modal, ThemeChoice};
use super::screens::{about_modal, discovery_modal, edit_modal, library, settings_modal};
use super::state::{AppState, SettingsForm};
use super::task;
use super::theme;

pub fn run() -> Result<()> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"))
        .add_directive("icu_provider=error".parse().unwrap())
        .add_directive("icu_segmenter=error".parse().unwrap());
    let _ = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .try_init();

    iced::application(PoemApp::new, PoemApp::update, PoemApp::view)
        .theme(PoemApp::theme)
        .window(window::Settings {
            size: Size::new(1460.0, 920.0),
            min_size: Some(Size::new(1240.0, 780.0)),
            position: window::Position::Centered,
            ..Default::default()
        })
        .run()?;

    Ok(())
}

struct PoemApp {
    paths: AppPaths,
    db: AppDatabase,
    ai_config: StoredAiConfig,
    state: AppState,
}

impl PoemApp {
    fn new() -> (Self, Task<Message>) {
        let paths = AppPaths::resolve().expect("failed to resolve app paths");
        let db = AppDatabase::new(paths.db_path());
        db.bootstrap().expect("failed to bootstrap database");
        let ai_config = db.load_ai_config().expect("failed to load ai config");
        let poems = db.list_poems().expect("failed to load poems");
        let selected_poem_id = poems.first().map(|poem| poem.id.clone());
        let settings_form = SettingsForm::from_stored(&paths, &ai_config);
        let active_theme =
            ThemeChoice::from_saved(db.load_theme_preference().ok().flatten().as_deref());
        let state = AppState::new(poems, selected_poem_id, settings_form, active_theme);

        let mut app = Self {
            paths,
            db,
            ai_config,
            state,
        };
        app.refresh_cached_appreciation();

        (app, Task::none())
    }

    fn theme(&self) -> Theme {
        theme::app_theme(self.state.active_theme)
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SelectPoem(poem_id) => {
                self.state.selected_poem_id = Some(poem_id);
                self.refresh_cached_appreciation();
                Task::none()
            }
            Message::SearchChanged(query) => {
                self.state.search_query = query;
                self.state.sync_selection();
                self.refresh_cached_appreciation();
                Task::none()
            }
            Message::SwitchContentMode(mode) => {
                let next_mode = if self.state.content_mode == mode {
                    ContentMode::Library
                } else {
                    mode
                };
                self.state.switch_content_mode(next_mode);
                self.refresh_cached_appreciation();
                Task::none()
            }
            Message::OpenModal(modal) => {
                match modal {
                    Modal::Settings => {
                        self.state.settings_form = SettingsForm::rehydrated(
                            SettingsForm::from_stored(&self.paths, &self.ai_config),
                            String::new(),
                        );
                        self.state.open_modal(Modal::Settings);
                    }
                    Modal::Edit => self.state.open_edit_for_selected(),
                    _ => self.state.open_modal(modal),
                }
                Task::none()
            }
            Message::CloseModal => {
                if self.state.active_modal == Modal::Edit {
                    self.state.close_edit();
                } else {
                    self.state.close_modal();
                }
                Task::none()
            }
            Message::ToggleFavorite => {
                let Some(poem_id) = self.state.selected_poem_id.clone() else {
                    return Task::none();
                };
                match self.db.toggle_favorite(&poem_id) {
                    Ok(is_favorite) => {
                        self.reload_poems();
                        self.refresh_cached_appreciation();
                        let label = if is_favorite {
                            "已加入收藏夹"
                        } else {
                            "已取消收藏"
                        };
                        let revision = self.state.toast.show(label);
                        dismiss_toast_later(revision)
                    }
                    Err(err) => {
                        let revision = self.state.toast.show(format!("收藏失败: {err}"));
                        dismiss_toast_later(revision)
                    }
                }
            }
            Message::DiscoveryQueryChanged(query) => {
                self.state.discovery_query = query;
                Task::none()
            }
            Message::SubmitDiscovery => {
                if self.state.discovery_query.trim().is_empty() {
                    self.state.discovery_loading = false;
                    self.state.discovery_status = "请输入关键词".to_string();
                    return Task::none();
                }
                self.state.discovery_loading = true;
                self.state.discovery_status.clear();
                self.state.active_modal = Modal::Discovery;
                let paths = self.paths.clone();
                let config = self.ai_config.clone();
                let query = self.state.discovery_query.clone();
                Task::perform(
                    task::run_discovery_search(paths, config, query),
                    Message::DiscoveryLoaded,
                )
            }
            Message::DiscoveryLoaded(result) => {
                self.state.discovery_loading = false;
                match result {
                    Ok(items) => {
                        self.state.discovery_results = items;
                        self.state.discovery_status = if self.state.discovery_results.is_empty() {
                            "AI 没有返回可用诗词，请换个关键词再试。".to_string()
                        } else {
                            String::new()
                        };
                    }
                    Err(message) => {
                        self.state.discovery_results.clear();
                        self.state.discovery_status = message;
                    }
                }
                Task::none()
            }
            Message::ImportDiscovery(index) => {
                if let Some(poem) = self.state.discovery_results.get(index).cloned() {
                    return Task::perform(
                        task::import_discovery_poem(self.db.clone(), poem),
                        Message::ImportFinished,
                    );
                }
                Task::none()
            }
            Message::ImportFinished(result) => match result {
                Ok(imported) => {
                    self.reload_poems();
                    self.state.selected_poem_id = Some(imported.poem_id);
                    self.state.switch_content_mode(ContentMode::Library);
                    self.state.close_modal();
                    self.refresh_cached_appreciation();
                    let revision = self
                        .state
                        .toast
                        .show(format!("已添加《{}》到诗库", imported.title));
                    dismiss_toast_later(revision)
                }
                Err(message) => {
                    self.state.discovery_status = message;
                    Task::none()
                }
            },
            Message::SettingsBaseUrlChanged(value) => {
                self.state.settings_form.base_url = value;
                Task::none()
            }
            Message::SettingsModelChanged(value) => {
                self.state.settings_form.model = value;
                Task::none()
            }
            Message::SettingsApiKeyChanged(value) => {
                self.state.settings_form.api_key = value;
                Task::none()
            }
            Message::SettingsFallbackChanged(value) => {
                self.state.settings_form.allow_file_fallback = value;
                Task::none()
            }
            Message::SaveSettings => {
                self.ai_config.settings = self.state.settings_form.into_settings();
                self.ai_config.allow_file_fallback = self.state.settings_form.allow_file_fallback;
                let api_key = self.state.settings_form.api_key.clone();
                Task::perform(
                    task::save_settings(
                        self.paths.clone(),
                        self.db.clone(),
                        self.ai_config.clone(),
                        api_key,
                    ),
                    Message::SettingsSaved,
                )
            }
            Message::SettingsSaved(result) => match result {
                Ok(saved) => {
                    self.state.settings_form = SettingsForm::rehydrated(
                        SettingsForm::from_stored(&self.paths, &self.ai_config),
                        saved.warning,
                    );
                    let revision = self.state.toast.show(saved.message);
                    dismiss_toast_later(revision)
                }
                Err(message) => {
                    self.state.settings_form.warning = message;
                    Task::none()
                }
            },
            Message::ClearApiKey => Task::perform(
                task::clear_api_key(self.paths.clone()),
                Message::ApiKeyCleared,
            ),
            Message::ApiKeyCleared(result) => match result {
                Ok(saved) => {
                    self.state.settings_form = SettingsForm::rehydrated(
                        SettingsForm::from_stored(&self.paths, &self.ai_config),
                        saved.warning,
                    );
                    let revision = self.state.toast.show(saved.message);
                    dismiss_toast_later(revision)
                }
                Err(message) => {
                    self.state.settings_form.warning = message;
                    Task::none()
                }
            },
            Message::OpenEditModal => {
                self.state.open_edit_for_selected();
                Task::none()
            }
            Message::EditTitleChanged(value) => {
                if let Some(form) = &mut self.state.edit_form {
                    form.title = value;
                }
                Task::none()
            }
            Message::EditAuthorChanged(value) => {
                if let Some(form) = &mut self.state.edit_form {
                    form.author = value;
                }
                Task::none()
            }
            Message::EditDynastyChanged(value) => {
                if let Some(form) = &mut self.state.edit_form {
                    form.dynasty = value;
                }
                Task::none()
            }
            Message::EditContentChanged(value) => {
                if let Some(form) = &mut self.state.edit_form {
                    form.content = value;
                }
                Task::none()
            }
            Message::SaveEdit => {
                if let Some(form) = self.state.edit_form.clone() {
                    return Task::perform(
                        task::save_edited_poem(self.db.clone(), form),
                        Message::EditSaved,
                    );
                }
                Task::none()
            }
            Message::EditSaved(result) => match result {
                Ok(edited) => {
                    self.reload_poems();
                    self.state.selected_poem_id = Some(edited.poem_id);
                    self.state.close_edit();
                    self.refresh_cached_appreciation();
                    let revision = self.state.toast.show(format!("《{}》已保存", edited.title));
                    dismiss_toast_later(revision)
                }
                Err(message) => {
                    let revision = self.state.toast.show(message);
                    dismiss_toast_later(revision)
                }
            },
            Message::RequestAppreciation => {
                let Some(poem) = self.state.selected_poem() else {
                    return Task::none();
                };
                self.state.appreciation.poem_id = Some(poem.id.clone());
                self.state.appreciation.loading = true;
                self.state.appreciation.error.clear();
                let paths = self.paths.clone();
                let db = self.db.clone();
                let config = self.ai_config.clone();
                Task::perform(
                    task::generate_and_persist_appreciation(paths, db, config, poem),
                    Message::AppreciationLoaded,
                )
            }
            Message::AppreciationLoaded(result) => {
                self.state.appreciation.loading = false;
                match result {
                    Ok(payload) => {
                        self.state.appreciation.poem_id = Some(payload.poem_id);
                        self.state.appreciation.content = payload.content;
                        self.state.appreciation.error.clear();
                    }
                    Err(message) => {
                        self.state.appreciation.error = message;
                        self.state.appreciation.content.clear();
                    }
                }
                Task::none()
            }
            Message::SwitchTheme(choice) => {
                self.state.active_theme = choice;
                let _ = self.db.save_theme_preference(choice.as_str());
                Task::none()
            }
            Message::DismissToast => {
                self.state.toast.dismiss();
                Task::none()
            }
            Message::ToastExpired(revision) => {
                self.state.toast.dismiss_if_current(revision);
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let shell = row![
            self.sidebar_view(),
            container(self.middle_pane())
                .width(Length::FillPortion(5))
                .height(Length::Fill),
            container(self.detail_pane())
                .width(Length::FillPortion(3))
                .height(Length::Fill),
        ]
        .spacing(24)
        .height(Length::Fill);

        let base: Element<'_, Message> = page_shell(shell).into();
        let with_modal = match self.state.active_modal {
            Modal::None => base,
            Modal::Discovery => modal_overlay(
                base,
                discovery_modal::view(
                    &self.state.discovery_query,
                    self.state.discovery_loading,
                    &self.state.discovery_status,
                    self.state.discovery_items(),
                ),
                Some(Message::CloseModal),
            ),
            Modal::Settings => modal_overlay(
                base,
                settings_modal::view(&self.state.settings_form),
                Some(Message::CloseModal),
            ),
            Modal::About => modal_overlay(base, about_modal::view(), Some(Message::CloseModal)),
            Modal::Edit => {
                if let Some(form) = &self.state.edit_form {
                    modal_overlay(base, edit_modal::view(form), Some(Message::CloseModal))
                } else {
                    base
                }
            }
        };

        if self.state.toast.visible {
            toast_host(
                with_modal,
                Some(
                    mouse_area(toast(
                        Some("提示"),
                        self.state.toast.message.clone(),
                        ToastTone::Success,
                    ))
                    .on_press(Message::DismissToast)
                    .into(),
                ),
            )
        } else {
            with_modal
        }
    }

    fn sidebar_view(&self) -> Element<'_, Message> {
        let theme_switch = row![
            compact_button(
                "松烟笺",
                if self.state.active_theme == ThemeChoice::Songyanjian {
                    ButtonKind::NavActive
                } else {
                    ButtonKind::Ghost
                }
            )
            .on_press(Message::SwitchTheme(ThemeChoice::Songyanjian)),
            compact_button(
                "寒江雪",
                if self.state.active_theme == ThemeChoice::Hanjiangxue {
                    ButtonKind::NavActive
                } else {
                    ButtonKind::Ghost
                }
            )
            .on_press(Message::SwitchTheme(ThemeChoice::Hanjiangxue)),
        ]
        .spacing(8);

        nav_surface(
            column![
                action_button("发现新诗词", ButtonKind::Primary)
                    .width(Length::Fill)
                    .on_press(Message::OpenModal(Modal::Discovery)),
                nav_button("首页", self.state.content_mode == ContentMode::Library)
                    .on_press(Message::SwitchContentMode(ContentMode::Library)),
                nav_button("收藏夹", self.state.content_mode == ContentMode::Favorites)
                    .on_press(Message::SwitchContentMode(ContentMode::Favorites)),
                nav_button("关于", self.state.active_modal == Modal::About)
                    .on_press(Message::OpenModal(Modal::About)),
                iced::widget::Space::new().height(Length::Fill),
                text("主题").size(13),
                theme_switch,
                nav_button("设置", self.state.active_modal == Modal::Settings)
                    .on_press(Message::OpenModal(Modal::Settings)),
            ]
            .spacing(16),
        )
        .width(280)
        .height(Length::Fill)
        .into()
    }

    fn middle_pane(&self) -> Element<'_, Message> {
        let title = if self.state.content_mode == ContentMode::Favorites {
            "我的收藏"
        } else {
            "诗词列表"
        };

        library::view(
            self.state.visible_poems(),
            self.state.selected_poem_id.as_deref(),
            &self.state.search_query,
            title,
        )
    }

    fn detail_pane(&self) -> Element<'_, Message> {
        let Some(poem) = self.state.selected_poem() else {
            return surface(
                column![
                    text("暂无诗词").size(28),
                    text("当前模式下没有可展示的内容。").size(16),
                ]
                .spacing(12),
                SurfaceKind::Raised,
            )
            .into();
        };

        let favorite_label = if poem.is_favorite {
            "★ 收藏中"
        } else {
            "☆ 收藏"
        };
        let appreciation_card = if self.state.appreciation.loading {
            surface(text("AI 正在生成赏析…").size(15), SurfaceKind::Accent)
        } else if !self.state.appreciation.error.is_empty() {
            surface(
                text(self.state.appreciation.error.clone()).size(15),
                SurfaceKind::Accent,
            )
        } else if !self.state.appreciation.content.is_empty()
            && self.state.appreciation.poem_id.as_deref() == Some(poem.id.as_str())
        {
            container(section_surface(
                "AI 赏析",
                text(self.state.appreciation.content.clone()).size(16),
                SurfaceKind::Accent,
            ))
            .into()
        } else {
            container(iced::widget::Space::new()).into()
        };

        section_surface(
            "阅读",
            column![
                row![
                    iced::widget::Space::new().width(Length::Fill),
                    compact_button(
                        favorite_label,
                        if poem.is_favorite {
                            ButtonKind::Primary
                        } else {
                            ButtonKind::Ghost
                        }
                    )
                    .on_press(Message::ToggleFavorite),
                    compact_button("✎ 编辑", ButtonKind::Secondary)
                        .on_press(Message::OpenEditModal),
                ]
                .spacing(12),
                container(text(poem.title.clone()).size(36)).style(theme::title_text),
                container(text(poem.metadata()).size(18)).style(theme::subdued_text),
                container(text(poem.content.clone()).size(24)).style(theme::title_text),
                row![
                    action_button("AI 赏析", ButtonKind::Primary)
                        .on_press(Message::RequestAppreciation),
                ]
                .spacing(12),
                appreciation_card,
            ]
            .spacing(20),
            SurfaceKind::Raised,
        )
    }

    fn reload_poems(&mut self) {
        self.state.poems = self.db.list_poems().unwrap_or_default();
        self.state.sync_selection();
    }

    fn refresh_cached_appreciation(&mut self) {
        let Some(poem) = self.state.selected_poem() else {
            self.state.appreciation.clear();
            return;
        };

        match self.db.load_cached_analysis(&poem.id) {
            Ok(Some(appreciation)) => {
                self.state.appreciation.poem_id = Some(poem.id);
                self.state.appreciation.content = appreciation.display_text();
                self.state.appreciation.loading = false;
                self.state.appreciation.error.clear();
            }
            Ok(None) => self.state.appreciation.clear(),
            Err(err) => {
                self.state.appreciation.poem_id = Some(poem.id);
                self.state.appreciation.content.clear();
                self.state.appreciation.loading = false;
                self.state.appreciation.error = format!("读取赏析缓存失败: {err}");
            }
        }
    }
}

fn dismiss_toast_later(revision: u64) -> Task<Message> {
    Task::perform(
        async move {
            std::thread::sleep(Duration::from_secs(2));
            revision
        },
        Message::ToastExpired,
    )
}
