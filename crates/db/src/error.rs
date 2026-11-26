use thiserror::Error;

/// Errors raised by the persistence layer.
#[derive(Debug, Error)]
pub enum DbError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("schema version {found} is newer than supported version {supported}")]
    SchemaTooNew { found: u32, supported: u32 },
    #[error("invalid schema version: {0}")]
    InvalidSchemaVersion(i64),
    #[error("audit chain is broken at row {0}")]
    AuditChainBroken(i64),
}
