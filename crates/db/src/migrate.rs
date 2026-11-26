//! Small, explicit migration runner.
//!
//! The project originally planned to use `rusqlite_migration`, but a custom
//! runner keeps every migration a static SQL file with no extra dependency,
//! and it makes the version check and transaction boundary obvious. Each
//! migration runs once, inside `BEGIN IMMEDIATE`, and bumps `user_version`.

use rusqlite::{Connection, TransactionBehavior};

use crate::error::DbError;

/// Version written by the last migration.
pub const LATEST_VERSION: u32 = 3;

const MIGRATIONS: &[(u32, &str)] = &[
    (1, include_str!("../migrations/0001_schema.sql")),
    (2, include_str!("../migrations/0002_seed.sql")),
    (3, include_str!("../migrations/0003_audit.sql")),
];

/// Applies all pending migrations and returns the resulting schema version.
pub fn migrate(connection: &mut Connection) -> Result<u32, DbError> {
    let current_raw: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    let current =
        u32::try_from(current_raw).map_err(|_| DbError::InvalidSchemaVersion(current_raw))?;

    if current > LATEST_VERSION {
        return Err(DbError::SchemaTooNew {
            found: current,
            supported: LATEST_VERSION,
        });
    }

    for (version, sql) in MIGRATIONS {
        if *version <= current {
            continue;
        }
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        transaction.execute_batch(sql)?;
        transaction.pragma_update(None, "user_version", i64::from(*version))?;
        transaction.commit()?;
    }

    Ok(LATEST_VERSION)
}
