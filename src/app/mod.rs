mod stderr_filter;

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use tracing_subscriber::EnvFilter;

use crate::config::ai::{
    AiMode, AiSettings, FILE_FALLBACK_WARNING, FileSecretStore, KeyringSecretStore,
    SecretPersistencePlan,
};
use crate::config::app::AppPaths;
use crate::domain::{AiAppreciation, AiRecommendation, Poem, PoemCandidate};
use crate::services::ai::{
    HttpAiTransport, OpenAiCompatibleClient, build_appreciation_prompt, build_recommendation_prompt,
};
use crate::services::local::{curated_recommendations, discover_locally};
use crate::services::normalization::{
    RecommendationResolution, resolve_recommendations, validate_appreciation,
};
use crate::storage::{AppDatabase, StoredAiConfig};
use crate::{AppWindow, PoemRow, RecommendationRow};

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

    let controller = Arc::new(Mutex::new(AppController::new(db, paths, ai_config)?));
    let app = AppWindow::new()?;

    bind_callbacks(&app, controller.clone());
    controller.lock().unwrap().render(&app)?;
    app.run()?;
    Ok(())
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
    app.on_run_discover(move |query| {
        AppController::spawn_discover(ctrl.clone(), weak.clone(), query.to_string());
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

struct AppController {
    db: AppDatabase,
    paths: AppPaths,
    ai_config: StoredAiConfig,
    filter: FilterKind,
    selected_poem_id: Option<String>,
    banner: String,
    ai_busy: bool,
    discover_query: String,
    recommendations: Vec<AiRecommendation>,
    analysis_text: String,
    analysis_status: String,
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
            banner: "欢迎来到诗词收藏桌面端：先浏览本地诗库，再用 AI 发现意境相近的作品。".into(),
            ai_busy: false,
            discover_query: String::new(),
            recommendations: Vec::new(),
            analysis_text: "选择一首诗后，可在这里查看缓存赏析或生成新的 AI 赏析。".into(),
            analysis_status: "离线可浏览/收藏；AI 为增强能力。".into(),
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
            && !poems.iter().any(|poem| poem.id == selected_id)
        {
            self.selected_poem_id = visible.first().map(|poem| poem.id.clone());
        }

        let selected = self
            .selected_poem_id
            .as_deref()
            .and_then(|poem_id| poems.iter().find(|poem| poem.id == poem_id))
            .cloned()
            .or_else(|| visible.first().cloned())
            .or_else(|| poems.first().cloned());

        if let Some(poem) = &selected {
            self.selected_poem_id = Some(poem.id.clone());
            if let Some(cached) = self.db.load_cached_analysis(&poem.id)? {
                self.analysis_text = cached.display_text();
                self.analysis_status = "已加载缓存赏析。".into();
            } else if self.analysis_text.is_empty() {
                self.analysis_text = "点击“生成 AI 赏析”后，这里会展示结构化解析。".into();
            }
        }

        let section_title =
            if self.filter == FilterKind::Recommended && !self.recommendations.is_empty() {
                "发现结果"
            } else {
                self.filter.title()
            };
        app.set_section_title(section_title.into());
        app.set_banner_text(soft_wrap(&self.banner, 28).into());
        app.set_poem_count(format!("共 {} 首 · 收藏 {} 首", poems.len(), favorites).into());
        app.set_ai_busy(self.ai_busy);
        app.set_ai_mode_label(self.current_ai_mode().label().into());
        app.set_discover_query(self.discover_query.clone().into());
        app.set_discover_placeholder(self.discover_placeholder().into());
        app.set_analysis_status(self.analysis_status.clone().into());
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
        app.set_recommendation_rows(ModelRc::from(std::rc::Rc::new(VecModel::from(
            self.recommendation_rows(&poems),
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
        self.render(app)
    }

    fn select_poem(&mut self, poem_id: &str, app: &AppWindow) -> Result<()> {
        self.selected_poem_id = Some(poem_id.to_string());
        self.analysis_text.clear();
        self.analysis_status = "已切换诗词。可查看缓存赏析，或重新生成 AI 赏析。".into();
        self.render(app)
    }

    fn toggle_favorite(&mut self, poem_id: &str, app: &AppWindow) -> Result<()> {
        let favored = self.db.toggle_favorite(poem_id)?;
        self.banner = if favored {
            "这首诗已加入收藏。".into()
        } else {
            "已从收藏中移除。".into()
        };
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
            self.banner = "AI 设置已保存。现在可以尝试智能发现或 AI 赏析。".into();
        } else {
            self.banner = "已保存 AI 基础设置；如需联网能力，请输入 API Key。".into();
        }
        self.render(app)
    }

    fn clear_ai_key(&mut self, app: &AppWindow) -> Result<()> {
        let keyring = KeyringSecretStore;
        let file_store = FileSecretStore::new(self.paths.config_dir());
        let _ = keyring.clear();
        let _ = file_store.clear();
        self.banner = "已清除 AI Key，应用回到本地浏览/收藏模式。".into();
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

    fn discover_placeholder(&self) -> &'static str {
        match self.current_ai_mode() {
            AiMode::Configured | AiMode::FallbackStorage => {
                "描述你想找的意境，例如：秋夜思乡但不太悲伤"
            }
            _ => "AI 未配置时，会自动使用本地诗库进行意境匹配",
        }
    }

    fn recommendation_rows(&self, poems: &[Poem]) -> Vec<RecommendationRow> {
        let lookup = poems
            .iter()
            .map(|poem| (poem.id.as_str(), poem))
            .collect::<std::collections::HashMap<_, _>>();
        let items = if self.recommendations.is_empty() {
            curated_recommendations(poems, 3)
        } else {
            self.recommendations.clone()
        };

        items
            .into_iter()
            .filter_map(|item| {
                let poem = lookup.get(item.poem_id.as_str())?;
                Some(RecommendationRow {
                    id: poem.id.clone().into(),
                    title: poem.title.clone().into(),
                    meta: poem.metadata().into(),
                    reason: soft_wrap(&item.reason, 18).into(),
                })
            })
            .collect()
    }

    fn visible_poems(&self, poems: &[Poem]) -> Vec<Poem> {
        if self.filter == FilterKind::Recommended && !self.recommendations.is_empty() {
            let lookup = poems
                .iter()
                .map(|poem| (poem.id.as_str(), poem.clone()))
                .collect::<std::collections::HashMap<_, _>>();
            let mut recommended = self
                .recommendations
                .iter()
                .filter_map(|item| lookup.get(item.poem_id.as_str()).cloned())
                .collect::<Vec<_>>();
            recommended.truncate(12);
            if !recommended.is_empty() {
                return recommended;
            }
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

    fn spawn_discover(controller: Arc<Mutex<Self>>, weak: slint::Weak<AppWindow>, query: String) {
        {
            let mut ctrl = controller.lock().unwrap();
            ctrl.ai_busy = true;
            ctrl.banner = "正在生成推荐…".into();
            ctrl.discover_query = query.clone();
            if let Some(window) = weak.upgrade() {
                let _ = ctrl.render(&window);
            }
        }

        let (db, settings, poems, secret) = {
            let ctrl = controller.lock().unwrap();
            let poems = ctrl.db.list_poems().unwrap_or_default();
            let (secret, _) = ctrl.current_secret();
            (
                ctrl.db.clone(),
                ctrl.ai_config.settings.clone(),
                poems,
                secret,
            )
        };

        std::thread::spawn(move || {
            let known_ids = poems
                .iter()
                .map(|poem| poem.id.clone())
                .collect::<HashSet<_>>();
            let fallback = discover_locally(&query, &poems, 3);
            let resolution = if let Some(secret) = secret.clone() {
                let client = OpenAiCompatibleClient::new(HttpAiTransport::new(
                    settings.clone(),
                    Some(secret),
                ));
                let candidates = poems
                    .iter()
                    .take(10)
                    .map(PoemCandidate::from)
                    .collect::<Vec<_>>();
                let prompt = build_recommendation_prompt(&query, &candidates);
                resolve_recommendations(client.recommend(&prompt), &known_ids, fallback)
            } else {
                RecommendationResolution {
                    source: crate::services::normalization::RecommendationSource::LocalFallback,
                    recommendations: fallback,
                    discarded_ids: Vec::new(),
                    valid_ratio: 0.0,
                    fallback_reason: Some(
                        crate::services::normalization::FallbackReason::Unconfigured,
                    ),
                }
            };

            let _ = slint::invoke_from_event_loop(move || {
                if let Some(window) = weak.upgrade() {
                    let mut ctrl = controller.lock().unwrap();
                    ctrl.ai_busy = false;
                    ctrl.recommendations = resolution.recommendations.clone();
                    ctrl.banner = resolution
                        .warning_banner()
                        .unwrap_or("已根据你的描述生成推荐结果。")
                        .to_string();
                    if let Some(first) = ctrl.recommendations.first() {
                        ctrl.selected_poem_id = Some(first.poem_id.clone());
                    }
                    let _ = db.path();
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
            ctrl.analysis_status = "正在生成 AI 赏析…".into();
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
                    match result {
                        Ok(appreciation) => {
                            ctrl.analysis_text = appreciation.display_text();
                            ctrl.analysis_status = "AI 赏析已生成并缓存。".into();
                            ctrl.banner = "已生成当前诗词的 AI 赏析。".into();
                            let _ = ctrl.db.save_cached_analysis(
                                &poem_id,
                                &appreciation,
                                &settings.model,
                            );
                        }
                        Err(message) => {
                            ctrl.analysis_text =
                                "当前未生成新的赏析，可继续浏览本地诗词或稍后重试。".into();
                            ctrl.analysis_status = message.clone();
                            ctrl.banner = message;
                        }
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
