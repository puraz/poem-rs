use crate::domain::DiscoveredPoem;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Modal {
    None,
    Discovery,
    Settings,
    About,
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectPoem(String),
    SearchChanged(String),
    OpenModal(Modal),
    CloseModal,
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
