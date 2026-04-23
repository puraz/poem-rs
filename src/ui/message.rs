use crate::domain::DiscoveredPoem;
use iced::widget::text_editor;

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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeChoice {
    Songyanjian,
    Hanjiangxue,
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
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Songyanjian => "松烟笺",
            Self::Hanjiangxue => "寒江雪",
        }
    }

    pub fn from_saved(value: Option<&str>) -> Self {
        match value {
            Some("hanjiangxue") => Self::Hanjiangxue,
            _ => Self::Songyanjian,
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
    SettingsFallbackChanged(bool),
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
    AppreciationLoaded(Result<AppreciationResult, String>),
    SwitchTheme(ThemeChoice),
    DismissToast,
    ToastExpired(u64),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedPoem {
    pub poem_id: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsSaveResult {
    pub message: String,
    pub warning: String,
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
