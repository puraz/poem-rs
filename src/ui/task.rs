use anyhow::{Result, anyhow};

use crate::config::ai::{FileSecretStore, KeyringSecretStore};
use crate::config::app::AppPaths;
use crate::domain::{AiAppreciation, DiscoveredPoem, Poem};
use crate::services::ai::{
    HttpAiTransport, OpenAiCompatibleClient, build_appreciation_prompt, build_discovery_prompt,
};
use crate::storage::{AppDatabase, StoredAiConfig};

use super::message::{AppreciationResult, EditedPoem, ImportedPoem, SettingsSaveResult};
use super::state::{EditForm, current_secret};

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

pub async fn request_appreciation(
    paths: AppPaths,
    config: StoredAiConfig,
    poem: Poem,
) -> Result<AiAppreciation, String> {
    let (secret, _) = current_secret(&paths, config.allow_file_fallback);
    let secret =
        secret.ok_or_else(|| "AI 未配置，请先在设置中填写可用模型与 API Key。".to_string())?;
    let client = OpenAiCompatibleClient::new(HttpAiTransport::new(config.settings, Some(secret)));
    let prompt = build_appreciation_prompt(
        &poem.id,
        &poem.title,
        &poem.author,
        &poem.dynasty,
        &poem.content,
    );

    client
        .appreciate(&prompt)
        .map_err(|err| format!("AI 赏析失败: {err:?}"))
}

pub async fn generate_and_persist_appreciation(
    paths: AppPaths,
    db: AppDatabase,
    config: StoredAiConfig,
    poem: Poem,
) -> Result<AppreciationResult, String> {
    let poem_id = poem.id.clone();
    let model = config.settings.model.clone();
    let appreciation = request_appreciation(paths, config, poem).await?;

    db.save_cached_analysis(&poem_id, &appreciation, &model)
        .map_err(|err| format!("保存赏析缓存失败: {err}"))?;

    Ok(AppreciationResult {
        poem_id,
        content: appreciation.display_text(),
    })
}

pub async fn save_edited_poem(db: AppDatabase, form: EditForm) -> Result<EditedPoem, String> {
    db.update_poem(
        &form.poem_id,
        form.title.trim(),
        form.author.trim(),
        form.dynasty.trim(),
        form.content.trim(),
    )
    .map_err(|err| format!("保存诗词失败: {err}"))?;

    Ok(EditedPoem {
        poem_id: form.poem_id,
        title: form.title.trim().to_string(),
    })
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
