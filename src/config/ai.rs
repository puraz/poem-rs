use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use keyring::use_native_store;
use keyring_core::Entry;

pub const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";
pub const DEFAULT_MODEL: &str = "gpt-4.1-mini";
pub const DEFAULT_TIMEOUT_SECS: u64 = 20;
pub const SECRET_FILE_NAME: &str = "ai-secret.toml";
pub const FILE_FALLBACK_WARNING: &str =
    "API key file fallback is less secure than keyring storage and should require explicit opt-in.";
pub const KEYRING_SERVICE: &str = "poem-rs";
pub const KEYRING_USERNAME: &str = "openai-compatible-api-key";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiMode {
    Unconfigured,
    Configured,
    FallbackStorage,
    Unavailable,
}

impl AiMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Unconfigured => "未配置",
            Self::Configured => "已配置",
            Self::FallbackStorage => "回退存储",
            Self::Unavailable => "不可用",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecretPersistencePlan {
    Keyring,
    WarnedFileFallback,
    LocalOnly,
    Unavailable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AiSettings {
    pub base_url: String,
    pub model: String,
    pub timeout_secs: u64,
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
            model: DEFAULT_MODEL.to_string(),
            timeout_secs: DEFAULT_TIMEOUT_SECS,
        }
    }
}

impl AiSettings {
    pub fn mode_for(&self, has_secret: bool, persistence: SecretPersistencePlan) -> AiMode {
        let _ = self;

        if !has_secret {
            return match persistence {
                SecretPersistencePlan::Unavailable => AiMode::Unavailable,
                _ => AiMode::Unconfigured,
            };
        }

        match persistence {
            SecretPersistencePlan::Keyring | SecretPersistencePlan::LocalOnly => AiMode::Configured,
            SecretPersistencePlan::WarnedFileFallback => AiMode::FallbackStorage,
            SecretPersistencePlan::Unavailable => AiMode::Unavailable,
        }
    }
}

pub fn plan_secret_persistence(
    keyring_available: bool,
    persist_requested: bool,
    allow_file_fallback: bool,
) -> SecretPersistencePlan {
    if !persist_requested {
        return SecretPersistencePlan::LocalOnly;
    }

    if keyring_available {
        return SecretPersistencePlan::Keyring;
    }

    if allow_file_fallback {
        return SecretPersistencePlan::WarnedFileFallback;
    }

    SecretPersistencePlan::Unavailable
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileSecretStore {
    path: PathBuf,
}

impl FileSecretStore {
    pub fn new(config_dir: impl AsRef<Path>) -> Self {
        Self {
            path: config_dir.as_ref().join(SECRET_FILE_NAME),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn save_api_key(&self, api_key: &str) -> io::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&self.path, format_secret_file(api_key))?;
        restrict_permissions(&self.path)?;
        Ok(())
    }

    pub fn load_api_key(&self) -> io::Result<Option<String>> {
        match fs::read_to_string(&self.path) {
            Ok(contents) => parse_secret_file(&contents).map(Some),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }

    pub fn clear(&self) -> io::Result<()> {
        match fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct KeyringSecretStore;

impl KeyringSecretStore {
    pub fn is_available() -> bool {
        use_native_store(false).is_ok()
    }

    pub fn save_api_key(&self, api_key: &str) -> io::Result<()> {
        Self::entry()?
            .set_password(api_key)
            .map_err(map_keyring_error)
    }

    pub fn load_api_key(&self) -> io::Result<Option<String>> {
        match Self::entry()?.get_password() {
            Ok(password) => Ok(Some(password)),
            Err(err) if is_missing_credential(&err) => Ok(None),
            Err(err) => Err(map_keyring_error(err)),
        }
    }

    pub fn clear(&self) -> io::Result<()> {
        match Self::entry()?.delete_credential() {
            Ok(()) => Ok(()),
            Err(err) if is_missing_credential(&err) => Ok(()),
            Err(err) => Err(map_keyring_error(err)),
        }
    }

    fn entry() -> io::Result<Entry> {
        Entry::new(KEYRING_SERVICE, KEYRING_USERNAME).map_err(map_keyring_error)
    }
}

fn is_missing_credential(err: &keyring_core::Error) -> bool {
    matches!(err, keyring_core::Error::NoEntry)
}

fn map_keyring_error(err: keyring_core::Error) -> io::Error {
    io::Error::other(err.to_string())
}

fn format_secret_file(api_key: &str) -> String {
    let escaped = api_key.replace('\\', "\\\\").replace('"', "\\\"");
    format!("api_key = \"{escaped}\"\n")
}

fn parse_secret_file(contents: &str) -> io::Result<String> {
    let Some(line) = contents
        .lines()
        .find(|line| line.trim_start().starts_with("api_key"))
    else {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            "missing api_key entry in fallback secret file",
        ));
    };

    let (_, value) = line
        .split_once('=')
        .ok_or_else(|| io::Error::new(ErrorKind::InvalidData, "invalid api_key assignment"))?;
    let trimmed = value.trim();

    if !(trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2) {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            "api_key value must be a quoted string",
        ));
    }

    unescape_secret(&trimmed[1..trimmed.len() - 1])
}

fn unescape_secret(value: &str) -> io::Result<String> {
    let mut chars = value.chars();
    let mut output = String::with_capacity(value.len());

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            output.push(ch);
            continue;
        }

        match chars.next() {
            Some('\\') => output.push('\\'),
            Some('"') => output.push('"'),
            Some(other) => {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    format!("unsupported escape sequence: \\{other}"),
                ));
            }
            None => {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    "trailing escape in api_key value",
                ));
            }
        }
    }

    Ok(output)
}

fn restrict_permissions(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        let permissions = fs::Permissions::from_mode(0o600);
        fs::set_permissions(path, permissions)?;
    }

    #[cfg(not(unix))]
    {
        let _ = path;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("poem-rs-ai-config-{nanos}"))
    }

    #[test]
    fn chooses_keyring_when_available() {
        assert_eq!(
            plan_secret_persistence(true, true, false),
            SecretPersistencePlan::Keyring
        );
    }

    #[test]
    fn chooses_warned_file_fallback_when_opted_in() {
        assert_eq!(
            plan_secret_persistence(false, true, true),
            SecretPersistencePlan::WarnedFileFallback
        );
    }

    #[test]
    fn stays_local_only_when_persistence_not_requested() {
        assert_eq!(
            plan_secret_persistence(false, false, false),
            SecretPersistencePlan::LocalOnly
        );
    }

    #[test]
    fn settings_mode_tracks_secret_persistence() {
        let settings = AiSettings::default();
        assert_eq!(
            settings.mode_for(true, SecretPersistencePlan::WarnedFileFallback),
            AiMode::FallbackStorage
        );
        assert_eq!(
            settings.mode_for(false, SecretPersistencePlan::Unavailable),
            AiMode::Unavailable
        );
    }

    #[test]
    fn file_secret_store_round_trips() {
        let temp_dir = unique_temp_dir();
        let store = FileSecretStore::new(&temp_dir);

        store
            .save_api_key("secret-123")
            .expect("save fallback secret");
        let loaded = store.load_api_key().expect("load fallback secret");
        assert_eq!(loaded.as_deref(), Some("secret-123"));

        store.clear().expect("clear fallback secret");
        let cleared = store.load_api_key().expect("load cleared fallback secret");
        assert_eq!(cleared, None);
        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn file_fallback_uses_secure_permissions_on_unix() {
        let temp_dir = unique_temp_dir();
        let store = FileSecretStore::new(&temp_dir);
        store
            .save_api_key("secret-xyz")
            .expect("save fallback secret");

        #[cfg(unix)]
        {
            let metadata = std::fs::metadata(store.path()).expect("fallback metadata");
            assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
        }

        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
