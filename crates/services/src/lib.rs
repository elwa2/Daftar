//! Application services for Daftar.
//!
//! Services sit between the UI/Tauri commands and the database. They receive a
//! validated [`daftar_core::Context`], enforce permissions, call the pure domain
//! rules, and execute SQLite transactions. They must not contain UI code.

pub mod accounts;
pub mod company;
pub mod error;
pub mod journal;
pub mod reports;

pub use error::{ServiceError, ServiceResult};
