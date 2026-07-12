use crate::domain::DiscoveredPoem;
use iced::{theme, widget::text_editor};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modal {
    None,
    Discovery,
    Settings,
    About,
    Edit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentMode {
    Library,
    Favorites,
    PoetPage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeChoice {
    Songyanjian,
    Hanjiangxue,
    FollowSystem,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailTool {
    Favorite,
    Edit,
    Appreciation,
}

impl ThemeChoice {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Songyanjian => "songyanjian",
            Self::Hanjiangxue => "hanjiangxue",
            Self::FollowSystem => "system",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Songyanjian => "寒江雪",
            Self::Hanjiangxue => "松烟笺",
            Self::FollowSystem => "跟随系统",
        }
    }

    pub fn from_saved(value: Option<&str>) -> Self {
        match value {
            Some("hanjiangxue") => Self::Hanjiangxue,
            Some("system") => Self::FollowSystem,
            _ => Self::Songyanjian,
        }
    }

    pub fn resolve(self, system_mode: theme::Mode) -> Self {
        match self {
            Self::FollowSystem if system_mode == theme::Mode::Dark => Self::Hanjiangxue,
            Self::FollowSystem => Self::Songyanjian,
            explicit => explicit,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectPoem(String),
    SearchChanged(String),
    SwitchContentMode(ContentMode),
    OpenModal(Modal),
    CloseModal,
    ToggleFavorite,
    DiscoveryQueryChanged(String),
    SubmitDiscovery,
    DiscoveryLoaded(Result<Vec<DiscoveredPoem>, String>),
    ImportDiscovery(usize),
    ImportFinished(Result<ImportedPoem, String>),
    SettingsBaseUrlChanged(String),
    SettingsModelChanged(String),
    SettingsApiKeyChanged(String),
    SaveSettings,
    SettingsSaved(Result<SettingsSaveResult, String>),
    ClearApiKey,
    ApiKeyCleared(Result<SettingsSaveResult, String>),
    OpenEditModal,
    EditTitleChanged(String),
    EditAuthorChanged(String),
    EditDynastyChanged(String),
    EditContentChanged(text_editor::Action),
    HoverDetailTool(Option<DetailTool>),
    SaveEdit,
    EditSaved(Result<EditedPoem, String>),
    RequestAppreciation,
    AppreciationLoaded(Result<AppreciationResult, AppreciationFailure>),
    LoadingTick,
    ToggleThemePanel,
    CloseThemePanel,
    SwitchTheme(ThemeChoice),
    SystemThemeChanged(theme::Mode),
    DismissToast,
    ToastExpired(u64),
    ExportPoems,
    ImportPoems,
    ExportFinished(Result<String, String>),
    BulkImportFinished(Result<usize, String>),
    PoetNameClicked(String),
    PoetFilterChanged(String),
    RefreshPoetProfile(String),
    PoetProfileLoaded(Result<PoetProfileLoadedPayload, String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedPoem {
    pub poem_id: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsSaveResult {
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditedPoem {
    pub poem_id: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppreciationResult {
    pub poem_id: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppreciationFailure {
    pub poem_id: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoetProfileLoadedPayload {
    pub poet_name: String,
    pub content: String,
}

#[cfg(test)]
mod tests {
    use super::ThemeChoice;
    use iced::theme::Mode;

    #[test]
    fn theme_display_names_match_current_visual_mapping() {
        assert_eq!(ThemeChoice::Songyanjian.display_name(), "寒江雪");
        assert_eq!(ThemeChoice::Hanjiangxue.display_name(), "松烟笺");
        assert_eq!(ThemeChoice::FollowSystem.display_name(), "跟随系统");
    }

    #[test]
    fn follow_system_resolves_against_theme_mode() {
        assert_eq!(
            ThemeChoice::FollowSystem.resolve(Mode::Light),
            ThemeChoice::Songyanjian
        );
        assert_eq!(
            ThemeChoice::FollowSystem.resolve(Mode::Dark),
            ThemeChoice::Hanjiangxue
        );
    }
}
