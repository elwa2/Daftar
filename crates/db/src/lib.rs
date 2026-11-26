//! SQLite persistence layer for Daftar.
//!
//! One SQLite file per company. Integrity rules live in the schema itself
//! (CHECK constraints, foreign keys and triggers) so a bug in a service cannot
//! silently corrupt the ledger.

pub mod audit;
pub mod error;
pub mod functions;
pub mod migrate;
pub mod open;

pub use error::DbError;
pub use functions::UserSlot;
pub use open::Database;
