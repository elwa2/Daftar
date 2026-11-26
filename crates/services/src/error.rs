use daftar_core::CoreError;
use daftar_db::DbError;
use thiserror::Error;

/// Result type returned by service operations.
pub type ServiceResult<T> = Result<T, ServiceError>;

/// Errors raised by the application service layer.
#[derive(Debug, Error)]
pub enum ServiceError {
    #[error(transparent)]
    Core(#[from] CoreError),
    #[error(transparent)]
    Db(#[from] DbError),
    #[error("{0}")]
    Validation(&'static str),
    #[error("{0}")]
    NotFound(&'static str),
    #[error("{0}")]
    Conflict(&'static str),
}

impl ServiceError {
    /// Stable machine code.
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Core(err) => err.code(),
            Self::Db(DbError::Sqlite(_)) => "db_error",
            Self::Db(DbError::SchemaTooNew { .. }) => "schema_too_new",
            Self::Db(DbError::InvalidSchemaVersion(_)) => "invalid_schema_version",
            Self::Db(DbError::AuditChainBroken(_)) => "audit_chain_broken",
            Self::Validation(_) => "validation_error",
            Self::NotFound(_) => "not_found",
            Self::Conflict(_) => "conflict",
        }
    }

    /// User-facing Arabic message.
    pub fn message_ar(&self) -> String {
        match self {
            Self::Core(err) => err.message_ar().to_owned(),
            Self::Db(DbError::Sqlite(_)) => "حدث خطأ في قاعدة البيانات".to_owned(),
            Self::Db(DbError::SchemaTooNew { .. }) => {
                "إصدار قاعدة البيانات أحدث من إصدار التطبيق".to_owned()
            }
            Self::Db(DbError::InvalidSchemaVersion(_)) => {
                "إصدار قاعدة البيانات غير صالح".to_owned()
            }
            Self::Db(DbError::AuditChainBroken(_)) => "تم اكتشاف تلف في سلسلة التدقيق".to_owned(),
            Self::Validation(msg) | Self::NotFound(msg) | Self::Conflict(msg) => (*msg).to_owned(),
        }
    }
}
