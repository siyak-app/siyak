//! API keys in the OS keychain (service `com.kemalozyon.siyak`, account
//! `<provider>`). Keys are never written to a database and never returned to
//! the webview: the interface only learns whether a key is saved (spec §14.2).
//!
//! The embedding APIs (spec §7.4) and the AI layer (spec §8) share these keys:
//! one per [`ProviderId`]. A key read once is kept in memory, so a request does
//! not ask the keychain each time (unsigned development builds may make macOS
//! ask the user for every read).

use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::embed::ApiProvider;
use crate::error::{AppError, AppErrorCode, AppResult};

/// A provider whose key may be saved: every embedding API ([`ApiProvider`])
/// plus Anthropic, which offers chat models but no embeddings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
pub enum ProviderId {
    /// Anthropic (Claude), chat only.
    #[serde(rename = "anthropic")]
    Anthropic,
    /// OpenAI.
    #[serde(rename = "openai")]
    OpenAi,
    /// Voyage AI, embeddings only.
    #[serde(rename = "voyage")]
    Voyage,
    /// Google Gemini.
    #[serde(rename = "gemini")]
    Gemini,
    /// An OpenAI-compatible server at a user-entered address.
    #[serde(rename = "openai_compatible")]
    OpenAiCompatible,
}

impl ProviderId {
    /// Every provider, in display order.
    pub const ALL: [Self; 5] = [
        Self::Anthropic,
        Self::OpenAi,
        Self::Voyage,
        Self::Gemini,
        Self::OpenAiCompatible,
    ];

    /// The keychain account of the provider's key; the embedding providers
    /// keep the values of [`ApiProvider::as_str`], so saved keys stay found.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::OpenAi => "openai",
            Self::Voyage => "voyage",
            Self::Gemini => "gemini",
            Self::OpenAiCompatible => "openai_compatible",
        }
    }

    /// The provider for a stored value.
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.as_str() == value)
    }

    /// Whether requests need a key; a local OpenAI-compatible server often
    /// runs without one.
    pub fn needs_key(self) -> bool {
        !matches!(self, Self::OpenAiCompatible)
    }
}

impl From<ApiProvider> for ProviderId {
    fn from(provider: ApiProvider) -> Self {
        match provider {
            ApiProvider::OpenAi => Self::OpenAi,
            ApiProvider::Voyage => Self::Voyage,
            ApiProvider::Gemini => Self::Gemini,
            ApiProvider::OpenAiCompatible => Self::OpenAiCompatible,
        }
    }
}

/// The keychain service of the app's keys; also its Tauri identifier.
pub const SERVICE: &str = "com.kemalozyon.siyak";

/// Longest key accepted. Provider keys are far shorter.
const MAX_KEY_LEN: usize = 1024;

/// Where API keys are kept.
pub trait KeyStore: Send + Sync {
    /// The key of `provider`, if one is saved.
    fn key(&self, provider: ProviderId) -> AppResult<Option<String>>;

    /// Saves (or replaces) the key of `provider`.
    fn set_key(&self, provider: ProviderId, key: &str) -> AppResult<()>;

    /// Removes the key of `provider`; succeeds if none was saved.
    fn delete_key(&self, provider: ProviderId) -> AppResult<()>;

    /// Whether the key of `provider` is saved.
    fn has_key(&self, provider: ProviderId) -> AppResult<bool> {
        Ok(self.key(provider)?.is_some())
    }
}

/// A key store that has no keys and cannot save any (the app before its
/// databases open).
pub struct NoKeyStore;

impl KeyStore for NoKeyStore {
    fn key(&self, _provider: ProviderId) -> AppResult<Option<String>> {
        Ok(None)
    }

    fn set_key(&self, _provider: ProviderId, _key: &str) -> AppResult<()> {
        Err(keychain_error("no keychain is available"))
    }

    fn delete_key(&self, _provider: ProviderId) -> AppResult<()> {
        Ok(())
    }
}

/// Checks a key the user entered and returns it without surrounding spaces.
/// A key goes into an HTTP header, so it must be one line of visible ASCII.
pub fn clean_key(key: &str) -> AppResult<&str> {
    let key = key.trim();
    if key.is_empty() || key.len() > MAX_KEY_LEN || !key.bytes().all(|b| b.is_ascii_graphic()) {
        return Err(AppError::new(
            AppErrorCode::InvalidInput,
            "an API key is one line of letters, digits and symbols",
        ));
    }
    Ok(key)
}

/// The OS keychain, with the keys read so far kept in memory.
#[derive(Default)]
pub struct Keychain {
    /// Keys by provider; `None` means "read, and not saved".
    cache: Mutex<HashMap<ProviderId, Option<String>>>,
}

impl Keychain {
    fn entry(provider: ProviderId) -> AppResult<keyring::Entry> {
        keyring::Entry::new(SERVICE, provider.as_str()).map_err(keychain_error)
    }

    fn cache(&self) -> std::sync::MutexGuard<'_, HashMap<ProviderId, Option<String>>> {
        self.cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

impl KeyStore for Keychain {
    fn key(&self, provider: ProviderId) -> AppResult<Option<String>> {
        if let Some(cached) = self.cache().get(&provider) {
            return Ok(cached.clone());
        }
        let key = match Self::entry(provider)?.get_password() {
            Ok(key) => Some(key),
            Err(keyring::Error::NoEntry) => None,
            Err(err) => return Err(keychain_error(err)),
        };
        self.cache().insert(provider, key.clone());
        Ok(key)
    }

    fn set_key(&self, provider: ProviderId, key: &str) -> AppResult<()> {
        let key = clean_key(key)?;
        Self::entry(provider)?
            .set_password(key)
            .map_err(keychain_error)?;
        self.cache().insert(provider, Some(key.to_owned()));
        Ok(())
    }

    fn delete_key(&self, provider: ProviderId) -> AppResult<()> {
        match Self::entry(provider)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => {}
            Err(err) => return Err(keychain_error(err)),
        }
        self.cache().insert(provider, None);
        Ok(())
    }
}

/// A keychain error. Its message never contains the key: keyring errors name
/// the entry, not its secret.
fn keychain_error(err: impl std::fmt::Display) -> AppError {
    AppError::new(AppErrorCode::KeychainFailed, format!("keychain: {err}"))
}

/// Keys kept in memory, for tests.
#[cfg(test)]
#[derive(Default)]
pub(crate) struct MemoryKeys(pub Mutex<HashMap<ProviderId, String>>);

#[cfg(test)]
impl MemoryKeys {
    /// A store holding `key` for `provider`.
    pub(crate) fn with(provider: impl Into<ProviderId>, key: &str) -> Self {
        let keys = Self::default();
        keys.0
            .lock()
            .unwrap()
            .insert(provider.into(), key.to_owned());
        keys
    }
}

#[cfg(test)]
impl KeyStore for MemoryKeys {
    fn key(&self, provider: ProviderId) -> AppResult<Option<String>> {
        Ok(self.0.lock().unwrap().get(&provider).cloned())
    }

    fn set_key(&self, provider: ProviderId, key: &str) -> AppResult<()> {
        let key = clean_key(key)?;
        self.0.lock().unwrap().insert(provider, key.to_owned());
        Ok(())
    }

    fn delete_key(&self, provider: ProviderId) -> AppResult<()> {
        self.0.lock().unwrap().remove(&provider);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_trimmed_and_kept_to_one_line() {
        assert_eq!(clean_key("  sk-abc_123  ").unwrap(), "sk-abc_123");
        for bad in [
            "",
            "   ",
            "sk abc",
            "sk-abc\nX-Other: 1",
            "anahtarı",
            &"x".repeat(2000),
        ] {
            let err = clean_key(bad).unwrap_err();
            assert_eq!(err.code, AppErrorCode::InvalidInput, "{bad:?}");
        }
    }

    #[test]
    fn memory_keys_save_replace_and_delete() {
        let keys = MemoryKeys::default();
        assert!(!keys.has_key(ProviderId::Voyage).unwrap());
        keys.set_key(ProviderId::Voyage, " pa-1 ").unwrap();
        keys.set_key(ProviderId::Voyage, "pa-2").unwrap();
        assert_eq!(
            keys.key(ProviderId::Voyage).unwrap().as_deref(),
            Some("pa-2")
        );
        keys.delete_key(ProviderId::Voyage).unwrap();
        keys.delete_key(ProviderId::Voyage).unwrap();
        assert!(!keys.has_key(ProviderId::Voyage).unwrap());
    }

    #[test]
    fn embedding_providers_keep_their_keychain_accounts() {
        for provider in ApiProvider::ALL {
            assert_eq!(ProviderId::from(provider).as_str(), provider.as_str());
        }
        for provider in ProviderId::ALL {
            assert_eq!(ProviderId::parse(provider.as_str()), Some(provider));
        }
    }

    #[test]
    fn no_key_store_refuses_to_save() {
        let err = NoKeyStore.set_key(ProviderId::OpenAi, "sk-1").unwrap_err();
        assert_eq!(err.code, AppErrorCode::KeychainFailed);
        assert_eq!(NoKeyStore.key(ProviderId::OpenAi).unwrap(), None);
    }
}
