use std::time::Duration;

use anyhow::Result;
use iced::widget::{column, mouse_area, row, text};
use iced::{Element, Length, Size, Task, Theme, window};
use tracing_subscriber::EnvFilter;

use crate::config::app::AppPaths;
use crate::storage::{AppDatabase, StoredAiConfig};

use super::components::{
    ButtonKind, ToastTone, action_button, modal_overlay, page_shell, shell_surface, toast,
    toast_host,
};
use super::message::{Message, Modal};
use super::screens::{about_modal, discovery_modal, library, settings_modal};
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
            size: Size::new(1320.0, 860.0),
            min_size: Some(Size::new(1120.0, 720.0)),
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
        let state = AppState::new(poems, selected_poem_id, settings_form);

        (
            Self {
                paths,
                db,
                ai_config,
                state,
            },
            Task::none(),
        )
    }

    fn theme(&self) -> Theme {
        theme::app_theme()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SelectPoem(poem_id) => {
                self.state.selected_poem_id = Some(poem_id);
                Task::none()
            }
            Message::SearchChanged(query) => {
                self.state.search_query = query;
                self.state.sync_selection();
                Task::none()
            }
            Message::OpenModal(modal) => {
                if matches!(modal, Modal::Settings) {
                    self.state.settings_form = SettingsForm::rehydrated(
                        SettingsForm::from_stored(&self.paths, &self.ai_config),
                        String::new(),
                    );
                }
                self.state.open_modal(modal);
                Task::none()
            }
            Message::CloseModal => {
                self.state.close_modal();
                Task::none()
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
                        if self.state.discovery_results.is_empty() {
                            self.state.discovery_status =
                                "AI 没有返回可用诗词，请换个关键词再试。".to_string();
                        } else {
                            self.state.discovery_status.clear();
                        }
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
                    self.state.poems = self.db.list_poems().unwrap_or_default();
                    self.state.selected_poem_id = Some(imported.poem_id);
                    self.state.sync_selection();
                    self.state.active_modal = Modal::None;
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
        let visible_poems = self.state.visible_poems();
        let selected = self.state.selected_poem();
        let discovery_items = self.state.discovery_items();

        let header = row![
            column![
                text("诗词桌面").size(16),
                text("Pure Rust Iced Edition").size(34),
                text("更像成熟桌面应用的纯 Rust 诗词阅读器").size(15),
            ]
            .spacing(6)
            .width(Length::Fill),
            action_button("发现新诗词", ButtonKind::Primary)
                .on_press(Message::OpenModal(Modal::Discovery)),
            action_button("关于", ButtonKind::Secondary).on_press(Message::OpenModal(Modal::About)),
            action_button("AI 设置", ButtonKind::Secondary)
                .on_press(Message::OpenModal(Modal::Settings)),
        ]
        .spacing(12)
        .align_y(iced::alignment::Vertical::Center);

        let body = library::view(visible_poems, selected, &self.state.search_query);

        let base: Element<'_, Message> =
            page_shell(column![shell_surface(header), body,].spacing(f32::from(theme::SPACE_5)))
                .into();

        let with_modal = match self.state.active_modal {
            Modal::None => base,
            Modal::Discovery => modal_overlay(
                base,
                discovery_modal::view(
                    self.state.discovery_query.clone(),
                    self.state.discovery_loading,
                    self.state.discovery_status.clone(),
                    discovery_items,
                ),
                Some(Message::CloseModal),
            ),
            Modal::Settings => modal_overlay(
                base,
                settings_modal::view(&self.state.settings_form),
                Some(Message::CloseModal),
            ),
            Modal::About => modal_overlay(base, about_modal::view(), Some(Message::CloseModal)),
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
