//! SQL functions used by the triggers.
//!
//! `trusted_schema = OFF` means SQLite refuses to call a non-innocuous
//! application function from a trigger or a view. Both functions below are
//! therefore registered as `SQLITE_INNOCUOUS`; `audit_hash` is additionally
//! deterministic.

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use daftar_core::audit::audit_hash as compute_audit_hash;
use rusqlite::functions::FunctionFlags;
use rusqlite::Connection;

/// Sentinel stored when no user is authenticated. Real user ids are positive.
const NO_USER: i64 = -1;

/// Per-connection authenticated user, read by the `app_user_id()` SQL function.
#[derive(Debug, Clone)]
pub struct UserSlot {
    inner: Arc<AtomicI64>,
}

impl UserSlot {
    fn new() -> Self {
        Self {
            inner: Arc::new(AtomicI64::new(NO_USER)),
        }
    }

    /// Sets or clears the current user. Every audit trigger fired afterwards
    /// attributes its row to this user.
    pub fn set(&self, user_id: Option<i64>) {
        self.inner
            .store(user_id.unwrap_or(NO_USER), Ordering::Relaxed);
    }

    #[must_use]
    pub fn get(&self) -> Option<i64> {
        let value = self.inner.load(Ordering::Relaxed);
        if value < 0 {
            None
        } else {
            Some(value)
        }
    }
}

/// Registers `app_user_id()` and `audit_hash(...)` on `connection`.
pub fn register(connection: &Connection) -> rusqlite::Result<UserSlot> {
    let slot = UserSlot::new();

    let user_slot = slot.clone();
    connection.create_scalar_function(
        "app_user_id",
        0,
        FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_INNOCUOUS,
        move |_context| {
            let value = user_slot.inner.load(Ordering::Relaxed);
            Ok(if value < 0 { None } else { Some(value) })
        },
    )?;

    connection.create_scalar_function(
        "audit_hash",
        8,
        FunctionFlags::SQLITE_UTF8
            | FunctionFlags::SQLITE_DETERMINISTIC
            | FunctionFlags::SQLITE_INNOCUOUS,
        |context| {
            let prev_hash: String = context.get(0)?;
            let at: String = context.get(1)?;
            let user_id: Option<i64> = context.get(2)?;
            let action: String = context.get(3)?;
            let table_name: String = context.get(4)?;
            let record_id: String = context.get(5)?;
            let old: Option<String> = context.get(6)?;
            let new: Option<String> = context.get(7)?;
            Ok(compute_audit_hash(
                &prev_hash,
                &at,
                user_id,
                &action,
                &table_name,
                &record_id,
                old.as_deref(),
                new.as_deref(),
            ))
        },
    )?;

    Ok(slot)
}
