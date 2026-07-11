use anyhow::{Result, anyhow};

use crate::config::ai::KeyringSecretStore;
use crate::domain::{AiAppreciation, DiscoveredPoem, Poem};
use crate::services::ai::{
    HttpAiTransport, OpenAiCompatibleClient, build_appreciation_prompt, build_discovery_prompt,
};
use crate::storage::{AppDatabase, StoredAiConfig};

use super::message::{
    AppreciationFailure, AppreciationResult, EditedPoem, ImportedPoem, SettingsSaveResult,
};
use super::state::{EditForm, current_secret};

pub async fn run_discovery_search(
    config: StoredAiConfig,
    query: String,
) -> Result<Vec<DiscoveredPoem>, String> {
    let (secret, _) = current_secret();
    let secret =
        secret.ok_or_else(|| "AI 未配置，请先在设置中填写可用模型与 API Key。".to_string())?;
    let prompt = build_discovery_prompt(&query);
    let client = OpenAiCompatibleClient::new(HttpAiTransport::new(config.settings, Some(secret)));

    client
        .discover(&prompt)
        .await
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
    config: StoredAiConfig,
    poem: Poem,
) -> Result<AiAppreciation, String> {
    let (secret, _) = current_secret();
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
        .await
        .map_err(|err| format!("AI 赏析失败: {err:?}"))
}

pub async fn generate_and_persist_appreciation(
    db: AppDatabase,
    config: StoredAiConfig,
    poem: Poem,
) -> Result<AppreciationResult, AppreciationFailure> {
    let poem_id = poem.id.clone();
    let model = config.settings.model.clone();
    let appreciation = request_appreciation(config, poem)
        .await
        .map_err(|message| AppreciationFailure {
            poem_id: poem_id.clone(),
            message,
        })?;

    db.save_cached_analysis(&poem_id, &appreciation, &model)
        .map_err(|err| AppreciationFailure {
            poem_id: poem_id.clone(),
            message: format!("保存赏析缓存失败: {err}"),
        })?;

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
    db: AppDatabase,
    config: StoredAiConfig,
    api_key: String,
) -> Result<SettingsSaveResult, String> {
    db.save_ai_config(&config)
        .map_err(|err| format!("保存设置失败: {err}"))?;

    if !api_key.trim().is_empty() {
        store_secret(api_key.trim())
            .map_err(|err| format!("保存 API Key 失败: {err}"))?;
    }

    Ok(SettingsSaveResult {
        message: "设置已保存".to_string(),
    })
}

pub async fn clear_api_key() -> Result<SettingsSaveResult, String> {
    let keyring = KeyringSecretStore;
    keyring
        .clear()
        .map_err(|err| format!("清除 API Key 失败: {err}"))?;

    Ok(SettingsSaveResult {
        message: "API Key 已清除".to_string(),
    })
}

fn store_secret(api_key: &str) -> Result<()> {
    let keyring = KeyringSecretStore;

    if KeyringSecretStore::is_available() {
        keyring.save_api_key(api_key)?;
        return Ok(());
    }

    Err(anyhow!("系统钥匙串不可用，无法保存 API Key"))
}

pub async fn export_all(db: AppDatabase) -> Result<String, String> {
    let json = db
        .export_all_as_json()
        .map_err(|err| format!("导出失败: {err}"))?;

    let path = std::thread::spawn(|| {
        rfd::FileDialog::new()
            .set_title("导出诗词")
            .set_file_name("poems-backup.json")
            .add_filter("JSON", &["json"])
            .save_file()
    })
    .join()
    .map_err(|_| "文件对话框错误".to_string())?
    .ok_or_else(|| "已取消导出".to_string())?;

    std::fs::write(&path, &json).map_err(|err| format!("写入文件失败: {err}"))?;

    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("poems-backup.json")
        .to_string();
    Ok(name)
}

pub async fn import_all(db: AppDatabase) -> Result<usize, String> {
    let path = std::thread::spawn(|| {
        rfd::FileDialog::new()
            .set_title("导入诗词")
            .add_filter("JSON", &["json"])
            .pick_file()
    })
    .join()
    .map_err(|_| "文件对话框错误".to_string())?
    .ok_or_else(|| "已取消导入".to_string())?;

    let json = std::fs::read_to_string(&path)
        .map_err(|err| format!("读取文件失败: {err}"))?;

    db.import_from_json_str(&json)
        .map_err(|err| format!("导入失败: {err}"))
}
