//! Pure domain layer for Daftar.
//!
//! This crate must stay independent from Tauri, SQLite and the operating
//! system. It owns the accounting rules that can be expressed without a
//! database: integer money, exchange rates, dates, entry validation, the
//! permission context and the audit hash formula.

pub mod audit;
pub mod clock;
pub mod context;
pub mod domain;
pub mod error;
pub mod money;

pub use context::{CompanyId, Context};
pub use domain::{AccountType, Date, EntryStatus, JournalEntry, JournalLine, NormalBalance};
pub use error::CoreError;
pub use money::{ExchangeRate, Money, RATE_SCALE};
