use crate::config::ai::{
    AiSettings, FILE_FALLBACK_WARNING, FileSecretStore, KeyringSecretStore, SecretPersistencePlan,
};
use crate::config::app::AppPaths;
use crate::domain::{DiscoveredPoem, Poem};
use crate::storage::StoredAiConfig;

use super::message::{ContentMode, Modal, ThemeChoice};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ToastState {
    pub message: String,
    pub visible: bool,
    pub revision: u64,
}

impl ToastState {
    pub fn show(&mut self, message: impl Into<String>) -> u64 {
        self.message = message.into();
        self.visible = true;
        self.revision = self.revision.wrapping_add(1);
        self.revision
    }

    pub fn dismiss(&mut self) -> bool {
        if !self.visible {
            return false;
        }

        self.visible = false;
        self.revision = self.revision.wrapping_add(1);
        true
    }

    pub fn dismiss_if_current(&mut self, expected_revision: u64) -> bool {
        if self.visible && self.revision == expected_revision {
            return self.dismiss();
        }

        false
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SettingsForm {
    pub base_url: String,
    pub model: String,
    pub api_key: String,
    pub allow_file_fallback: bool,
    pub mode_label: String,
    pub warning: String,
}

impl SettingsForm {
    pub fn from_stored(paths: &AppPaths, config: &StoredAiConfig) -> Self {
        let file_store = FileSecretStore::new(paths.config_dir());
        let (secret, persistence) = current_secret(paths, config.allow_file_fallback);
        let warning = if persistence == SecretPersistencePlan::WarnedFileFallback {
            FILE_FALLBACK_WARNING.to_string()
        } else {
            String::new()
        };
        let api_key = if secret.is_some() && KeyringSecretStore::is_available() {
            String::new()
        } else {
            file_store.load_api_key().ok().flatten().unwrap_or_default()
        };

        Self {
            base_url: config.settings.base_url.clone(),
            model: config.settings.model.clone(),
            api_key,
            allow_file_fallback: config.allow_file_fallback,
            mode_label: config
                .settings
                .mode_for(secret.is_some(), persistence)
                .label()
                .to_string(),
            warning,
        }
    }

    pub fn into_settings(&self) -> AiSettings {
        AiSettings {
            base_url: if self.base_url.trim().is_empty() {
                AiSettings::default().base_url
            } else {
                self.base_url.trim().to_string()
            },
            model: if self.model.trim().is_empty() {
                AiSettings::default().model
            } else {
                self.model.trim().to_string()
            },
            timeout_secs: AiSettings::default().timeout_secs,
        }
    }

    pub fn rehydrated(mut persisted: Self, warning: impl Into<String>) -> Self {
        persisted.warning = warning.into();
        persisted
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EditForm {
    pub poem_id: String,
    pub title: String,
    pub author: String,
    pub dynasty: String,
    pub content: String,
}

impl EditForm {
    pub fn from_poem(poem: &Poem) -> Self {
        Self {
            poem_id: poem.id.clone(),
            title: poem.title.clone(),
            author: poem.author.clone(),
            dynasty: poem.dynasty.clone(),
            content: poem.content.clone(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AppreciationState {
    pub poem_id: Option<String>,
    pub content: String,
    pub loading: bool,
    pub error: String,
}

impl AppreciationState {
    pub fn clear(&mut self) {
        self.poem_id = None;
        self.content.clear();
        self.loading = false;
        self.error.clear();
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DiscoveryPoemExcerpt {
    pub text: String,
    pub centered: bool,
}

#[derive(Clone, Debug)]
pub struct DiscoveryListItem {
    pub title: String,
    pub author: String,
    pub dynasty: String,
    pub reason: String,
    pub relevance: String,
    pub excerpt: DiscoveryPoemExcerpt,
}

#[derive(Clone, Debug)]
pub struct AppState {
    pub poems: Vec<Poem>,
    pub selected_poem_id: Option<String>,
    pub search_query: String,
    pub discovery_query: String,
    pub discovery_results: Vec<DiscoveredPoem>,
    pub discovery_loading: bool,
    pub discovery_status: String,
    pub settings_form: SettingsForm,
    pub toast: ToastState,
    pub active_modal: Modal,
    pub content_mode: ContentMode,
    pub active_theme: ThemeChoice,
    pub appreciation: AppreciationState,
    pub edit_form: Option<EditForm>,
}

impl AppState {
    pub fn new(
        poems: Vec<Poem>,
        selected_poem_id: Option<String>,
        settings_form: SettingsForm,
        active_theme: ThemeChoice,
    ) -> Self {
        Self {
            poems,
            selected_poem_id,
            search_query: String::new(),
            discovery_query: String::new(),
            discovery_results: Vec::new(),
            discovery_loading: false,
            discovery_status: String::new(),
            settings_form,
            toast: ToastState::default(),
            active_modal: Modal::None,
            content_mode: ContentMode::Library,
            active_theme,
            appreciation: AppreciationState::default(),
            edit_form: None,
        }
    }

    pub fn visible_poems(&self) -> Vec<Poem> {
        let mode_poems = match self.content_mode {
            ContentMode::Library => self.poems.clone(),
            ContentMode::Favorites => self
                .poems
                .iter()
                .filter(|poem| poem.is_favorite)
                .cloned()
                .collect(),
        };

        filter_poems(&mode_poems, &self.search_query)
    }

    pub fn selected_poem(&self) -> Option<Poem> {
        let visible = self.visible_poems();
        self.selected_poem_id
            .as_deref()
            .and_then(|poem_id| visible.iter().find(|poem| poem.id == poem_id))
            .cloned()
            .or_else(|| visible.first().cloned())
    }

    pub fn sync_selection(&mut self) {
        let visible = self.visible_poems();
        if let Some(current) = self.selected_poem_id.as_deref()
            && visible.iter().any(|poem| poem.id == current)
        {
            return;
        }

        self.selected_poem_id = visible.first().map(|poem| poem.id.clone());
    }

    pub fn switch_content_mode(&mut self, mode: ContentMode) {
        self.content_mode = mode;
        self.sync_selection();
    }

    pub fn open_modal(&mut self, modal: Modal) {
        self.active_modal = modal;
    }

    pub fn close_modal(&mut self) {
        self.active_modal = Modal::None;
    }

    pub fn discovery_items(&self) -> Vec<DiscoveryListItem> {
        self.discovery_results
            .iter()
            .map(|item| DiscoveryListItem {
                title: item.title.clone(),
                author: item.author.clone(),
                dynasty: item.dynasty.clone(),
                reason: format!("匹配: {}", item.match_reason),
                relevance: item.relevance_percent(),
                excerpt: discovery_poem_excerpt(&item.content),
            })
            .collect()
    }

    pub fn open_edit_for_selected(&mut self) {
        self.edit_form = self.selected_poem().as_ref().map(EditForm::from_poem);
        if self.edit_form.is_some() {
            self.active_modal = Modal::Edit;
        }
    }

    pub fn close_edit(&mut self) {
        self.edit_form = None;
        self.active_modal = Modal::None;
    }
}

pub fn current_secret(
    paths: &AppPaths,
    allow_file_fallback: bool,
) -> (Option<String>, SecretPersistencePlan) {
    let keyring = KeyringSecretStore;
    let file_store = FileSecretStore::new(paths.config_dir());

    if KeyringSecretStore::is_available() {
        if let Ok(Some(secret)) = keyring.load_api_key() {
            return (Some(secret), SecretPersistencePlan::Keyring);
        }
        if allow_file_fallback && let Ok(Some(secret)) = file_store.load_api_key() {
            return (Some(secret), SecretPersistencePlan::WarnedFileFallback);
        }

        return (None, SecretPersistencePlan::Keyring);
    }

    if allow_file_fallback {
        if let Ok(Some(secret)) = file_store.load_api_key() {
            return (Some(secret), SecretPersistencePlan::WarnedFileFallback);
        }

        return (None, SecretPersistencePlan::WarnedFileFallback);
    }

    (None, SecretPersistencePlan::Unavailable)
}

pub fn filter_poems(poems: &[Poem], query: &str) -> Vec<Poem> {
    let normalized_query = query.trim().to_lowercase();
    if normalized_query.is_empty() {
        return poems.to_vec();
    }

    let tokens = normalized_query
        .split_whitespace()
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();

    poems
        .iter()
        .filter(|poem| {
            let haystack =
                format!("{} {} {}", poem.title, poem.author, poem.content).to_lowercase();
            haystack.contains(&normalized_query)
                || tokens.iter().all(|token| haystack.contains(token))
        })
        .cloned()
        .collect()
}

pub fn discovery_poem_excerpt(content: &str) -> DiscoveryPoemExcerpt {
    const MAX_LINES: usize = 4;
    const CENTER_MAX_CHARS_PER_LINE: usize = 16;
    const LEFT_WRAP_WIDTH: usize = 18;

    let mut lines = content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(MAX_LINES + 1)
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();

    if lines.is_empty() {
        return DiscoveryPoemExcerpt {
            text: "（暂无诗句）".to_string(),
            centered: true,
        };
    }

    let truncated = lines.len() > MAX_LINES;
    lines.truncate(MAX_LINES);

    let centered = lines
        .iter()
        .all(|line| poetry_line_char_count(line) <= CENTER_MAX_CHARS_PER_LINE);

    if truncated && let Some(last) = lines.last_mut() {
        append_poetry_ellipsis(last);
    }

    let text = if centered {
        lines.join("\n")
    } else {
        soft_wrap(&lines.join("\n"), LEFT_WRAP_WIDTH)
    };

    DiscoveryPoemExcerpt { text, centered }
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

fn poetry_line_char_count(line: &str) -> usize {
    line.chars()
        .filter(|ch| !ch.is_whitespace() && !matches!(ch, '，' | '。' | '？' | '！' | '；' | '、'))
        .count()
}

fn append_poetry_ellipsis(line: &mut String) {
    while line.ends_with(|ch: char| {
        ch.is_whitespace() || matches!(ch, '，' | '。' | '？' | '！' | '；' | '、')
    }) {
        line.pop();
    }
    line.push('…');
}

#[cfg(test)]
mod tests {
    use crate::domain::Poem;

    use super::{
        AppState, ContentMode, Modal, SettingsForm, ThemeChoice, ToastState,
        discovery_poem_excerpt, filter_poems, soft_wrap,
    };

    fn poem(id: &str, title: &str, author: &str, content: &str) -> Poem {
        Poem {
            id: id.to_string(),
            title: title.to_string(),
            author: author.to_string(),
            dynasty: "唐".to_string(),
            content: content.to_string(),
            tags: Vec::new(),
            source: "seed".to_string(),
            license: "public".to_string(),
            is_favorite: false,
        }
    }

    #[test]
    fn search_matches_full_query_or_all_tokens() {
        let poems = vec![
            poem("1", "静夜思", "李白", "床前明月光"),
            poem("2", "春晓", "孟浩然", "春眠不觉晓"),
        ];

        let result = filter_poems(&poems, "李白 明月");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "1");
    }

    #[test]
    fn search_returns_all_poems_when_query_is_empty() {
        let poems = vec![
            poem("1", "静夜思", "李白", "床前明月光"),
            poem("2", "春晓", "孟浩然", "春眠不觉晓"),
        ];

        let result = filter_poems(&poems, "   ");
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn settings_form_blank_values_fall_back_to_defaults() {
        let form = SettingsForm {
            base_url: " ".into(),
            model: "".into(),
            api_key: String::new(),
            allow_file_fallback: false,
            mode_label: String::new(),
            warning: String::new(),
        };

        let settings = form.into_settings();
        assert_eq!(settings.base_url, crate::config::ai::DEFAULT_BASE_URL);
        assert_eq!(settings.model, crate::config::ai::DEFAULT_MODEL);
    }

    #[test]
    fn switching_to_favorites_filters_visible_poems() {
        let mut poems = vec![
            poem("1", "静夜思", "李白", "床前明月光"),
            poem("2", "春晓", "孟浩然", "春眠不觉晓"),
        ];
        poems[1].is_favorite = true;

        let mut state = AppState::new(
            poems,
            Some("1".into()),
            SettingsForm::default(),
            ThemeChoice::Songyanjian,
        );
        state.switch_content_mode(ContentMode::Favorites);

        let visible = state.visible_poems();
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].id, "2");
    }

    #[test]
    fn theme_choice_defaults_to_songyanjian() {
        assert_eq!(ThemeChoice::from_saved(None), ThemeChoice::Songyanjian);
    }

    #[test]
    fn theme_choice_restores_hanjiangxue_from_saved_value() {
        assert_eq!(
            ThemeChoice::from_saved(Some("hanjiangxue")),
            ThemeChoice::Hanjiangxue
        );
    }

    #[test]
    fn modal_open_replaces_previous_modal_and_close_clears_it() {
        let poems = vec![poem("1", "静夜思", "李白", "床前明月光")];
        let mut state = AppState::new(
            poems,
            Some("1".into()),
            SettingsForm::default(),
            ThemeChoice::Songyanjian,
        );

        state.open_modal(Modal::Discovery);
        assert_eq!(state.active_modal, Modal::Discovery);

        state.open_modal(Modal::Settings);
        assert_eq!(state.active_modal, Modal::Settings);

        state.close_modal();
        assert_eq!(state.active_modal, Modal::None);
    }

    #[test]
    fn opening_edit_preloads_selected_poem() {
        let poems = vec![poem("1", "静夜思", "李白", "床前明月光")];
        let mut state = AppState::new(
            poems,
            Some("1".into()),
            SettingsForm::default(),
            ThemeChoice::Songyanjian,
        );

        state.open_edit_for_selected();
        let edit = state.edit_form.expect("edit form");
        assert_eq!(edit.title, "静夜思");
        assert_eq!(edit.author, "李白");
        assert_eq!(state.active_modal, Modal::Edit);
    }

    #[test]
    fn discovery_excerpt_preserves_short_poem_lines_centered() {
        let excerpt = discovery_poem_excerpt("春眠不觉晓\n处处闻啼鸟\n夜来风雨声\n花落知多少");

        assert!(excerpt.centered);
        assert_eq!(
            excerpt.text,
            "春眠不觉晓\n处处闻啼鸟\n夜来风雨声\n花落知多少"
        );
    }

    #[test]
    fn discovery_excerpt_truncates_long_poems_cleanly() {
        let excerpt = discovery_poem_excerpt("一行\n二行\n三行\n四行。\n五行");

        assert!(excerpt.centered);
        assert_eq!(excerpt.text, "一行\n二行\n三行\n四行…");
    }

    #[test]
    fn discovery_excerpt_left_aligns_long_lines() {
        let excerpt = discovery_poem_excerpt(
            "这是一句明显超过短诗展示宽度的长句需要保留可读性\n第二句也很长很长不适合强制居中",
        );

        assert!(!excerpt.centered);
        assert_eq!(
            excerpt.text,
            soft_wrap(
                "这是一句明显超过短诗展示宽度的长句需要保留可读性\n第二句也很长很长不适合强制居中",
                18,
            )
        );
    }

    #[test]
    fn toast_dismiss_ignores_stale_revision() {
        let mut toast = ToastState::default();

        let first = toast.show("第一次");
        let second = toast.show("第二次");

        assert!(toast.visible);
        assert_eq!(toast.message, "第二次");
        assert!(!toast.dismiss_if_current(first));
        assert!(toast.visible);
        assert!(toast.dismiss_if_current(second));
        assert!(!toast.visible);
    }

    #[test]
    fn manual_toast_dismiss_does_not_corrupt_later_toast_revision() {
        let mut toast = ToastState::default();

        let first = toast.show("第一次");
        assert!(toast.dismiss());

        let second = toast.show("第二次");
        assert!(toast.visible);
        assert!(!toast.dismiss_if_current(first));
        assert!(toast.dismiss_if_current(second));
    }
}
