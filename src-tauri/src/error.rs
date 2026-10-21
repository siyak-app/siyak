//! The single error type that crosses the IPC boundary.
//!
//! Rust never produces user-facing prose: every failure carries a stable
//! `snake_case` [`AppErrorCode`] that the frontend maps to a translated message
//! (`errors.<code>` in `src/i18n/*.json`, spec §16.1). The `message` field is a
//! developer diagnostic for logs and bug reports only.

use serde::Serialize;

/// Stable, machine-readable error codes. Renaming a variant is a breaking
/// change for the frontend's i18n keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
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
}

/// Error returned by every command: `{ code, message }` on the frontend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, thiserror::Error)]
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
