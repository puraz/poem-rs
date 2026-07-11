use std::io;

use keyring::Entry;

pub const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";
pub const DEFAULT_MODEL: &str = "gpt-4.1-mini";
pub const DEFAULT_TIMEOUT_SECS: u64 = 120;
pub const KEYRING_SERVICE: &str = "poem-rs";
pub const KEYRING_USERNAME: &str = "openai-compatible-api-key";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiMode {
    Unconfigured,
    Configured,
    Unavailable,
}

impl AiMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Unconfigured => "未配置",
            Self::Configured => "已配置",
            Self::Unavailable => "不可用",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecretPersistencePlan {
    Keyring,
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
    pub fn effective_timeout_secs(&self) -> u64 {
        self.timeout_secs.max(DEFAULT_TIMEOUT_SECS)
    }

    pub fn mode_for(&self, has_secret: bool, persistence: SecretPersistencePlan) -> AiMode {
        let _ = self;

        match persistence {
            SecretPersistencePlan::Unavailable => AiMode::Unavailable,
            _ if has_secret => AiMode::Configured,
            _ => AiMode::Unconfigured,
        }
    }
}

pub struct KeyringSecretStore;

impl KeyringSecretStore {
    pub fn is_available() -> bool {
        Entry::new(KEYRING_SERVICE, KEYRING_USERNAME).is_ok()
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

fn is_missing_credential(err: &keyring::Error) -> bool {
    matches!(err, keyring::Error::NoEntry)
}

fn map_keyring_error(err: keyring::Error) -> io::Error {
    io::Error::other(err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeout_never_drops_below_default() {
        let mut settings = AiSettings {
            timeout_secs: DEFAULT_TIMEOUT_SECS - 1,
            ..AiSettings::default()
        };
        assert_eq!(settings.effective_timeout_secs(), DEFAULT_TIMEOUT_SECS);

        settings.timeout_secs = DEFAULT_TIMEOUT_SECS + 30;
        assert_eq!(settings.effective_timeout_secs(), DEFAULT_TIMEOUT_SECS + 30);
    }

    #[test]
    fn settings_mode_tracks_secret_persistence() {
        let settings = AiSettings::default();
        assert_eq!(
            settings.mode_for(true, SecretPersistencePlan::Keyring),
            AiMode::Configured
        );
        assert_eq!(
            settings.mode_for(false, SecretPersistencePlan::Unavailable),
            AiMode::Unavailable
        );
        assert_eq!(
            settings.mode_for(false, SecretPersistencePlan::Keyring),
            AiMode::Unconfigured
        );
    }
}
