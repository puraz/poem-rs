use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::config::ai::AiSettings;
use crate::domain::{AiAppreciation, Poem};

const MANIFEST_JSON: &str = include_str!("../../assets/poetry/manifest.json");
const CORPUS_JSON: &str = include_str!("../../assets/poetry/corpus.json");

#[derive(Clone, Debug)]
pub struct AppDatabase {
    path: PathBuf,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StoredAiConfig {
    pub settings: AiSettings,
    pub allow_file_fallback: bool,
}

#[derive(Clone, Debug)]
pub struct PoetrySnapshot {
    pub poems: Vec<Poem>,
    pub favorites: Vec<Poem>,
}

impl AppDatabase {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn bootstrap(&self) -> Result<()> {
        let conn = self.connect()?;
        self.create_schema(&conn)?;
        self.import_seed_if_needed(&conn)?;
        Ok(())
    }

    pub fn list_poems(&self) -> Result<Vec<Poem>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT p.id, p.title, p.author, p.dynasty, p.content, p.tags_json, p.source, p.license,
                   EXISTS(SELECT 1 FROM favorites f WHERE f.poem_id = p.id) AS is_favorite
            FROM poems p
            ORDER BY p.dynasty, p.author, p.title
            "#,
        )?;

        let rows = stmt.query_map([], map_poem)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn list_favorites(&self) -> Result<Vec<Poem>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT p.id, p.title, p.author, p.dynasty, p.content, p.tags_json, p.source, p.license, 1 AS is_favorite
            FROM poems p
            INNER JOIN favorites f ON f.poem_id = p.id
            ORDER BY f.created_at DESC, p.title
            "#,
        )?;
        let rows = stmt.query_map([], map_poem)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn get_poem(&self, poem_id: &str) -> Result<Option<Poem>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT p.id, p.title, p.author, p.dynasty, p.content, p.tags_json, p.source, p.license,
                   EXISTS(SELECT 1 FROM favorites f WHERE f.poem_id = p.id) AS is_favorite
            FROM poems p
            WHERE p.id = ?1
            "#,
        )?;
        Ok(stmt.query_row([poem_id], map_poem).optional()?)
    }

    pub fn toggle_favorite(&self, poem_id: &str) -> Result<bool> {
        let conn = self.connect()?;
        let exists: Option<String> = conn
            .query_row(
                "SELECT poem_id FROM favorites WHERE poem_id = ?1",
                [poem_id],
                |row| row.get(0),
            )
            .optional()?;

        if exists.is_some() {
            conn.execute("DELETE FROM favorites WHERE poem_id = ?1", [poem_id])?;
            Ok(false)
        } else {
            conn.execute(
                "INSERT INTO favorites(poem_id, created_at) VALUES (?1, ?2)",
                params![poem_id, now_string()],
            )?;
            Ok(true)
        }
    }

    pub fn load_ai_config(&self) -> Result<StoredAiConfig> {
        let conn = self.connect()?;
        let values = self.load_meta_map(&conn)?;
        Ok(StoredAiConfig {
            settings: AiSettings {
                base_url: values
                    .get("ai.base_url")
                    .cloned()
                    .unwrap_or_else(|| AiSettings::default().base_url),
                model: values
                    .get("ai.model")
                    .cloned()
                    .unwrap_or_else(|| AiSettings::default().model),
                timeout_secs: values
                    .get("ai.timeout_secs")
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(AiSettings::default().timeout_secs),
            },
            allow_file_fallback: values
                .get("ai.allow_file_fallback")
                .map(|v| v == "1")
                .unwrap_or(false),
        })
    }

    pub fn save_ai_config(&self, config: &StoredAiConfig) -> Result<()> {
        let conn = self.connect()?;
        self.set_meta(&conn, "ai.base_url", &config.settings.base_url)?;
        self.set_meta(&conn, "ai.model", &config.settings.model)?;
        self.set_meta(
            &conn,
            "ai.timeout_secs",
            &config.settings.timeout_secs.to_string(),
        )?;
        self.set_meta(
            &conn,
            "ai.allow_file_fallback",
            if config.allow_file_fallback { "1" } else { "0" },
        )?;
        Ok(())
    }

    pub fn load_cached_analysis(&self, poem_id: &str) -> Result<Option<AiAppreciation>> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare("SELECT analysis_markdown FROM analysis_cache WHERE poem_id = ?1 LIMIT 1")?;
        let text = stmt
            .query_row([poem_id], |row| row.get::<_, String>(0))
            .optional()?;
        Ok(text
            .map(|notes| AiAppreciation::new(poem_id, "已缓存赏析", Vec::new(), Vec::new(), notes)))
    }

    pub fn save_cached_analysis(
        &self,
        poem_id: &str,
        analysis: &AiAppreciation,
        model: &str,
    ) -> Result<()> {
        let conn = self.connect()?;
        conn.execute(
            r#"
            INSERT INTO analysis_cache(poem_id, analysis_markdown, model, updated_at)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(poem_id) DO UPDATE SET
                analysis_markdown = excluded.analysis_markdown,
                model = excluded.model,
                updated_at = excluded.updated_at
            "#,
            params![poem_id, analysis.display_text(), model, now_string()],
        )?;
        Ok(())
    }

    pub fn poetry_snapshot(&self) -> Result<PoetrySnapshot> {
        Ok(PoetrySnapshot {
            poems: self.list_poems()?,
            favorites: self.list_favorites()?,
        })
    }

    fn connect(&self) -> Result<Connection> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Ok(Connection::open(&self.path)?)
    }

    fn create_schema(&self, conn: &Connection) -> Result<()> {
        conn.execute_batch(
            r#"
            PRAGMA foreign_keys = ON;
            CREATE TABLE IF NOT EXISTS poems (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                author TEXT NOT NULL,
                dynasty TEXT NOT NULL,
                content TEXT NOT NULL,
                tags_json TEXT NOT NULL DEFAULT '[]',
                source TEXT NOT NULL,
                license TEXT NOT NULL,
                checksum TEXT NOT NULL,
                seed_version INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS favorites (
                poem_id TEXT PRIMARY KEY REFERENCES poems(id) ON DELETE CASCADE,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS app_meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS analysis_cache (
                poem_id TEXT PRIMARY KEY REFERENCES poems(id) ON DELETE CASCADE,
                analysis_markdown TEXT NOT NULL,
                model TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            "#,
        )?;
        Ok(())
    }

    fn import_seed_if_needed(&self, conn: &Connection) -> Result<()> {
        let manifest: SeedManifest = serde_json::from_str(MANIFEST_JSON)?;
        let corpus_checksum = checksum_hex(CORPUS_JSON.as_bytes());
        let expected = manifest
            .files
            .iter()
            .find(|entry| entry.path == "corpus.json")
            .context("manifest missing corpus.json entry")?;

        if expected.sha256 != corpus_checksum {
            anyhow::bail!("corpus checksum mismatch between manifest and embedded corpus")
        }

        let current_version = self
            .get_meta(conn, "seed_version")
            .ok()
            .flatten()
            .and_then(|value| value.parse::<i64>().ok())
            .unwrap_or_default();
        let current_checksum = self.get_meta(conn, "seed_checksum").ok().flatten();

        if current_version == manifest.seed_version
            && current_checksum.as_deref() == Some(&corpus_checksum)
        {
            return Ok(());
        }

        let seed_poems: Vec<SeedPoem> = serde_json::from_str(CORPUS_JSON)?;
        let tx = conn.unchecked_transaction()?;
        {
            let mut stmt = tx.prepare(
                r#"
                INSERT INTO poems(id, title, author, dynasty, content, tags_json, source, license, checksum, seed_version)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                ON CONFLICT(id) DO UPDATE SET
                    title = excluded.title,
                    author = excluded.author,
                    dynasty = excluded.dynasty,
                    content = excluded.content,
                    tags_json = excluded.tags_json,
                    source = excluded.source,
                    license = excluded.license,
                    checksum = excluded.checksum,
                    seed_version = excluded.seed_version
                "#,
            )?;
            for poem in seed_poems {
                let content = poem.lines.join("\n");
                let checksum = checksum_hex(content.as_bytes());
                stmt.execute(params![
                    poem.id,
                    poem.title,
                    poem.author,
                    poem.dynasty,
                    content,
                    serde_json::to_string(&poem.tags)?,
                    poem.source,
                    poem.license,
                    checksum,
                    manifest.seed_version,
                ])?;
            }
        }

        self.set_meta_tx(&tx, "seed_version", &manifest.seed_version.to_string())?;
        self.set_meta_tx(&tx, "seed_checksum", &corpus_checksum)?;
        tx.commit()?;
        Ok(())
    }

    fn load_meta_map(&self, conn: &Connection) -> Result<HashMap<String, String>> {
        let mut stmt = conn.prepare("SELECT key, value FROM app_meta")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        Ok(rows.collect::<rusqlite::Result<HashMap<_, _>>>()?)
    }

    fn get_meta(&self, conn: &Connection, key: &str) -> Result<Option<String>> {
        Ok(conn
            .query_row("SELECT value FROM app_meta WHERE key = ?1", [key], |row| {
                row.get(0)
            })
            .optional()?)
    }

    fn set_meta(&self, conn: &Connection, key: &str, value: &str) -> Result<()> {
        self.set_meta_tx(conn, key, value)
    }

    fn set_meta_tx(&self, conn: &Connection, key: &str, value: &str) -> Result<()> {
        conn.execute(
            r#"
            INSERT INTO app_meta(key, value) VALUES (?1, ?2)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value
            "#,
            params![key, value],
        )?;
        Ok(())
    }
}

fn map_poem(row: &rusqlite::Row<'_>) -> rusqlite::Result<Poem> {
    let tags_json: String = row.get(5)?;
    let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
    Ok(Poem {
        id: row.get(0)?,
        title: row.get(1)?,
        author: row.get(2)?,
        dynasty: row.get(3)?,
        content: row.get(4)?,
        tags,
        source: row.get(6)?,
        license: row.get(7)?,
        is_favorite: row.get::<_, i64>(8)? != 0,
    })
}

fn now_string() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|_| "0".into())
}

fn checksum_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("{digest:x}")
}

#[derive(Debug, Deserialize)]
struct SeedManifest {
    seed_version: i64,
    files: Vec<SeedFile>,
}

#[derive(Debug, Deserialize)]
struct SeedFile {
    path: String,
    sha256: String,
}

#[derive(Debug, Deserialize)]
struct SeedPoem {
    id: String,
    title: String,
    author: String,
    dynasty: String,
    tags: Vec<String>,
    lines: Vec<String>,
    source: String,
    license: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db_path(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("poem-rs-{name}-{nanos}.sqlite3"))
    }

    #[test]
    fn bootstrap_imports_seed_idempotently() {
        let path = temp_db_path("bootstrap");
        let db = AppDatabase::new(&path);
        db.bootstrap().expect("bootstrap 1");
        let first_count = db.list_poems().expect("list poems").len();
        db.bootstrap().expect("bootstrap 2");
        let second_count = db.list_poems().expect("list poems 2").len();
        assert_eq!(first_count, second_count);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn favorites_persist() {
        let path = temp_db_path("favorites");
        let db = AppDatabase::new(&path);
        db.bootstrap().expect("bootstrap");
        let poem_id = db.list_poems().expect("poems")[0].id.clone();
        assert!(db.toggle_favorite(&poem_id).expect("favorite on"));
        assert!(!db.list_favorites().expect("favorites").is_empty());
        assert!(!db.toggle_favorite(&poem_id).expect("favorite off"));
        let _ = std::fs::remove_file(path);
    }
}
