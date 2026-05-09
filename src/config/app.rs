use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use directories::ProjectDirs;

#[derive(Clone, Debug)]
pub struct AppPaths {
    app_dir: PathBuf,
    config_dir: PathBuf,
    data_dir: PathBuf,
    db_path: PathBuf,
}

impl AppPaths {
    pub fn resolve() -> Result<Self> {
        let dirs = ProjectDirs::from("rs", "puraz", "poem-rs")
            .context("failed to resolve project directories for poem-rs")?;
        let app_dir = dirs.config_dir().parent().map(Path::to_path_buf).context(
            "failed to resolve app root directory from config dir for poem-rs",
        )?;
        let config_dir = dirs.config_dir().to_path_buf();
        let data_dir = dirs.data_dir().to_path_buf();
        std::fs::create_dir_all(&app_dir)
            .with_context(|| format!("failed to create app dir {}", app_dir.display()))?;
        std::fs::create_dir_all(&config_dir)
            .with_context(|| format!("failed to create config dir {}", config_dir.display()))?;
        std::fs::create_dir_all(&data_dir)
            .with_context(|| format!("failed to create data dir {}", data_dir.display()))?;

        let db_path = data_dir.join("poems.sqlite3");
        Ok(Self {
            app_dir,
            config_dir,
            data_dir,
            db_path,
        })
    }

    pub fn app_dir(&self) -> &Path {
        &self.app_dir
    }

    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }
}
