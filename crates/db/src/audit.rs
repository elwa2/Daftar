//! Audit chain verification and service-level audit events.

use daftar_core::audit::{audit_hash, GENESIS_HASH};
use daftar_core::clock::now_iso8601;
use rusqlite::{params, Connection};

use crate::error::DbError;

/// Recomputes every row hash and checks the links between rows.
pub fn verify_chain(connection: &Connection) -> Result<(), DbError> {
    let mut statement = connection.prepare(
        "SELECT id, at, user_id, action, table_name, record_id, old, new, prev_hash, hash
         FROM audit_log
         ORDER BY id",
    )?;
    let mut rows = statement.query([])?;

    let mut expected_prev = GENESIS_HASH.to_owned();
    while let Some(row) = rows.next()? {
        let id: i64 = row.get(0)?;
        let at: String = row.get(1)?;
        let user_id: Option<i64> = row.get(2)?;
        let action: String = row.get(3)?;
        let table_name: String = row.get(4)?;
        let record_id: String = row.get(5)?;
        let old: Option<String> = row.get(6)?;
        let new: Option<String> = row.get(7)?;
        let stored_prev: String = row.get(8)?;
        let stored_hash: String = row.get(9)?;

        if stored_prev != expected_prev {
            return Err(DbError::AuditChainBroken(id));
        }

        let computed = audit_hash(
            &stored_prev,
            &at,
            user_id,
            &action,
            &table_name,
            &record_id,
            old.as_deref(),
            new.as_deref(),
        );
        if computed != stored_hash {
            return Err(DbError::AuditChainBroken(id));
        }

        expected_prev = stored_hash;
    }

    Ok(())
}

/// Appends a non-table audit event (login, logout, backup, restore...).
///
/// Table changes are audited by triggers and must not call this function, to
/// avoid duplicate rows.
pub fn append_event(
    connection: &Connection,
    user_id: Option<i64>,
    action: &str,
    table_name: &str,
    record_id: &str,
    old: Option<&str>,
    new: Option<&str>,
) -> Result<(), DbError> {
    let at = now_iso8601();
    connection.execute(
        "INSERT INTO audit_log
             (at, user_id, action, table_name, record_id, old, new, prev_hash, hash)
         SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7, prev,
                audit_hash(prev, ?1, ?2, ?3, ?4, ?5, ?6, ?7)
         FROM (
             SELECT COALESCE(
                        (SELECT hash FROM audit_log ORDER BY id DESC LIMIT 1),
                        ?8
                    ) AS prev
         )",
        params![
            at,
            user_id,
            action,
            table_name,
            record_id,
            old,
            new,
            GENESIS_HASH
        ],
    )?;
    Ok(())
}
