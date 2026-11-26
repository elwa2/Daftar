//! Opening a company database with the mandatory connection settings.

use std::path::Path;

use rusqlite::Connection;

use crate::error::DbError;
use crate::functions::{self, UserSlot};
use crate::migrate;

/// An open company database and its authenticated-user slot.
pub struct Database {
    pub conn: Connection,
    pub user: UserSlot,
}

impl Database {
    /// Opens (or creates) a company database file and applies migrations.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, DbError> {
        let mut conn = Connection::open(path)?;
        let user = configure(&conn, false)?;
        migrate::migrate(&mut conn)?;
        Ok(Self { conn, user })
    }

    /// Opens a fresh in-memory database. Used by tests and by read-only
    /// tooling; never by the desktop application.
    pub fn open_in_memory() -> Result<Self, DbError> {
        let mut conn = Connection::open_in_memory()?;
        let user = configure(&conn, true)?;
        migrate::migrate(&mut conn)?;
        Ok(Self { conn, user })
    }

    /// Returns a mutable reference to the underlying connection. Prefer
    /// higher-level service methods; this exists for composition in tests and
    /// controlled service internals.
    pub fn conn_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }

    /// Marks the authenticated user for audit attribution.
    pub fn set_user(&self, user_id: Option<i64>) {
        self.user.set(user_id);
    }

    /// Verifies the complete audit hash chain.
    pub fn verify_audit(&self) -> Result<(), DbError> {
        crate::audit::verify_chain(&self.conn)
    }
}

fn configure(connection: &Connection, in_memory: bool) -> Result<UserSlot, DbError> {
    connection.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA trusted_schema = OFF;
         PRAGMA busy_timeout = 5000;
         PRAGMA synchronous = FULL;",
    )?;

    // WAL is persistent in the file header and is meaningless for :memory:.
    if !in_memory {
        let _mode: String =
            connection.query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))?;
    }

    Ok(functions::register(connection)?)
}
