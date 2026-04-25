use std::time::Duration;

use anyhow::Result;
use iced::widget::{Space, button, column, container, mouse_area, row, scrollable, text};
use iced::{
    Alignment, Color, Element, Length, Size, Subscription, Task, Theme, alignment, mouse, system,
    window,
};
use tracing_subscriber::EnvFilter;

use crate::config::app::AppPaths;
use crate::storage::{AppDatabase, StoredAiConfig};

use super::components::{
    SurfaceKind, ToastTone, modal_overlay, nav_surface, page_shell, surface, toast, toast_host,
};
use super::message::{ContentMode, DetailTool, Message, Modal, ThemeChoice};
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
        .subscription(PoemApp::subscription)
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

        (app, system::theme().map(Message::SystemThemeChanged))
    }

    fn subscription(&self) -> Subscription<Message> {
        system::theme_changes().map(Message::SystemThemeChanged)
    }

    fn theme(&self) -> Theme {
        theme::app_theme(self.state.resolved_theme())
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
                self.state.close_theme_panel();
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
                self.state.close_theme_panel();
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
                self.state.close_theme_panel();
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
            Message::EditContentChanged(action) => {
                if let Some(form) = &mut self.state.edit_form {
                    form.apply_content_action(action);
                }
                Task::none()
            }
            Message::HoverDetailTool(tool) => {
                self.state.hovered_detail_tool = tool;
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
            Message::ToggleThemePanel => {
                self.state.toggle_theme_panel();
                Task::none()
            }
            Message::CloseThemePanel => {
                self.state.close_theme_panel();
                Task::none()
            }
            Message::SwitchTheme(choice) => {
                self.state.active_theme = choice;
                self.state.close_theme_panel();
                let _ = self.db.save_theme_preference(choice.as_str());
                Task::none()
            }
            Message::SystemThemeChanged(mode) => {
                self.state.system_theme_mode = mode;
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
        let content_region = container(
            row![
                self.sidebar_view(),
                content_vertical_divider::<Message>(),
                container(self.middle_pane())
                    .width(Length::FillPortion(5))
                    .height(Length::Fill),
                container(self.detail_pane())
                    .width(Length::FillPortion(3))
                    .height(Length::Fill),
            ]
            .spacing(0)
            .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::content_shell);

        let base: Element<'_, Message> = page_shell(content_region).into();
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

        let with_theme_backdrop: Element<'_, Message> = if self.state.theme_panel_open {
            mouse_area(with_modal)
                .on_press(Message::CloseThemePanel)
                .into()
        } else {
            with_modal
        };

        if self.state.toast.visible {
            toast_host(
                with_theme_backdrop,
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
            with_theme_backdrop
        }
    }

    fn sidebar_view(&self) -> Element<'_, Message> {
        let header = column![
            row![
                sidebar_icon::<Message>(ICON_BRAND, 30.0, SidebarIconTone::Accent),
                column![
                    container(text("诗词").size(22)).style(theme::title_text),
                    container(text("发现诗意之美").size(13)).style(theme::subdued_text),
                ]
                .spacing(6),
            ]
            .spacing(14)
            .align_y(Alignment::Center),
            sidebar_divider::<Message>(),
        ]
        .spacing(18);

        let main_nav = column![
            sidebar_primary_button(ICON_PLUS, "发现新诗词")
                .on_press(Message::OpenModal(Modal::Discovery)),
            sidebar_nav_button(
                ICON_HOME,
                "首页",
                self.state.content_mode == ContentMode::Library
            )
            .on_press(Message::SwitchContentMode(ContentMode::Library)),
            sidebar_nav_button(
                ICON_FAVORITE,
                "收藏夹",
                self.state.content_mode == ContentMode::Favorites
            )
            .on_press(Message::SwitchContentMode(ContentMode::Favorites)),
            sidebar_nav_button(ICON_ABOUT, "关于", self.state.active_modal == Modal::About)
                .on_press(Message::OpenModal(Modal::About)),
        ]
        .spacing(12);

        let theme_picker: Element<'_, Message> = if self.state.theme_panel_open {
            column![
                theme_options_panel(self.state.active_theme),
                theme_trigger_button(
                    self.state.active_theme.display_name(),
                    self.state.theme_panel_open,
                ),
            ]
            .spacing(10)
            .width(Length::Fill)
            .into()
        } else {
            theme_trigger_button(
                self.state.active_theme.display_name(),
                self.state.theme_panel_open,
            )
            .into()
        };

        let footer = column![
            container(theme_picker).width(Length::Fill),
            sidebar_divider::<Message>(),
            sidebar_nav_button(
                ICON_SETTINGS,
                "设置",
                self.state.active_modal == Modal::Settings
            )
            .on_press(Message::OpenModal(Modal::Settings)),
        ]
        .spacing(16);

        nav_surface(
            column![
                header,
                Space::new().height(Length::Fixed(16.0)),
                main_nav,
                Space::new().height(Length::Fill),
                footer,
            ]
            .spacing(0),
        )
        .width(296)
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
            return row![
                content_vertical_divider::<Message>(),
                container(
                    column![
                        Space::new().height(Length::FillPortion(1)),
                        column![
                            text("暂无诗词").size(28),
                            text("当前模式下没有可展示的内容。").size(16),
                        ]
                        .spacing(12),
                        Space::new().height(Length::FillPortion(1)),
                    ]
                    .spacing(20)
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .padding([28, 34])
                .style(theme::detail_stage),
            ]
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
        };

        let favorite_hovered = self.state.hovered_detail_tool == Some(DetailTool::Favorite);
        let is_light_theme = matches!(self.state.resolved_theme(), ThemeChoice::Songyanjian);
        let favorite_icon = if poem.is_favorite {
            ICON_FAVORITE_FILLED
        } else {
            ICON_FAVORITE
        };
        let favorite_tone = if poem.is_favorite && favorite_hovered && is_light_theme {
            SidebarIconTone::Inverse
        } else if poem.is_favorite {
            SidebarIconTone::Accent
        } else {
            SidebarIconTone::Default
        };

        let action_row = row![
            Space::new().width(Length::Fill),
            detail_icon_action(
                favorite_icon,
                favorite_tone,
                poem.is_favorite,
                DetailTool::Favorite,
                Message::ToggleFavorite
            ),
            detail_icon_action(
                ICON_EDIT,
                SidebarIconTone::Default,
                false,
                DetailTool::Edit,
                Message::OpenEditModal,
            ),
            detail_icon_action(
                ICON_APPRECIATION,
                SidebarIconTone::Default,
                false,
                DetailTool::Appreciation,
                Message::RequestAppreciation,
            ),
        ]
        .spacing(12)
        .align_y(Alignment::Center);

        let mut reading_column = column![
            container(
                text(poem.title.clone())
                    .size(28)
                    .width(Length::Fill)
                    .align_x(alignment::Horizontal::Center),
            )
            .style(theme::title_text),
            container(
                text(poem.metadata())
                    .size(17)
                    .width(Length::Fill)
                    .align_x(alignment::Horizontal::Center),
            )
            .style(theme::subdued_text),
            container(
                text(poem.content.clone())
                    .size(19)
                    .width(Length::Fill)
                    .align_x(alignment::Horizontal::Center),
            )
            .style(theme::title_text),
        ]
        .align_x(Alignment::Center)
        .spacing(22);

        if self.state.appreciation.loading {
            reading_column = reading_column.push(surface(
                text("正在生成赏析…").size(15),
                SurfaceKind::Appreciation,
            ));
        } else if !self.state.appreciation.error.is_empty() {
            reading_column = reading_column.push(surface(
                text(self.state.appreciation.error.clone()).size(15),
                SurfaceKind::Appreciation,
            ));
        } else if !self.state.appreciation.content.is_empty()
            && self.state.appreciation.poem_id.as_deref() == Some(poem.id.as_str())
        {
            reading_column = reading_column.push(surface(
                text(self.state.appreciation.content.clone()).size(16),
                SurfaceKind::Appreciation,
            ));
        }

        let content = column![
            action_row,
            Space::new().height(Length::Fixed(28.0)),
            container(reading_column)
                .width(Length::Fill)
                .max_width(620)
                .center_x(Length::Fill),
        ]
        .spacing(0);

        row![
            content_vertical_divider::<Message>(),
            container(
                scrollable(content)
                    .direction(theme::scrollable_direction())
                    .style(theme::scrollable_style)
                    .height(Length::Fill),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .padding([28, 34])
            .style(theme::detail_stage),
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
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

const ICON_BRAND: &str = "assets/icons/brand-book.svg";
const ICON_PLUS: &str = "assets/icons/plus.svg";
const ICON_HOME: &str = "assets/icons/home.svg";
const ICON_FAVORITE: &str = "assets/icons/favorite-outline.svg";
const ICON_FAVORITE_FILLED: &str = "assets/icons/favorite-filled.svg";
const ICON_ABOUT: &str = "assets/icons/about.svg";
const ICON_SETTINGS: &str = "assets/icons/settings.svg";
const ICON_THEME: &str = "assets/icons/theme.svg";
const ICON_EDIT: &str = "assets/icons/edit.svg";
const ICON_APPRECIATION: &str = "assets/icons/appreciation.svg";

#[derive(Debug, Clone, Copy)]
enum SidebarIconTone {
    Accent,
    Default,
    Inverse,
}

fn sidebar_divider<'a, Message: 'a>() -> iced::widget::Container<'a, Message> {
    container(Space::new().height(1))
        .width(Length::Fill)
        .height(1)
        .style(theme::sidebar_divider)
}

fn content_vertical_divider<'a, Message: 'a>() -> iced::widget::Container<'a, Message> {
    container(Space::new().width(1))
        .width(1)
        .height(Length::Fill)
        .style(theme::content_divider)
}

fn sidebar_primary_button<'a>(
    icon_path: &'static str,
    label: &'a str,
) -> button::Button<'a, Message> {
    button(
        container(
            row![
                sidebar_icon::<Message>(icon_path, 18.0, SidebarIconTone::Inverse),
                text(label).size(16),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .center_x(Length::Fill),
    )
    .width(Length::Fill)
    .padding([18, 22])
    .style(theme::button_sidebar_primary)
}

fn sidebar_nav_button<'a>(
    icon_path: &'static str,
    label: &'a str,
    active: bool,
) -> button::Button<'a, Message> {
    let icon_tone = if active {
        SidebarIconTone::Accent
    } else {
        SidebarIconTone::Default
    };

    button(
        container(
            row![
                sidebar_icon::<Message>(icon_path, 22.0, icon_tone),
                text(label).size(17),
            ]
            .spacing(14)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill),
    )
    .width(Length::Fill)
    .padding([14, 16])
    .style(if active {
        theme::button_sidebar_nav_active
    } else {
        theme::button_sidebar_nav
    })
}

fn theme_trigger_button<'a>(current_label: &'a str, open: bool) -> button::Button<'a, Message> {
    let icon_tone = if open {
        SidebarIconTone::Accent
    } else {
        SidebarIconTone::Default
    };

    button(
        container(
            row![
                sidebar_icon::<Message>(ICON_THEME, 18.0, icon_tone),
                text("主题").size(15),
                Space::new().width(Length::Fill),
                text(current_label).size(12),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill),
    )
    .width(Length::Fill)
    .padding([12, 14])
    .style(theme::button_sidebar_nav)
    .on_press(Message::ToggleThemePanel)
}

fn theme_options_panel<'a>(selected: ThemeChoice) -> Element<'a, Message> {
    let options = [
        ThemeChoice::Songyanjian,
        ThemeChoice::Hanjiangxue,
        ThemeChoice::FollowSystem,
    ];

    container(
        container(options.into_iter().fold(
            column!().spacing(0).width(Length::Fill),
            |column, choice| column.push(theme_option_button(choice, selected)),
        ))
        .width(Length::Fixed(136.0))
        .padding(4)
        .style(theme::theme_menu_panel),
    )
    .width(Length::Fill)
    .align_x(alignment::Horizontal::Right)
    .into()
}

fn theme_option_button<'a>(
    choice: ThemeChoice,
    selected: ThemeChoice,
) -> button::Button<'a, Message> {
    button(
        container(text(choice.display_name()).size(14))
            .width(Length::Fill)
            .padding([0, 2]),
    )
    .width(Length::Fill)
    .padding([9, 12])
    .style(if choice == selected {
        theme::button_sidebar_theme_active
    } else {
        theme::button_sidebar_theme
    })
    .on_press(Message::SwitchTheme(choice))
}

fn detail_icon_action<'a>(
    icon_path: &'static str,
    icon_tone: SidebarIconTone,
    active: bool,
    tool: DetailTool,
    on_press: Message,
) -> Element<'a, Message> {
    mouse_area(
        button(
            container(sidebar_icon::<Message>(icon_path, 28.0, icon_tone))
                .width(Length::Fixed(40.0))
                .height(Length::Fixed(40.0))
                .center_x(Length::Shrink)
                .center_y(Length::Shrink),
        )
        .width(Length::Fixed(44.0))
        .height(Length::Fixed(44.0))
        .padding(0)
        .style(if active {
            theme::button_detail_icon_active
        } else {
            theme::button_detail_icon
        })
        .on_press(on_press),
    )
    .on_enter(Message::HoverDetailTool(Some(tool)))
    .on_exit(Message::HoverDetailTool(None))
    .interaction(mouse::Interaction::Pointer)
    .into()
}

fn sidebar_icon<'a, Message: 'a>(
    icon_path: &'static str,
    size: f32,
    tone: SidebarIconTone,
) -> Element<'a, Message> {
    iced::widget::svg(icon_path)
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .style(move |theme: &Theme, _status| iced::widget::svg::Style {
            color: Some(sidebar_icon_color(theme, tone)),
        })
        .into()
}

fn sidebar_icon_color(theme: &Theme, tone: SidebarIconTone) -> Color {
    let tokens = theme::tokens(theme);

    match tone {
        SidebarIconTone::Accent => tokens.primary,
        SidebarIconTone::Default => {
            if tokens.title.r > 0.7 {
                tokens.text
            } else {
                tokens.title
            }
        }
        SidebarIconTone::Inverse => Color::from_rgb8(0xFF, 0xF8, 0xF2),
    }
}
