use anyhow::{Result, anyhow};

use crate::config::ai::{FileSecretStore, KeyringSecretStore};
use crate::config::app::AppPaths;
use crate::domain::DiscoveredPoem;
use crate::services::ai::{HttpAiTransport, OpenAiCompatibleClient, build_discovery_prompt};
use crate::storage::{AppDatabase, StoredAiConfig};

use super::message::{ImportedPoem, SettingsSaveResult};
use super::state::current_secret;

pub async fn run_discovery_search(
    paths: AppPaths,
    config: StoredAiConfig,
    query: String,
) -> Result<Vec<DiscoveredPoem>, String> {
    let (secret, _) = current_secret(&paths, config.allow_file_fallback);
    let secret =
        secret.ok_or_else(|| "AI 未配置，请先在设置中填写可用模型与 API Key。".to_string())?;
    let prompt = build_discovery_prompt(&query);
    let client = OpenAiCompatibleClient::new(HttpAiTransport::new(config.settings, Some(secret)));

    client
        .discover(&prompt)
        .map(|payload| payload.poems)
        .map_err(|err| format!("AI 搜索失败: {err:?}"))
}

pub async fn import_discovery_poem(
    db: AppDatabase,
    poem: DiscoveredPoem,
) -> Result<ImportedPoem, String> {
    db.insert_imported_poem(&poem)
        .map(|poem_id| ImportedPoem {
            poem_id,
            title: poem.title,
        })
        .map_err(|err| format!("导入失败: {err}"))
}

pub async fn save_settings(
    paths: AppPaths,
    db: AppDatabase,
    config: StoredAiConfig,
    api_key: String,
) -> Result<SettingsSaveResult, String> {
    db.save_ai_config(&config)
        .map_err(|err| format!("保存设置失败: {err}"))?;

    if !api_key.trim().is_empty() {
        store_secret(&paths, &config, api_key.trim())
            .map_err(|err| format!("保存 API Key 失败: {err}"))?;
    }

    let (_, persistence) = current_secret(&paths, config.allow_file_fallback);
    let warning = if matches!(
        persistence,
        crate::config::ai::SecretPersistencePlan::WarnedFileFallback
    ) {
        crate::config::ai::FILE_FALLBACK_WARNING.to_string()
    } else {
        String::new()
    };

    Ok(SettingsSaveResult {
        message: "设置已保存".to_string(),
        warning,
    })
}

pub async fn clear_api_key(paths: AppPaths) -> Result<SettingsSaveResult, String> {
    let keyring = KeyringSecretStore;
    let file_store = FileSecretStore::new(paths.config_dir());
    keyring
        .clear()
        .map_err(|err| format!("清除 keyring 中的 API Key 失败: {err}"))?;
    file_store
        .clear()
        .map_err(|err| format!("清除本地回退 API Key 失败: {err}"))?;

    Ok(SettingsSaveResult {
        message: "API Key 已清除".to_string(),
        warning: String::new(),
    })
}

fn store_secret(paths: &AppPaths, config: &StoredAiConfig, api_key: &str) -> Result<()> {
    let keyring = KeyringSecretStore;
    let file_store = FileSecretStore::new(paths.config_dir());

    if KeyringSecretStore::is_available() {
        keyring.save_api_key(api_key)?;
        let _ = file_store.clear();
        return Ok(());
    }

    if config.allow_file_fallback {
        file_store.save_api_key(api_key)?;
        return Ok(());
    }

    Err(anyhow!("{}", crate::config::ai::FILE_FALLBACK_WARNING))
}
