//! Integration tests for the ledger integrity rules.
//!
//! These tests run the real SQLite schema with `trusted_schema = OFF` and the
//! real Rust functions, so they cover the path the application uses.

use daftar_db::{Database, DbError};
use rusqlite::{params, Connection, TransactionBehavior};

const NOW: &str = "2026-03-01T00:00:00Z";
const POSTED_AT: &str = "2026-03-02T00:00:00Z";

const SETUP_SQL: &str = r#"
INSERT INTO users (id, username, password_hash, full_name, created_at, updated_at)
VALUES
    (1, 'admin', 'hash', 'مدير', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
    (2, 'reviewer', 'hash', 'مراجع', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z');

INSERT INTO fiscal_years (id, name, start_date, end_date, created_at)
VALUES (1, '2026', '2026-01-01', '2026-12-31', '2026-01-01T00:00:00Z');

INSERT INTO fiscal_periods (id, fiscal_year_id, name, start_date, end_date)
VALUES (1, 1, '2026', '2026-01-01', '2026-12-31');

INSERT INTO accounts
    (id, code, name_ar, type, normal_balance, parent_id, is_postable, created_at, updated_at)
VALUES
    (1, '1000', 'نقدية', 'asset', 'debit', NULL, 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
    (2, '4000', 'إيرادات', 'revenue', 'credit', NULL, 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
    (3, '1100', 'عملاء', 'asset', 'debit', NULL, 0, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
    (4, '1110', 'عملاء محليون', 'asset', 'debit', 3, 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z');
"#;

fn setup() -> Database {
    let db = Database::open_in_memory().expect("open in-memory database");
    db.conn.execute_batch(SETUP_SQL).expect("seed test data");
    db
}

fn try_new_entry(
    conn: &Connection,
    date: &str,
    source: &str,
    reversed: Option<i64>,
    created_by: i64,
    lines: &[(i64, i64, i64)],
) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO journal_entries
             (fiscal_year_id, fiscal_period_id, entry_date, description, currency,
              exchange_rate, source_type, reversed_entry_id, created_by, created_at, updated_at)
         VALUES (1, 1, ?1, 'قيد اختبار', 'EGP', 1000000, ?2, ?3, ?4, ?5, ?5)",
        params![date, source, reversed, created_by, NOW],
    )?;
    let entry_id = conn.last_insert_rowid();
    for (index, (account, debit, credit)) in lines.iter().enumerate() {
        let line_no = i64::try_from(index).expect("line index fits") + 1;
        conn.execute(
            "INSERT INTO journal_lines
                 (entry_id, line_no, account_id, currency, exchange_rate,
                  debit, credit, base_debit, base_credit)
             VALUES (?1, ?2, ?3, 'EGP', 1000000, ?4, ?5, ?4, ?5)",
            params![entry_id, line_no, account, debit, credit],
        )?;
    }
    Ok(entry_id)
}

fn new_entry(
    conn: &Connection,
    date: &str,
    source: &str,
    reversed: Option<i64>,
    created_by: i64,
    lines: &[(i64, i64, i64)],
) -> i64 {
    try_new_entry(conn, date, source, reversed, created_by, lines).expect("insert entry")
}

fn post(conn: &mut Connection, entry_id: i64, user_id: i64) -> rusqlite::Result<i64> {
    let transaction = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    transaction.execute(
        "INSERT INTO entry_counters (fiscal_year_id, last_number)
         SELECT fiscal_year_id, 1 FROM journal_entries WHERE id = ?1
         ON CONFLICT(fiscal_year_id) DO UPDATE SET last_number = last_number + 1",
        params![entry_id],
    )?;
    let number: i64 = transaction.query_row(
        "SELECT last_number FROM entry_counters
         WHERE fiscal_year_id = (SELECT fiscal_year_id FROM journal_entries WHERE id = ?1)",
        params![entry_id],
        |row| row.get(0),
    )?;
    transaction.execute(
        "UPDATE journal_entries
         SET status = 'posted', number = ?2, posted_by = ?3, posted_at = ?4, updated_at = ?4
         WHERE id = ?1",
        params![entry_id, number, user_id, POSTED_AT],
    )?;
    transaction.commit()?;
    Ok(number)
}

fn error_text(result: rusqlite::Result<impl std::fmt::Debug>) -> String {
    match result {
        Ok(value) => panic!("expected an error, got {value:?}"),
        Err(error) => error.to_string(),
    }
}

fn assert_error_contains(result: rusqlite::Result<impl std::fmt::Debug>, fragment: &str) {
    let text = error_text(result);
    assert!(
        text.contains(fragment),
        "expected error containing {fragment:?}, got {text}"
    );
}

fn simple_lines() -> [(i64, i64, i64); 2] {
    [(1, 10_000, 0), (2, 0, 10_000)]
}

#[test]
fn balanced_entry_posts_and_audit_chain_is_valid() {
    let mut db = setup();
    db.set_user(Some(1));
    let entry = new_entry(&db.conn, "2026-03-01", "manual", None, 1, &simple_lines());

    assert_eq!(post(&mut db.conn, entry, 1).expect("post"), 1);

    let (status, number): (String, i64) = db
        .conn
        .query_row(
            "SELECT status, number FROM journal_entries WHERE id = ?1",
            params![entry],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("read entry");
    assert_eq!(status, "posted");
    assert_eq!(number, 1);
    db.verify_audit().expect("audit chain");
}

#[test]
fn posted_entry_is_immutable() {
    let mut db = setup();
    let entry = new_entry(&db.conn, "2026-03-01", "manual", None, 1, &simple_lines());
    post(&mut db.conn, entry, 1).expect("post");

    assert_error_contains(
        db.conn.execute(
            "UPDATE journal_entries SET description = 'changed' WHERE id = ?1",
            params![entry],
        ),
        "posted_entry_is_immutable",
    );
    assert_error_contains(
        db.conn
            .execute("DELETE FROM journal_entries WHERE id = ?1", params![entry]),
        "posted_entry_is_immutable",
    );
}

#[test]
fn direct_posted_insert_is_rejected() {
    let db = setup();
    assert_error_contains(
        db.conn.execute(
            "INSERT INTO journal_entries
                 (fiscal_year_id, fiscal_period_id, entry_date, description, currency,
                  exchange_rate, status, created_by, created_at, updated_at)
             VALUES (1, 1, '2026-03-01', 'x', 'EGP', 1000000, 'posted', 1, ?1, ?1)",
            params![NOW],
        ),
        "entry_must_start_as_draft",
    );
}

#[test]
fn unbalanced_posting_rolls_back_the_counter() {
    let mut db = setup();
    let entry = new_entry(
        &db.conn,
        "2026-03-01",
        "manual",
        None,
        1,
        &[(1, 10_000, 0), (2, 0, 9_000)],
    );

    assert_error_contains(post(&mut db.conn, entry, 1), "entry_not_balanced");

    let counter: i64 = db
        .conn
        .query_row(
            "SELECT COALESCE(MAX(last_number), 0) FROM entry_counters",
            [],
            |row| row.get(0),
        )
        .expect("counter");
    assert_eq!(counter, 0);

    let status: String = db
        .conn
        .query_row(
            "SELECT status FROM journal_entries WHERE id = ?1",
            params![entry],
            |row| row.get(0),
        )
        .expect("status");
    assert_eq!(status, "draft");
}

#[test]
fn closed_period_blocks_posting() {
    let mut db = setup();
    db.conn
        .execute(
            "UPDATE fiscal_periods
             SET is_closed = 1, closed_at = ?1, closed_by = 1
             WHERE id = 1",
            params![POSTED_AT],
        )
        .expect("close period");

    let entry = new_entry(&db.conn, "2026-03-01", "manual", None, 1, &simple_lines());
    assert_error_contains(
        post(&mut db.conn, entry, 1),
        "entry_period_closed_or_invalid",
    );
}

#[test]
fn non_postable_account_blocks_posting() {
    let mut db = setup();
    let entry = new_entry(
        &db.conn,
        "2026-03-01",
        "manual",
        None,
        1,
        &[(3, 500, 0), (2, 0, 500)],
    );
    assert_error_contains(
        post(&mut db.conn, entry, 1),
        "entry_has_non_postable_account",
    );
}

#[test]
fn dual_approval_requires_a_different_poster() {
    let mut db = setup();
    db.conn
        .execute(
            "UPDATE settings SET value = '1' WHERE key = 'require_dual_approval'",
            [],
        )
        .expect("enable dual approval");

    let entry = new_entry(&db.conn, "2026-03-01", "manual", None, 1, &simple_lines());
    assert_error_contains(post(&mut db.conn, entry, 1), "dual_approval_required");
    assert_eq!(post(&mut db.conn, entry, 2).expect("reviewer posts"), 1);
}

#[test]
fn numbering_is_sequential_without_gaps() {
    let mut db = setup();
    let mut numbers = Vec::new();
    for _ in 0..3 {
        let entry = new_entry(&db.conn, "2026-03-01", "manual", None, 1, &simple_lines());
        numbers.push(post(&mut db.conn, entry, 1).expect("post"));
    }
    assert_eq!(numbers, vec![1, 2, 3]);
}

#[test]
fn reversal_is_linked_and_unique() {
    let mut db = setup();
    let original = new_entry(&db.conn, "2026-03-01", "manual", None, 1, &simple_lines());
    assert_eq!(post(&mut db.conn, original, 1).expect("post"), 1);

    let reversal = new_entry(
        &db.conn,
        "2026-03-01",
        "reversal",
        Some(original),
        1,
        &[(2, 10_000, 0), (1, 0, 10_000)],
    );
    assert_eq!(post(&mut db.conn, reversal, 1).expect("post reversal"), 2);

    assert_error_contains(
        try_new_entry(
            &db.conn,
            "2026-03-01",
            "reversal",
            Some(original),
            1,
            &[(2, 1, 0), (1, 0, 1)],
        ),
        "UNIQUE constraint failed: journal_entries.reversed_entry_id",
    );

    let draft = new_entry(&db.conn, "2026-03-01", "manual", None, 1, &simple_lines());
    assert_error_contains(
        try_new_entry(
            &db.conn,
            "2026-03-01",
            "reversal",
            Some(draft),
            1,
            &[(2, 1, 0), (1, 0, 1)],
        ),
        "invalid_reversal_target",
    );
}

#[test]
fn entry_outside_its_period_is_rejected() {
    let db = setup();
    assert_error_contains(
        try_new_entry(&db.conn, "2027-01-01", "manual", None, 1, &simple_lines()),
        "entry_period_mismatch",
    );
}

#[test]
fn void_is_terminal() {
    let db = setup();
    let entry = new_entry(&db.conn, "2026-03-01", "manual", None, 1, &simple_lines());
    db.conn
        .execute(
            "UPDATE journal_entries SET status = 'void', updated_at = ?2 WHERE id = ?1",
            params![entry, POSTED_AT],
        )
        .expect("void draft");

    assert_error_contains(
        db.conn.execute(
            "UPDATE journal_entries SET description = 'changed' WHERE id = ?1",
            params![entry],
        ),
        "void_entry_is_immutable",
    );
}

#[test]
fn account_cycle_is_rejected() {
    let db = setup();
    db.conn
        .execute_batch(
            "INSERT INTO accounts
                 (id, code, name_ar, type, normal_balance, is_postable, created_at, updated_at)
             VALUES
                 (10, '2000', 'مجموعة', 'asset', 'debit', 0, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
                 (11, '2100', 'فرعية', 'asset', 'debit', 0, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z');
             UPDATE accounts SET parent_id = 10 WHERE id = 11;",
        )
        .expect("seed tree");

    assert_error_contains(
        db.conn
            .execute("UPDATE accounts SET parent_id = 11 WHERE id = 10", []),
        "account_cycle",
    );
}

#[test]
fn overlapping_periods_are_rejected() {
    let db = setup();
    assert_error_contains(
        db.conn.execute(
            "INSERT INTO fiscal_periods (fiscal_year_id, name, start_date, end_date)
             VALUES (1, 'متوسط', '2026-06-01', '2026-06-30')",
            [],
        ),
        "period_overlap",
    );
}

#[test]
fn year_cannot_close_with_open_periods() {
    let db = setup();
    db.conn
        .execute_batch(
            "INSERT INTO fiscal_years (id, name, start_date, end_date, created_at)
             VALUES (2, '2027', '2027-01-01', '2027-12-31', '2026-01-01T00:00:00Z');
             INSERT INTO fiscal_periods (id, fiscal_year_id, name, start_date, end_date)
             VALUES (2, 2, '2027', '2027-01-01', '2027-12-31');",
        )
        .expect("seed second year");

    assert_error_contains(
        db.conn.execute(
            "UPDATE fiscal_years
             SET is_closed = 1, closed_at = ?1, closed_by = 1
             WHERE id = 2",
            params![POSTED_AT],
        ),
        "year_has_open_periods",
    );
}

#[test]
fn audit_log_is_append_only() {
    let db = setup();
    assert_error_contains(
        db.conn.execute("DELETE FROM audit_log WHERE id = 1", []),
        "audit_log_is_append_only",
    );
    assert_error_contains(
        db.conn
            .execute("UPDATE audit_log SET old = 'x' WHERE id = 1", []),
        "audit_log_is_append_only",
    );
}

#[test]
fn audit_tampering_is_detected() {
    let db = setup();
    db.verify_audit().expect("chain starts valid");

    db.conn
        .execute("DROP TRIGGER trg_audit_log_no_update", [])
        .expect("drop guard for the test");
    db.conn
        .execute("UPDATE audit_log SET old = 'tampered' WHERE id = 1", [])
        .expect("tamper");

    match db.verify_audit() {
        Err(DbError::AuditChainBroken(_)) => {}
        other => panic!("expected a broken chain, got {other:?}"),
    }
}

#[test]
fn service_events_extend_the_chain() {
    let db = setup();
    daftar_db::audit::append_event(
        &db.conn,
        Some(1),
        "login_success",
        "sessions_log",
        "1",
        None,
        Some("{\"app_version\":\"0.1.0\"}"),
    )
    .expect("append event");
    db.verify_audit().expect("chain after event");
}
