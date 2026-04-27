use crate::config::ai::{
    AiSettings, FILE_FALLBACK_WARNING, FileSecretStore, KeyringSecretStore, SecretPersistencePlan,
};
use crate::config::app::AppPaths;
use crate::domain::{DiscoveredPoem, Poem};
use crate::storage::StoredAiConfig;
use iced::{theme, widget::text_editor};

use super::message::{ContentMode, DetailTool, Modal, ThemeChoice};

const MASKED_API_KEY_SENTINEL: &str = "__saved_api_key__";

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
    pub api_key_masked: bool,
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
        let has_saved_api_key = secret.is_some();
        let api_key = if has_saved_api_key {
            String::new()
        } else {
            file_store.load_api_key().ok().flatten().unwrap_or_default()
        };

        Self {
            base_url: config.settings.base_url.clone(),
            model: config.settings.model.clone(),
            api_key,
            api_key_masked: has_saved_api_key,
            allow_file_fallback: config.allow_file_fallback,
            mode_label: config
                .settings
                .mode_for(secret.is_some(), persistence)
                .label()
                .to_string(),
            warning,
        }
    }

    pub fn to_settings(&self) -> AiSettings {
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

    pub fn api_key_input_value(&self) -> &str {
        if self.api_key_masked {
            MASKED_API_KEY_SENTINEL
        } else {
            &self.api_key
        }
    }

    pub fn set_api_key_input(&mut self, value: String) {
        if !self.api_key_masked {
            self.api_key = value;
            return;
        }

        if let Some(suffix) = value.strip_prefix(MASKED_API_KEY_SENTINEL) {
            if suffix.is_empty() {
                return;
            }

            self.api_key = suffix.to_string();
            self.api_key_masked = false;
            return;
        }

        self.api_key.clear();
        self.api_key_masked = false;

        if !MASKED_API_KEY_SENTINEL.starts_with(&value) {
            self.api_key = value;
        }
    }

    pub fn api_key_for_save(&self) -> Option<&str> {
        if self.api_key_masked {
            return None;
        }

        let trimmed = self.api_key.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    }
}

#[derive(Clone, Debug)]
pub struct EditForm {
    pub poem_id: String,
    pub title: String,
    pub author: String,
    pub dynasty: String,
    pub content: String,
    pub content_editor: text_editor::Content,
}

impl Default for EditForm {
    fn default() -> Self {
        Self {
            poem_id: String::new(),
            title: String::new(),
            author: String::new(),
            dynasty: String::new(),
            content: String::new(),
            content_editor: text_editor::Content::new(),
        }
    }
}

impl EditForm {
    pub fn from_poem(poem: &Poem) -> Self {
        Self {
            poem_id: poem.id.clone(),
            title: poem.title.clone(),
            author: poem.author.clone(),
            dynasty: poem.dynasty.clone(),
            content: poem.content.clone(),
            content_editor: text_editor::Content::with_text(&poem.content),
        }
    }

    pub fn apply_content_action(&mut self, action: text_editor::Action) {
        self.content_editor.perform(action);
        self.content = self.content_editor.text();
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AppreciationState {
    pub poem_id: Option<String>,
    pub content: String,
    pub loading: bool,
    pub loading_poem_id: Option<String>,
    pub error_poem_id: Option<String>,
    pub error: String,
}

impl AppreciationState {
    pub fn clear(&mut self) {
        self.poem_id = None;
        self.content.clear();
        self.loading = false;
        self.loading_poem_id = None;
        self.error_poem_id = None;
        self.error.clear();
    }

    pub fn begin_loading(&mut self, poem_id: impl Into<String>) {
        self.loading = true;
        self.loading_poem_id = Some(poem_id.into());
        self.poem_id = None;
        self.content.clear();
        self.error_poem_id = None;
        self.error.clear();
    }

    pub fn finish_loading(&mut self, poem_id: &str) {
        if self.loading_poem_id.as_deref() == Some(poem_id) {
            self.loading = false;
            self.loading_poem_id = None;
        }
    }

    pub fn clear_visible_feedback(&mut self) {
        self.poem_id = None;
        self.content.clear();
        self.error_poem_id = None;
        self.error.clear();
    }

    pub fn set_content(&mut self, poem_id: impl Into<String>, content: impl Into<String>) {
        self.poem_id = Some(poem_id.into());
        self.content = content.into();
        self.error_poem_id = None;
        self.error.clear();
    }

    pub fn set_error(&mut self, poem_id: impl Into<String>, error: impl Into<String>) {
        self.poem_id = None;
        self.content.clear();
        self.error_poem_id = Some(poem_id.into());
        self.error = error.into();
    }

    pub fn is_loading_for(&self, poem_id: &str) -> bool {
        self.loading && self.loading_poem_id.as_deref() == Some(poem_id)
    }

    pub fn has_content_for(&self, poem_id: &str) -> bool {
        !self.content.is_empty() && self.poem_id.as_deref() == Some(poem_id)
    }

    pub fn has_error_for(&self, poem_id: &str) -> bool {
        !self.error.is_empty() && self.error_poem_id.as_deref() == Some(poem_id)
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
    pub system_theme_mode: theme::Mode,
    pub theme_panel_open: bool,
    pub appreciation: AppreciationState,
    pub edit_form: Option<EditForm>,
    pub hovered_detail_tool: Option<DetailTool>,
    pub loading_frame: usize,
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
            system_theme_mode: theme::Mode::default(),
            theme_panel_open: false,
            appreciation: AppreciationState::default(),
            edit_form: None,
            hovered_detail_tool: None,
            loading_frame: 0,
        }
    }

    pub fn is_loading_animation_active(&self) -> bool {
        self.discovery_loading || self.appreciation.loading
    }

    pub fn advance_loading_frame(&mut self) {
        self.loading_frame = (self.loading_frame + 1) % 4;
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

    pub fn resolved_theme(&self) -> ThemeChoice {
        self.active_theme.resolve(self.system_theme_mode)
    }

    pub fn toggle_theme_panel(&mut self) {
        self.theme_panel_open = !self.theme_panel_open;
    }

    pub fn close_theme_panel(&mut self) {
        self.theme_panel_open = false;
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
    use iced::theme::Mode;

    use super::{
        AppState, AppreciationState, ContentMode, MASKED_API_KEY_SENTINEL, Modal, SettingsForm,
        ThemeChoice, ToastState, discovery_poem_excerpt, filter_poems, soft_wrap,
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
            api_key_masked: false,
            allow_file_fallback: false,
            mode_label: String::new(),
            warning: String::new(),
        };

        let settings = form.to_settings();
        assert_eq!(settings.base_url, crate::config::ai::DEFAULT_BASE_URL);
        assert_eq!(settings.model, crate::config::ai::DEFAULT_MODEL);
    }

    #[test]
    fn masked_api_key_uses_secure_placeholder_and_skips_save() {
        let form = SettingsForm {
            api_key_masked: true,
            ..SettingsForm::default()
        };

        assert_eq!(form.api_key_input_value(), MASKED_API_KEY_SENTINEL);
        assert_eq!(form.api_key_for_save(), None);
    }

    #[test]
    fn appending_to_masked_api_key_replaces_placeholder_with_new_value() {
        let mut form = SettingsForm {
            api_key_masked: true,
            ..SettingsForm::default()
        };

        form.set_api_key_input(format!("{MASKED_API_KEY_SENTINEL}sk-new-key"));

        assert_eq!(form.api_key, "sk-new-key");
        assert!(!form.api_key_masked);
        assert_eq!(form.api_key_for_save(), Some("sk-new-key"));
    }

    #[test]
    fn replacing_masked_api_key_with_paste_uses_new_value_directly() {
        let mut form = SettingsForm {
            api_key_masked: true,
            ..SettingsForm::default()
        };

        form.set_api_key_input("sk-pasted-key".into());

        assert_eq!(form.api_key, "sk-pasted-key");
        assert!(!form.api_key_masked);
        assert_eq!(form.api_key_for_save(), Some("sk-pasted-key"));
    }

    #[test]
    fn deleting_masked_api_key_exits_placeholder_mode_without_saving() {
        let mut form = SettingsForm {
            api_key_masked: true,
            ..SettingsForm::default()
        };

        let shortened_mask = &MASKED_API_KEY_SENTINEL[..MASKED_API_KEY_SENTINEL.len() - 1];
        form.set_api_key_input(shortened_mask.to_string());

        assert_eq!(form.api_key, "");
        assert!(!form.api_key_masked);
        assert_eq!(form.api_key_for_save(), None);
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
    fn theme_choice_restores_follow_system_from_saved_value() {
        assert_eq!(
            ThemeChoice::from_saved(Some("system")),
            ThemeChoice::FollowSystem
        );
    }

    #[test]
    fn invalid_theme_choice_falls_back_to_songyanjian() {
        assert_eq!(
            ThemeChoice::from_saved(Some("unexpected")),
            ThemeChoice::Songyanjian
        );
    }

    #[test]
    fn resolved_theme_tracks_system_mode_when_following_system() {
        let mut state = AppState::new(
            vec![poem("1", "静夜思", "李白", "床前明月光")],
            Some("1".into()),
            SettingsForm::default(),
            ThemeChoice::FollowSystem,
        );

        state.system_theme_mode = Mode::Light;
        assert_eq!(state.resolved_theme(), ThemeChoice::Songyanjian);

        state.system_theme_mode = Mode::Dark;
        assert_eq!(state.resolved_theme(), ThemeChoice::Hanjiangxue);
    }

    #[test]
    fn theme_panel_toggle_and_close_behave_as_expected() {
        let mut state = AppState::new(
            vec![poem("1", "静夜思", "李白", "床前明月光")],
            Some("1".into()),
            SettingsForm::default(),
            ThemeChoice::Songyanjian,
        );

        assert!(!state.theme_panel_open);
        state.toggle_theme_panel();
        assert!(state.theme_panel_open);
        state.toggle_theme_panel();
        assert!(!state.theme_panel_open);

        state.toggle_theme_panel();
        state.close_theme_panel();
        assert!(!state.theme_panel_open);
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

    #[test]
    fn loading_frame_wraps_after_four_steps() {
        let mut state = AppState::new(
            vec![poem("1", "静夜思", "李白", "床前明月光")],
            Some("1".into()),
            SettingsForm::default(),
            ThemeChoice::Songyanjian,
        );

        for _ in 0..5 {
            state.advance_loading_frame();
        }

        assert_eq!(state.loading_frame, 1);
    }

    #[test]
    fn appreciation_loading_belongs_to_original_poem() {
        let mut state = AppState::new(
            vec![
                poem("1", "静夜思", "李白", "床前明月光"),
                poem("2", "春晓", "孟浩然", "春眠不觉晓"),
            ],
            Some("1".into()),
            SettingsForm::default(),
            ThemeChoice::Songyanjian,
        );

        state.appreciation.begin_loading("1");
        state.selected_poem_id = Some("2".into());
        state.appreciation.clear_visible_feedback();

        assert!(state.appreciation.loading);
        assert!(state.appreciation.is_loading_for("1"));
        assert!(!state.appreciation.is_loading_for("2"));
        assert!(!state.appreciation.has_content_for("2"));
    }

    #[test]
    fn appreciation_finish_loading_only_clears_matching_request() {
        let mut state = AppreciationState::default();

        state.begin_loading("poem-1");
        state.finish_loading("poem-2");
        assert!(state.loading);

        state.finish_loading("poem-1");
        assert!(!state.loading);
        assert_eq!(state.loading_poem_id, None);
    }
}
