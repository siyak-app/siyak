//! The single error type that crosses the IPC boundary.
//!
//! Rust never produces user-facing prose: every failure carries a stable
//! `snake_case` [`AppErrorCode`] that the frontend maps to a translated message
//! (`errors.<code>` in `src/i18n/*.json`, spec §16.1). The `message` field is a
//! developer diagnostic for logs and bug reports only.

use serde::Serialize;
use specta::Type;

/// Stable, machine-readable error codes. Renaming a variant is a breaking
/// change for the frontend's i18n keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum AppErrorCode {
    /// A database file could not be opened or configured.
    DbOpenFailed,
    /// Applying a schema migration failed; the database was left unchanged.
    MigrationFailed,
    /// The database was written by a newer app version than this one.
    DbVersionUnsupported,
    /// The bundled seed database needed for first launch is missing.
    SeedMissing,
    /// A file system operation (copy, backup, directory creation) failed.
    IoFailed,
    /// A database query failed.
    QueryFailed,
    /// The requested item does not exist.
    NotFound,
    /// A setting key does not match the allowed key format.
    InvalidSettingKey,
    /// A setting value is not valid JSON.
    InvalidSettingValue,
    /// Updating the Quran text and bundled translations from the newer bundled
    /// `core.db` failed; the previous data is still in use (spec §6.2).
    CoreRefreshFailed,
    /// A network request failed (offline, timeout, connection reset).
    NetworkFailed,
    /// A request or redirect went to a host outside the allow-list (spec §14.1).
    HostNotAllowed,
    /// The server answered a download with an unexpected status or body.
    DownloadFailed,
    /// The pack manifest is not valid JSON or breaks the manifest rules.
    ManifestInvalid,
    /// The pack manifest has a `schema_version` this app does not know.
    ManifestUnsupported,
    /// No manifest could be fetched and none is cached.
    ManifestUnavailable,
    /// The pack type is not supported by this app version.
    PackUnsupported,
    /// The pack needs a newer app version (`min_app_version`).
    PackIncompatible,
    /// Bundled packs cannot be installed or removed; they ship with the app.
    PackBundled,
    /// The pack is being downloaded or installed right now.
    PackBusy,
    /// The downloaded file does not match the manifest's sha256.
    PackHashMismatch,
    /// The pack is larger than the manifest says.
    PackTooLarge,
    /// The pack's structure or `pack_info` is not what its manifest entry describes.
    PackInvalid,
    /// The pack names verses the development data lacks; debug builds only.
    PackNeedsFullData,
    /// The pack was built for a content schema this app does not know.
    PackSchemaUnsupported,
    /// There is not enough free disk space for the download and installation.
    DiskSpaceInsufficient,
    /// The user cancelled the installation.
    InstallCancelled,
    /// The pack's terms must be acknowledged before its first download.
    TermsNotAcknowledged,
    /// Installing this morphology pack replaces the installed one and needs confirmation.
    MorphologyReplaceRequired,
    /// A request names something that cannot exist: an empty note or title, a
    /// reversed verse range, an unknown citation style.
    InvalidInput,
    /// The chosen file is not a Siyak backup.
    BackupInvalid,
    /// The backup was written by a newer app version than this one.
    BackupUnsupported,
    /// Semantic search needs a vector pack with a working embedder (spec §7.4);
    /// `get_semantic_status` says what is missing.
    SemanticUnavailable,
    /// The OS keychain could not read, save or delete an API key (spec §14.2).
    KeychainFailed,
    /// The provider rejected the API key (HTTP 401 or 403).
    ProviderKeyRejected,
    /// The provider limits requests or the key's quota is used up (HTTP 429).
    ProviderRateLimited,
    /// The provider answered with another error or an unexpected response.
    ProviderFailed,
    /// The provider sent nothing back in time (a thinking model under load).
    ProviderTimeout,
    /// The model is overloaded (HTTP 503), even after one retry.
    ProviderOverloaded,
    /// AI features need a provider, a key (or server address) and a model
    /// (spec §8); `get_ai_status` says what is missing.
    AiUnavailable,
}

/// Error returned by every command: `{ code, message }` on the frontend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type, thiserror::Error)]
#[error("{code:?}: {message}")]
pub struct AppError {
    /// Stable code the frontend maps to a translated message.
    pub code: AppErrorCode,
    /// Developer diagnostic in English; never shown to users as-is.
    pub message: String,
}

impl AppError {
    /// Creates an error with the given code and diagnostic message.
    pub fn new(code: AppErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

/// Query errors default to [`AppErrorCode::QueryFailed`] so `?` works inside
/// queries; callers that know better (opening, migrating) map errors explicitly.
impl From<rusqlite::Error> for AppError {
    fn from(err: rusqlite::Error) -> Self {
        let code = match err {
            rusqlite::Error::QueryReturnedNoRows => AppErrorCode::NotFound,
            _ => AppErrorCode::QueryFailed,
        };
        Self::new(code, err.to_string())
    }
}

/// Shorthand for results whose error is [`AppError`].
pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_code_as_snake_case() {
        let err = AppError::new(AppErrorCode::DbOpenFailed, "boom");
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "code": "db_open_failed", "message": "boom" })
        );
    }

    #[test]
    fn no_rows_maps_to_not_found() {
        let err = AppError::from(rusqlite::Error::QueryReturnedNoRows);
        assert_eq!(err.code, AppErrorCode::NotFound);
    }
}
