//! Audit hash chain.
//!
//! The SQLite triggers call the SQL function `audit_hash(...)`, which is
//! registered from [`audit_hash`] below. The same function is used to verify
//! the chain, so the formula has exactly one implementation.

use sha2::{Digest, Sha256};

/// Hash of the first audit row: 64 hexadecimal zeroes.
pub const GENESIS_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Computes `sha256(prev_hash | at | user_id | action | table | record_id | old | new)`.
///
/// Missing `user_id`, `old` and `new` are represented by an empty string, which
/// matches `COALESCE(..., '')` in SQL.
// The eight parameters mirror the SQL function signature exactly; grouping them
// in a struct would make the triggers' call site less obvious.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn audit_hash(
    prev_hash: &str,
    at: &str,
    user_id: Option<i64>,
    action: &str,
    table_name: &str,
    record_id: &str,
    old: Option<&str>,
    new: Option<&str>,
) -> String {
    let user = match user_id {
        Some(value) => value.to_string(),
        None => String::new(),
    };
    let payload = format!(
        "{prev_hash}|{at}|{user}|{action}|{table_name}|{record_id}|{}|{}",
        old.unwrap_or_default(),
        new.unwrap_or_default()
    );
    to_hex(&Sha256::digest(payload.as_bytes()))
}

fn to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_genesis_row() {
        let hash = audit_hash(
            GENESIS_HASH,
            "2026-01-01T00:00:00Z",
            None,
            "insert",
            "accounts",
            "1",
            None,
            Some("{}"),
        );
        assert_eq!(hash.len(), 64);
        assert!(hash.bytes().all(|b| b.is_ascii_hexdigit()));
    }

    #[test]
    fn hash_changes_with_payload() {
        let first = audit_hash(
            GENESIS_HASH,
            "2026-01-01T00:00:00Z",
            Some(1),
            "update",
            "accounts",
            "1",
            None,
            Some("{}"),
        );
        let second = audit_hash(
            GENESIS_HASH,
            "2026-01-01T00:00:00Z",
            Some(1),
            "update",
            "accounts",
            "1",
            None,
            Some("{\"a\":1}"),
        );
        assert_ne!(first, second);
    }
}
