mod stderr_filter;

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use slint::winit_030::{EventResult, WinitWindowAccessor, winit};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use tracing_subscriber::EnvFilter;

use crate::config::ai::{
    AiMode, AiSettings, FILE_FALLBACK_WARNING, FileSecretStore, KeyringSecretStore,
    SecretPersistencePlan,
};
use crate::config::app::AppPaths;
use crate::domain::{AiAppreciation, DiscoveredPoem, Poem};
use crate::services::ai::{
    HttpAiTransport, OpenAiCompatibleClient, build_appreciation_prompt, build_discovery_prompt,
};
use crate::services::normalization::validate_appreciation;
use crate::storage::{AppDatabase, StoredAiConfig, WindowGeometry};
use crate::{AppWindow, DiscoveryResultRow, PoemRow};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FilterKind {
    Recommended,
    Favorites,
    Tang,
    Song,
    Random,
}

impl FilterKind {
    fn from_label(value: &str) -> Self {
        match value {
            "favorites" => Self::Favorites,
            "tang" => Self::Tang,
            "song" => Self::Song,
            "random" => Self::Random,
            _ => Self::Recommended,
        }
    }

    fn title(self) -> &'static str {
        match self {
            Self::Recommended => "推荐诗库",
            Self::Favorites => "我的收藏",
            Self::Tang => "唐诗精选",
            Self::Song => "宋词清单",
            Self::Random => "随机精选",
        }
    }
}

#[derive(Clone, Debug)]
struct PendingDiscoveryResult {
    id: String,
    poem: DiscoveredPoem,
}

pub fn run() -> Result<()> {
    stderr_filter::install_known_warning_filter();
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"))
        .add_directive("icu_provider=error".parse().unwrap())
        .add_directive("icu_segmenter=error".parse().unwrap());
    let _ = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .try_init();

    let paths = AppPaths::resolve()?;
    let db = AppDatabase::new(paths.db_path());
    db.bootstrap()?;
    let ai_config = db.load_ai_config()?;
    let saved_window_geometry = db.load_window_geometry()?;

    let controller = Arc::new(Mutex::new(AppController::new(db, paths, ai_config)?));
    let app = AppWindow::new()?;

    bind_callbacks(&app, controller.clone());
    install_window_geometry_persistence(
        &app,
        controller.lock().unwrap().db.clone(),
        saved_window_geometry,
    );
    controller.lock().unwrap().render(&app)?;
    app.show()?;
    let weak = app.as_weak();
    slint::Timer::single_shot(Duration::ZERO, move || {
        if let Some(app) = weak.upgrade() {
            restore_or_center_window(&app, saved_window_geometry);
        }
    });
    slint::run_event_loop()?;
    app.hide()?;
    Ok(())
}

fn install_window_geometry_persistence(
    app: &AppWindow,
    db: AppDatabase,
    saved_geometry: Option<WindowGeometry>,
) {
    let latest_geometry = Arc::new(Mutex::new(saved_geometry));
    let latest_for_events = latest_geometry.clone();
    app.window().on_winit_window_event(move |window, event| {
        match event {
            winit::event::WindowEvent::Moved(position) => {
                let mut latest = latest_for_events.lock().unwrap();
                let geometry = latest.get_or_insert_with(WindowGeometry::default);
                geometry.x = position.x;
                geometry.y = position.y;
            }
            winit::event::WindowEvent::Resized(size) => {
                let mut latest = latest_for_events.lock().unwrap();
                let geometry = latest.get_or_insert_with(WindowGeometry::default);
                geometry.width = size.width;
                geometry.height = size.height;
            }
            winit::event::WindowEvent::CloseRequested => {
                if let Some(geometry) = latest_for_events.lock().unwrap().as_ref().copied() {
                    let _ = db.save_window_geometry(geometry);
                }
            }
            _ => {}
        }
        let latest_for_sync = latest_for_events.clone();
        let _ = window.with_winit_window(|winit_window: &winit::window::Window| {
            if let Some(geometry) = current_window_geometry(winit_window) {
                *latest_for_sync.lock().unwrap() = Some(geometry);
            }
        });
        EventResult::Propagate
    });
}

fn restore_or_center_window(app: &AppWindow, saved_geometry: Option<WindowGeometry>) {
    let _ = app
        .window()
        .with_winit_window(|window: &winit::window::Window| {
            if let Some(geometry) = saved_geometry.filter(|g| window_geometry_is_visible(window, g))
            {
                let _ = window.request_inner_size(winit::dpi::PhysicalSize::new(
                    geometry.width,
                    geometry.height,
                ));
                window
                    .set_outer_position(winit::dpi::PhysicalPosition::new(geometry.x, geometry.y));
            } else {
                center_winit_window_on_monitor(window);
            }
        });
}

fn center_winit_window_on_monitor(window: &winit::window::Window) {
    let Some(monitor) = window
        .current_monitor()
        .or_else(|| window.available_monitors().next())
    else {
        return;
    };

    let monitor_size = monitor.size();
    let monitor_origin = monitor.position();
    let window_size = window.outer_size();

    let x = monitor_origin.x + ((monitor_size.width as i32 - window_size.width as i32) / 2).max(0);
    let y =
        monitor_origin.y + ((monitor_size.height as i32 - window_size.height as i32) / 2).max(0);

    window.set_outer_position(winit::dpi::PhysicalPosition::new(x, y));
}

fn window_geometry_is_visible(window: &winit::window::Window, geometry: &WindowGeometry) -> bool {
    if geometry.width == 0 || geometry.height == 0 {
        return false;
    }

    let right = geometry.x.saturating_add(geometry.width as i32);
    let bottom = geometry.y.saturating_add(geometry.height as i32);

    window.available_monitors().any(|monitor| {
        let origin = monitor.position();
        let size = monitor.size();
        let monitor_right = origin.x.saturating_add(size.width as i32);
        let monitor_bottom = origin.y.saturating_add(size.height as i32);

        geometry.x < monitor_right
            && right > origin.x
            && geometry.y < monitor_bottom
            && bottom > origin.y
    })
}

fn current_window_geometry(window: &winit::window::Window) -> Option<WindowGeometry> {
    let position = window.outer_position().ok()?;
    let size = window.inner_size();
    Some(WindowGeometry {
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
    })
}

fn bind_callbacks(app: &AppWindow, controller: Arc<Mutex<AppController>>) {
    let weak = app.as_weak();
    let ctrl = controller.clone();
    app.on_select_filter(move |value| {
        if let Some(window) = weak.upgrade() {
            let _ = ctrl.lock().unwrap().set_filter(value.as_str(), &window);
        }
    });

    let weak = app.as_weak();
    let ctrl = controller.clone();
    app.on_select_poem(move |poem_id| {
        if let Some(window) = weak.upgrade() {
            let _ = ctrl.lock().unwrap().select_poem(poem_id.as_str(), &window);
        }
    });

    let weak = app.as_weak();
    let ctrl = controller.clone();
    app.on_toggle_favorite(move |poem_id| {
        if let Some(window) = weak.upgrade() {
            let _ = ctrl
                .lock()
                .unwrap()
                .toggle_favorite(poem_id.as_str(), &window);
        }
    });

    let weak = app.as_weak();
    let ctrl = controller.clone();
    app.on_run_local_search(move |query| {
        if let Some(window) = weak.upgrade() {
            let _ = ctrl
                .lock()
                .unwrap()
                .set_local_search(query.as_str(), &window);
        }
    });

    let weak = app.as_weak();
    let ctrl = controller.clone();
    app.on_run_discovery_search(move |query| {
        AppController::spawn_discovery_search(ctrl.clone(), weak.clone(), query.to_string());
    });

    let weak = app.as_weak();
    let ctrl = controller.clone();
    app.on_import_discovery(move |item_id| {
        if let Some(window) = weak.upgrade()
            && ctrl
                .lock()
                .unwrap()
                .import_discovery(item_id.as_str(), &window)
                .is_ok()
        {
            schedule_toast_dismiss(weak.clone());
        }
    });

    let weak = app.as_weak();
    app.on_dismiss_toast(move || {
        if let Some(window) = weak.upgrade() {
            window.set_toast_visible(false);
        }
    });

    let weak = app.as_weak();
    let ctrl = controller.clone();
    app.on_request_appreciation(move |poem_id| {
        AppController::spawn_appreciation(ctrl.clone(), weak.clone(), poem_id.to_string());
    });

    let weak = app.as_weak();
    let ctrl = controller.clone();
    app.on_save_ai_settings(move |base_url, model, api_key, allow_file_fallback| {
        if let Some(window) = weak.upgrade() {
            let _ = ctrl.lock().unwrap().save_ai_settings(
                base_url.as_str(),
                model.as_str(),
                api_key.as_str(),
                allow_file_fallback,
                &window,
            );
        }
    });

    let weak = app.as_weak();
    let ctrl = controller;
    app.on_clear_ai_key(move || {
        if let Some(window) = weak.upgrade() {
            let _ = ctrl.lock().unwrap().clear_ai_key(&window);
        }
    });
}

fn schedule_toast_dismiss(weak: slint::Weak<AppWindow>) {
    slint::Timer::single_shot(Duration::from_secs(2), move || {
        if let Some(window) = weak.upgrade() {
            window.set_toast_visible(false);
        }
    });
}

struct AppController {
    db: AppDatabase,
    paths: AppPaths,
    ai_config: StoredAiConfig,
    filter: FilterKind,
    selected_poem_id: Option<String>,
    ai_busy: bool,
    local_search_query: String,
    discovery_query: String,
    discovery_busy: bool,
    discovery_status: String,
    discovery_results: Vec<PendingDiscoveryResult>,
    toast_message: String,
    toast_visible: bool,
    analysis_poem_id: Option<String>,
    analysis_text: String,
}

impl AppController {
    fn new(db: AppDatabase, paths: AppPaths, ai_config: StoredAiConfig) -> Result<Self> {
        let poems = db.list_poems()?;
        let selected_poem_id = poems.first().map(|poem| poem.id.clone());
        Ok(Self {
            db,
            paths,
            ai_config,
            filter: FilterKind::Recommended,
            selected_poem_id,
            ai_busy: false,
            local_search_query: String::new(),
            discovery_query: String::new(),
            discovery_busy: false,
            discovery_status: String::new(),
            discovery_results: Vec::new(),
            toast_message: String::new(),
            toast_visible: false,
            analysis_poem_id: None,
            analysis_text: String::new(),
        })
    }

    fn render(&mut self, app: &AppWindow) -> Result<()> {
        let poems = self.db.list_poems()?;
        let visible = self.visible_poems(&poems);
        let favorites = poems.iter().filter(|poem| poem.is_favorite).count();

        if self.selected_poem_id.is_none() {
            self.selected_poem_id = visible.first().map(|poem| poem.id.clone());
        }
        if let Some(selected_id) = self.selected_poem_id.clone()
            && !visible.iter().any(|poem| poem.id == selected_id)
        {
            self.selected_poem_id = visible.first().map(|poem| poem.id.clone());
        }

        let selected = self
            .selected_poem_id
            .as_deref()
            .and_then(|poem_id| poems.iter().find(|poem| poem.id == poem_id))
            .cloned()
            .or_else(|| visible.first().cloned());

        if let Some(poem) = &selected {
            self.selected_poem_id = Some(poem.id.clone());
            if let Some(cached) = self.db.load_cached_analysis(&poem.id)? {
                self.analysis_poem_id = Some(poem.id.clone());
                self.analysis_text = cached.display_text();
            } else if self.analysis_poem_id.as_deref() != Some(poem.id.as_str()) {
                self.analysis_poem_id = None;
                self.analysis_text.clear();
            }
        }

        let section_title = if self.local_search_query.trim().is_empty() {
            self.filter.title()
        } else {
            "本地搜索结果"
        };
        app.set_section_title(section_title.into());
        app.set_poem_count(format!("共 {} 首 · 收藏 {} 首", poems.len(), favorites).into());
        app.set_ai_busy(self.ai_busy);
        app.set_ai_mode_label(self.current_ai_mode().label().into());
        app.set_local_search_query(self.local_search_query.clone().into());
        app.set_discovery_query(self.discovery_query.clone().into());
        app.set_discovery_busy(self.discovery_busy);
        app.set_discovery_status(self.discovery_status.clone().into());
        app.set_toast_message(self.toast_message.clone().into());
        app.set_toast_visible(self.toast_visible);
        app.set_analysis_text(soft_wrap(&self.analysis_text, 22).into());
        app.set_ai_base_url(self.ai_config.settings.base_url.clone().into());
        app.set_ai_model(self.ai_config.settings.model.clone().into());
        app.set_allow_file_fallback(self.ai_config.allow_file_fallback);
        let (_, persistence) = self.current_secret();
        let secret_warning = if persistence == SecretPersistencePlan::WarnedFileFallback {
            soft_wrap(FILE_FALLBACK_WARNING, 24)
        } else {
            String::new()
        };
        app.set_secret_warning(secret_warning.into());

        app.set_poem_rows(ModelRc::from(std::rc::Rc::new(VecModel::from(
            visible.iter().map(poem_to_row).collect::<Vec<_>>(),
        ))));
        app.set_discovery_rows(ModelRc::from(std::rc::Rc::new(VecModel::from(
            self.discovery_rows(),
        ))));

        if let Some(poem) = selected {
            app.set_selected_poem_id(poem.id.clone().into());
            app.set_selected_title(poem.title.clone().into());
            app.set_selected_meta(poem.metadata().into());
            app.set_selected_tags(poem.tags_summary().into());
            app.set_selected_content(poem.content.clone().into());
            app.set_selected_source(format!("{} · {}", poem.source, poem.license).into());
            app.set_selected_favorite(poem.is_favorite);
        } else {
            app.set_selected_poem_id(SharedString::default());
            app.set_selected_title("暂无诗词".into());
            app.set_selected_meta(SharedString::default());
            app.set_selected_tags(SharedString::default());
            app.set_selected_content("当前筛选条件下没有可展示的诗词。".into());
            app.set_selected_source(SharedString::default());
            app.set_selected_favorite(false);
        }

        Ok(())
    }

    fn set_filter(&mut self, value: &str, app: &AppWindow) -> Result<()> {
        self.filter = FilterKind::from_label(value);
        self.local_search_query.clear();
        self.render(app)
    }

    fn set_local_search(&mut self, query: &str, app: &AppWindow) -> Result<()> {
        self.local_search_query = query.trim().to_string();
        self.render(app)
    }

    fn select_poem(&mut self, poem_id: &str, app: &AppWindow) -> Result<()> {
        self.selected_poem_id = Some(poem_id.to_string());
        self.analysis_poem_id = None;
        self.analysis_text.clear();
        self.render(app)
    }

    fn toggle_favorite(&mut self, poem_id: &str, app: &AppWindow) -> Result<()> {
        self.db.toggle_favorite(poem_id)?;
        self.render(app)
    }

    fn save_ai_settings(
        &mut self,
        base_url: &str,
        model: &str,
        api_key: &str,
        allow_file_fallback: bool,
        app: &AppWindow,
    ) -> Result<()> {
        self.ai_config.settings = AiSettings {
            base_url: if base_url.trim().is_empty() {
                AiSettings::default().base_url
            } else {
                base_url.trim().to_string()
            },
            model: if model.trim().is_empty() {
                AiSettings::default().model
            } else {
                model.trim().to_string()
            },
            timeout_secs: self.ai_config.settings.timeout_secs,
        };
        self.ai_config.allow_file_fallback = allow_file_fallback;
        self.db.save_ai_config(&self.ai_config)?;

        if !api_key.trim().is_empty() {
            self.store_secret(api_key.trim())?;
        }
        self.render(app)
    }

    fn clear_ai_key(&mut self, app: &AppWindow) -> Result<()> {
        let keyring = KeyringSecretStore;
        let file_store = FileSecretStore::new(self.paths.config_dir());
        let _ = keyring.clear();
        let _ = file_store.clear();
        self.render(app)
    }

    fn store_secret(&self, api_key: &str) -> Result<()> {
        let keyring = KeyringSecretStore;
        let file_store = FileSecretStore::new(self.paths.config_dir());
        if KeyringSecretStore::is_available() {
            keyring.save_api_key(api_key)?;
            let _ = file_store.clear();
            return Ok(());
        }

        if self.ai_config.allow_file_fallback {
            file_store.save_api_key(api_key)?;
            return Ok(());
        }

        anyhow::bail!(FILE_FALLBACK_WARNING)
    }

    fn current_secret(&self) -> (Option<String>, SecretPersistencePlan) {
        let keyring = KeyringSecretStore;
        let file_store = FileSecretStore::new(self.paths.config_dir());

        if KeyringSecretStore::is_available() {
            if let Ok(Some(secret)) = keyring.load_api_key() {
                return (Some(secret), SecretPersistencePlan::Keyring);
            }
            if self.ai_config.allow_file_fallback
                && let Ok(Some(secret)) = file_store.load_api_key()
            {
                return (Some(secret), SecretPersistencePlan::WarnedFileFallback);
            }
            return (None, SecretPersistencePlan::Keyring);
        }

        if self.ai_config.allow_file_fallback {
            if let Ok(Some(secret)) = file_store.load_api_key() {
                return (Some(secret), SecretPersistencePlan::WarnedFileFallback);
            }
            return (None, SecretPersistencePlan::WarnedFileFallback);
        }

        (None, SecretPersistencePlan::Unavailable)
    }

    fn current_ai_mode(&self) -> AiMode {
        let (secret, persistence) = self.current_secret();
        self.ai_config
            .settings
            .mode_for(secret.is_some(), persistence)
    }

    fn discovery_rows(&self) -> Vec<DiscoveryResultRow> {
        self.discovery_results
            .iter()
            .map(|item| DiscoveryResultRow {
                id: item.id.clone().into(),
                title: item.poem.title.clone().into(),
                author: item.poem.author.clone().into(),
                dynasty: item.poem.dynasty.clone().into(),
                snippet: soft_wrap(&item.poem.snippet(), 18).into(),
                reason: soft_wrap(&item.poem.match_reason, 20).into(),
                relevance: item.poem.relevance_percent().into(),
            })
            .collect()
    }

    fn visible_poems(&self, poems: &[Poem]) -> Vec<Poem> {
        let normalized_query = self.local_search_query.trim().to_lowercase();
        if !normalized_query.is_empty() {
            let tokens = normalized_query
                .split_whitespace()
                .filter(|token| !token.is_empty())
                .collect::<Vec<_>>();
            return poems
                .iter()
                .filter(|poem| {
                    let haystack =
                        format!("{} {} {}", poem.title, poem.author, poem.content).to_lowercase();
                    haystack.contains(&normalized_query)
                        || tokens.iter().all(|token| haystack.contains(token))
                })
                .cloned()
                .collect();
        }

        let mut filtered = match self.filter {
            FilterKind::Recommended => poems.to_vec(),
            FilterKind::Favorites => poems
                .iter()
                .filter(|poem| poem.is_favorite)
                .cloned()
                .collect(),
            FilterKind::Tang => poems
                .iter()
                .filter(|poem| poem.dynasty == "唐")
                .cloned()
                .collect(),
            FilterKind::Song => poems
                .iter()
                .filter(|poem| poem.dynasty == "宋")
                .cloned()
                .collect(),
            FilterKind::Random => {
                let mut rev = poems.to_vec();
                rev.reverse();
                rev
            }
        };
        filtered.truncate(12);
        filtered
    }

    fn import_discovery(&mut self, item_id: &str, app: &AppWindow) -> Result<()> {
        let Some(index) = self
            .discovery_results
            .iter()
            .position(|item| item.id == item_id)
        else {
            return Ok(());
        };
        let item = self.discovery_results[index].clone();

        let new_poem_id = self.db.insert_imported_poem(&item.poem)?;
        self.discovery_results.remove(index);
        self.filter = FilterKind::Recommended;
        self.local_search_query.clear();
        self.selected_poem_id = Some(new_poem_id);
        self.discovery_status.clear();
        self.toast_message = format!("已添加《{}》到诗库", item.poem.title);
        self.toast_visible = true;
        self.render(app)
    }

    fn spawn_discovery_search(
        controller: Arc<Mutex<Self>>,
        weak: slint::Weak<AppWindow>,
        query: String,
    ) {
        {
            let mut ctrl = controller.lock().unwrap();
            ctrl.discovery_busy = true;
            ctrl.discovery_query = query.clone();
            ctrl.discovery_status.clear();
            if query.trim().is_empty() {
                ctrl.discovery_busy = false;
                ctrl.discovery_results.clear();
                ctrl.discovery_status = "请输入一个关键词、片段或意境描述。".to_string();
            }
            if let Some(window) = weak.upgrade() {
                let _ = ctrl.render(&window);
            }
        }

        if query.trim().is_empty() {
            return;
        }

        let (settings, secret) = {
            let ctrl = controller.lock().unwrap();
            let (secret, _) = ctrl.current_secret();
            (ctrl.ai_config.settings.clone(), secret)
        };

        std::thread::spawn(move || {
            let result: Result<Vec<PendingDiscoveryResult>, String> = if let Some(secret) = secret {
                let client = OpenAiCompatibleClient::new(HttpAiTransport::new(
                    settings.clone(),
                    Some(secret),
                ));
                let prompt = build_discovery_prompt(&query);
                client
                    .discover(&prompt)
                    .map(|payload| {
                        payload
                            .poems
                            .into_iter()
                            .enumerate()
                            .map(|(index, poem)| PendingDiscoveryResult {
                                id: format!("discovery-{index}"),
                                poem,
                            })
                            .collect::<Vec<_>>()
                    })
                    .map_err(|err| format!("AI 搜索失败：{err:?}"))
            } else {
                Err("AI 未配置，请先在 AI 设置中配置可用的模型与 API Key。".to_string())
            };

            let _ = slint::invoke_from_event_loop(move || {
                if let Some(window) = weak.upgrade() {
                    let mut ctrl = controller.lock().unwrap();
                    ctrl.discovery_busy = false;
                    match result {
                        Ok(items) => {
                            ctrl.discovery_results = items;
                            ctrl.discovery_status = if ctrl.discovery_results.is_empty() {
                                "AI 没有返回可用诗词，请换个关键词再试。".to_string()
                            } else {
                                String::new()
                            };
                        }
                        Err(message) => {
                            ctrl.discovery_results.clear();
                            ctrl.discovery_status = message;
                        }
                    }
                    let _ = ctrl.render(&window);
                }
            });
        });
    }

    fn spawn_appreciation(
        controller: Arc<Mutex<Self>>,
        weak: slint::Weak<AppWindow>,
        poem_id: String,
    ) {
        {
            let mut ctrl = controller.lock().unwrap();
            ctrl.ai_busy = true;
            if let Some(window) = weak.upgrade() {
                let _ = ctrl.render(&window);
            }
        }

        let (db, settings, secret, maybe_poem) = {
            let ctrl = controller.lock().unwrap();
            let poem = ctrl.db.get_poem(&poem_id).ok().flatten();
            let (secret, _) = ctrl.current_secret();
            (
                ctrl.db.clone(),
                ctrl.ai_config.settings.clone(),
                secret,
                poem,
            )
        };

        std::thread::spawn(move || {
            let result: Result<AiAppreciation, String> = (|| {
                let poem = maybe_poem.ok_or_else(|| "所选诗词不存在。".to_string())?;
                let known_ids = db
                    .list_poems()
                    .map_err(|err| err.to_string())?
                    .into_iter()
                    .map(|item| item.id)
                    .collect::<HashSet<_>>();

                let secret = secret
                    .ok_or_else(|| "AI 未配置，当前仅显示本地浏览与收藏能力。".to_string())?;
                let client = OpenAiCompatibleClient::new(HttpAiTransport::new(
                    settings.clone(),
                    Some(secret),
                ));
                let prompt = build_appreciation_prompt(
                    &poem.id,
                    &poem.title,
                    &poem.author,
                    &poem.dynasty,
                    &poem.content,
                );
                let appreciation = client
                    .appreciate(&prompt)
                    .map_err(|err| format!("AI 赏析失败：{err:?}"))?;
                validate_appreciation(appreciation, &known_ids)
                    .ok_or_else(|| "AI 返回的 poem_id 无法匹配本地诗词。".to_string())
            })();

            let _ = slint::invoke_from_event_loop(move || {
                if let Some(window) = weak.upgrade() {
                    let mut ctrl = controller.lock().unwrap();
                    ctrl.ai_busy = false;
                    if let Ok(appreciation) = result {
                        ctrl.analysis_poem_id = Some(poem_id.clone());
                        ctrl.analysis_text = appreciation.display_text();
                        let _ =
                            ctrl.db
                                .save_cached_analysis(&poem_id, &appreciation, &settings.model);
                    }
                    let _ = ctrl.render(&window);
                }
            });
        });
    }
}

fn soft_wrap(input: &str, width: usize) -> String {
    if width == 0 {
        return input.to_string();
    }

    let mut out = String::new();
    for (line_idx, segment) in input.split('\n').enumerate() {
        if line_idx > 0 {
            out.push('\n');
        }
        let mut count = 0usize;
        for ch in segment.chars() {
            if count >= width && !ch.is_whitespace() {
                out.push('\n');
                count = 0;
            }
            out.push(ch);
            count += 1;
        }
    }
    out
}

fn poem_to_row(poem: &Poem) -> PoemRow {
    PoemRow {
        id: poem.id.clone().into(),
        title: poem.title.clone().into(),
        meta: poem.metadata().into(),
        snippet: poem.snippet().into(),
        tags: poem.tags_summary().into(),
        favorite_label: if poem.is_favorite {
            "已收藏".into()
        } else {
            "收藏".into()
        },
        favorite: poem.is_favorite,
    }
}
