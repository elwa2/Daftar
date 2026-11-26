-- Daftar company database, schema version 1.
--
-- Every amount is an INTEGER count of the currency's smallest unit.
-- Every date is TEXT in YYYY-MM-DD format; every timestamp is ISO-8601 UTC.
--
-- This migration is immutable once released. Add a new file instead.

CREATE TABLE currencies (
    code         TEXT PRIMARY KEY,
    name_ar      TEXT NOT NULL,
    name_en      TEXT,
    minor_units  INTEGER NOT NULL DEFAULT 2 CHECK (minor_units BETWEEN 0 AND 6),
    is_active    INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1))
);

CREATE TABLE company_info (
    id             INTEGER PRIMARY KEY CHECK (id = 1),
    name_ar        TEXT NOT NULL CHECK (length(trim(name_ar)) > 0),
    name_en        TEXT,
    tax_id         TEXT,
    address        TEXT,
    phone          TEXT,
    email          TEXT,
    logo_path      TEXT,
    base_currency  TEXT NOT NULL DEFAULT 'EGP',
    created_at     TEXT NOT NULL,
    updated_at     TEXT NOT NULL,
    FOREIGN KEY (base_currency) REFERENCES currencies (code)
);

CREATE TABLE settings (
    key         TEXT PRIMARY KEY,
    value       TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE TABLE users (
    id                   INTEGER PRIMARY KEY,
    username             TEXT NOT NULL COLLATE NOCASE UNIQUE,
    password_hash        TEXT NOT NULL,
    full_name            TEXT NOT NULL,
    is_active            INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    must_change_password INTEGER NOT NULL DEFAULT 0 CHECK (must_change_password IN (0, 1)),
    failed_attempts      INTEGER NOT NULL DEFAULT 0 CHECK (failed_attempts >= 0),
    locked_until         TEXT,
    last_login_at        TEXT,
    created_at           TEXT NOT NULL,
    updated_at           TEXT NOT NULL,
    CHECK (length(trim(username)) > 0),
    CHECK (length(trim(full_name)) > 0)
);

CREATE TABLE roles (
    id        INTEGER PRIMARY KEY,
    code      TEXT NOT NULL UNIQUE,
    name_ar   TEXT NOT NULL,
    is_system INTEGER NOT NULL DEFAULT 0 CHECK (is_system IN (0, 1))
);

CREATE TABLE permissions (
    code    TEXT PRIMARY KEY,
    name_ar TEXT NOT NULL
);

CREATE TABLE role_permissions (
    role_id         INTEGER NOT NULL REFERENCES roles (id) ON DELETE CASCADE,
    permission_code TEXT NOT NULL REFERENCES permissions (code),
    PRIMARY KEY (role_id, permission_code)
);

CREATE TABLE user_roles (
    user_id INTEGER NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    role_id INTEGER NOT NULL REFERENCES roles (id) ON DELETE RESTRICT,
    PRIMARY KEY (user_id, role_id)
);

CREATE TABLE sessions_log (
    id          INTEGER PRIMARY KEY,
    user_id     INTEGER REFERENCES users (id),
    event       TEXT NOT NULL,
    at          TEXT NOT NULL,
    success     INTEGER NOT NULL CHECK (success IN (0, 1)),
    reason      TEXT,
    app_version TEXT
);

CREATE TABLE accounts (
    id              INTEGER PRIMARY KEY,
    code            TEXT NOT NULL UNIQUE,
    name_ar         TEXT NOT NULL,
    name_en         TEXT,
    type            TEXT NOT NULL CHECK (type IN
                        ('asset', 'liability', 'equity', 'revenue', 'expense')),
    normal_balance  TEXT NOT NULL CHECK (normal_balance IN ('debit', 'credit')),
    parent_id       INTEGER REFERENCES accounts (id) ON DELETE RESTRICT,
    is_postable     INTEGER NOT NULL DEFAULT 0 CHECK (is_postable IN (0, 1)),
    is_active       INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    currency        TEXT REFERENCES currencies (code),
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL,
    CHECK (parent_id IS NULL OR parent_id <> id),
    CHECK (length(trim(code)) > 0),
    CHECK (length(trim(name_ar)) > 0)
);

CREATE TABLE cost_centers (
    id         INTEGER PRIMARY KEY,
    code       TEXT NOT NULL UNIQUE,
    name_ar    TEXT NOT NULL,
    parent_id  INTEGER REFERENCES cost_centers (id) ON DELETE RESTRICT,
    is_active  INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1))
);

CREATE TABLE fiscal_years (
    id         INTEGER PRIMARY KEY,
    name       TEXT NOT NULL,
    start_date TEXT NOT NULL CHECK (date(start_date) = start_date),
    end_date   TEXT NOT NULL CHECK (date(end_date) = end_date),
    is_closed  INTEGER NOT NULL DEFAULT 0 CHECK (is_closed IN (0, 1)),
    closed_at  TEXT,
    closed_by  INTEGER REFERENCES users (id),
    created_at TEXT NOT NULL,
    CHECK (end_date >= start_date),
    CHECK (is_closed = 0 OR (closed_at IS NOT NULL AND closed_by IS NOT NULL))
);

CREATE TABLE fiscal_periods (
    id             INTEGER PRIMARY KEY,
    fiscal_year_id INTEGER NOT NULL REFERENCES fiscal_years (id) ON DELETE RESTRICT,
    name           TEXT NOT NULL,
    start_date     TEXT NOT NULL CHECK (date(start_date) = start_date),
    end_date       TEXT NOT NULL CHECK (date(end_date) = end_date),
    is_closed      INTEGER NOT NULL DEFAULT 0 CHECK (is_closed IN (0, 1)),
    closed_at      TEXT,
    closed_by      INTEGER REFERENCES users (id),
    CHECK (end_date >= start_date),
    CHECK (is_closed = 0 OR (closed_at IS NOT NULL AND closed_by IS NOT NULL)),
    UNIQUE (fiscal_year_id, start_date, end_date)
);

CREATE TABLE journal_entries (
    id                INTEGER PRIMARY KEY,
    fiscal_year_id    INTEGER NOT NULL REFERENCES fiscal_years (id) ON DELETE RESTRICT,
    fiscal_period_id  INTEGER NOT NULL REFERENCES fiscal_periods (id) ON DELETE RESTRICT,
    number            INTEGER,
    entry_date        TEXT NOT NULL CHECK (date(entry_date) = entry_date),
    reference         TEXT,
    description       TEXT NOT NULL CHECK (length(trim(description)) > 0),
    currency          TEXT NOT NULL REFERENCES currencies (code),
    exchange_rate     INTEGER NOT NULL CHECK (exchange_rate > 0),
    status            TEXT NOT NULL DEFAULT 'draft'
                          CHECK (status IN ('draft', 'posted', 'void')),
    reversed_entry_id INTEGER REFERENCES journal_entries (id) ON DELETE RESTRICT,
    source_type       TEXT NOT NULL DEFAULT 'manual'
                          CHECK (source_type IN
                              ('manual', 'reversal', 'treasury_receipt', 'treasury_payment',
                               'expense', 'opening_balance', 'year_close', 'inventory',
                               'payroll')),
    source_id         TEXT,
    created_by        INTEGER NOT NULL REFERENCES users (id) ON DELETE RESTRICT,
    posted_by         INTEGER REFERENCES users (id) ON DELETE RESTRICT,
    posted_at         TEXT,
    created_at        TEXT NOT NULL,
    updated_at        TEXT NOT NULL,
    CHECK (reversed_entry_id IS NULL OR reversed_entry_id <> id),
    CHECK (reversed_entry_id IS NULL OR source_type = 'reversal'),
    CHECK (status <> 'posted' OR
           (number IS NOT NULL AND posted_by IS NOT NULL AND posted_at IS NOT NULL)),
    CHECK (status = 'posted' OR number IS NULL),
    CHECK ((posted_by IS NULL AND posted_at IS NULL) OR status = 'posted')
);

CREATE UNIQUE INDEX ux_entries_number
    ON journal_entries (fiscal_year_id, number)
    WHERE number IS NOT NULL;

CREATE UNIQUE INDEX ux_entries_reversal
    ON journal_entries (reversed_entry_id)
    WHERE reversed_entry_id IS NOT NULL;

CREATE TABLE journal_lines (
    id              INTEGER PRIMARY KEY,
    entry_id        INTEGER NOT NULL REFERENCES journal_entries (id) ON DELETE CASCADE,
    line_no         INTEGER NOT NULL CHECK (line_no > 0),
    account_id      INTEGER NOT NULL REFERENCES accounts (id) ON DELETE RESTRICT,
    description     TEXT,
    currency        TEXT NOT NULL REFERENCES currencies (code),
    exchange_rate   INTEGER NOT NULL CHECK (exchange_rate > 0),
    debit           INTEGER NOT NULL DEFAULT 0 CHECK (debit >= 0),
    credit          INTEGER NOT NULL DEFAULT 0 CHECK (credit >= 0),
    base_debit      INTEGER NOT NULL DEFAULT 0 CHECK (base_debit >= 0),
    base_credit     INTEGER NOT NULL DEFAULT 0 CHECK (base_credit >= 0),
    cost_center_id  INTEGER REFERENCES cost_centers (id) ON DELETE RESTRICT,
    UNIQUE (entry_id, line_no),
    CHECK ((debit > 0 AND credit = 0) OR (credit > 0 AND debit = 0)),
    CHECK ((base_debit > 0 AND base_credit = 0) OR
           (base_credit > 0 AND base_debit = 0)),
    CHECK ((debit > 0) = (base_debit > 0)),
    CHECK ((credit > 0) = (base_credit > 0))
);

CREATE TABLE entry_counters (
    fiscal_year_id INTEGER PRIMARY KEY REFERENCES fiscal_years (id) ON DELETE RESTRICT,
    last_number    INTEGER NOT NULL DEFAULT 0 CHECK (last_number >= 0)
);

CREATE TABLE attachments (
    id            INTEGER PRIMARY KEY,
    entry_id      INTEGER NOT NULL REFERENCES journal_entries (id) ON DELETE RESTRICT,
    file_name     TEXT NOT NULL,
    relative_path TEXT NOT NULL,
    mime_type     TEXT,
    size_bytes    INTEGER NOT NULL CHECK (size_bytes >= 0),
    sha256        TEXT NOT NULL CHECK (length(sha256) = 64),
    created_by    INTEGER NOT NULL REFERENCES users (id),
    created_at    TEXT NOT NULL
);

CREATE TABLE entry_templates (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL,
    description TEXT,
    data_json   TEXT NOT NULL CHECK (json_valid(data_json)),
    created_by  INTEGER NOT NULL REFERENCES users (id),
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE TABLE audit_log (
    id           INTEGER PRIMARY KEY,
    at           TEXT NOT NULL,
    user_id      INTEGER REFERENCES users (id) ON DELETE RESTRICT,
    action       TEXT NOT NULL,
    table_name   TEXT NOT NULL,
    record_id    TEXT NOT NULL,
    old          TEXT,
    new          TEXT,
    prev_hash    TEXT NOT NULL CHECK (length(prev_hash) = 64),
    hash         TEXT NOT NULL UNIQUE CHECK (length(hash) = 64),
    hash_version INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX ix_entries_date_status ON journal_entries (entry_date, status);
CREATE INDEX ix_entries_year_status ON journal_entries (fiscal_year_id, status);
CREATE INDEX ix_entries_period ON journal_entries (fiscal_period_id);
CREATE INDEX ix_entries_created_by ON journal_entries (created_by);
CREATE INDEX ix_lines_entry ON journal_lines (entry_id);
CREATE INDEX ix_lines_account_entry ON journal_lines (account_id, entry_id);
CREATE INDEX ix_lines_account ON journal_lines (account_id);
CREATE INDEX ix_audit_at ON audit_log (at);
CREATE INDEX ix_audit_table_record ON audit_log (table_name, record_id);
CREATE INDEX ix_sessions_user_at ON sessions_log (user_id, at);

CREATE TABLE exchange_rates (
    id             INTEGER PRIMARY KEY,
    rate_date      TEXT NOT NULL CHECK (date(rate_date) = rate_date),
    from_currency  TEXT NOT NULL REFERENCES currencies (code),
    to_currency    TEXT NOT NULL REFERENCES currencies (code),
    rate_scaled    INTEGER NOT NULL CHECK (rate_scaled > 0),
    created_at     TEXT NOT NULL,
    UNIQUE (rate_date, from_currency, to_currency),
    CHECK (from_currency <> to_currency)
);

CREATE INDEX ix_rates_lookup ON exchange_rates (from_currency, to_currency, rate_date);

CREATE VIEW v_posted_entries AS
SELECT * FROM journal_entries WHERE status = 'posted';

CREATE VIEW v_posted_lines AS
SELECT
    l.id, l.entry_id, l.line_no, l.account_id, l.description,
    l.currency, l.exchange_rate, l.debit, l.credit,
    l.base_debit, l.base_credit, l.cost_center_id,
    e.fiscal_year_id, e.fiscal_period_id, e.number, e.entry_date,
    e.reference, e.source_type, e.source_id,
    e.description AS entry_description, e.created_by, e.posted_by
FROM journal_lines l
JOIN journal_entries e ON e.id = l.entry_id
WHERE e.status = 'posted';

CREATE VIEW v_ledger AS
SELECT
    l.*,
    SUM(l.base_debit - l.base_credit) OVER (
        PARTITION BY l.account_id
        ORDER BY l.entry_date, l.entry_id, l.line_no
        ROWS UNBOUNDED PRECEDING
    ) AS running_base_balance
FROM v_posted_lines l;

CREATE VIEW v_trial_balance AS
SELECT
    a.id AS account_id, a.code, a.name_ar, a.type, a.normal_balance,
    COALESCE(SUM(l.base_debit), 0) AS debit,
    COALESCE(SUM(l.base_credit), 0) AS credit,
    COALESCE(SUM(l.base_debit), 0) - COALESCE(SUM(l.base_credit), 0) AS net_debit
FROM accounts a
LEFT JOIN v_posted_lines l ON l.account_id = a.id
GROUP BY a.id;

CREATE VIEW v_income_statement AS
SELECT
    a.id AS account_id, a.code, a.name_ar, a.type,
    CASE
        WHEN a.type = 'revenue'
            THEN COALESCE(SUM(l.base_credit), 0) - COALESCE(SUM(l.base_debit), 0)
        WHEN a.type = 'expense'
            THEN COALESCE(SUM(l.base_debit), 0) - COALESCE(SUM(l.base_credit), 0)
        ELSE 0
    END AS amount
FROM accounts a
LEFT JOIN v_posted_lines l ON l.account_id = a.id
WHERE a.type IN ('revenue', 'expense')
GROUP BY a.id;

CREATE VIEW v_balance_sheet AS
SELECT 'asset' AS section, a.id AS account_id, a.code, a.name_ar,
       COALESCE(SUM(l.base_debit), 0) - COALESCE(SUM(l.base_credit), 0) AS amount
FROM accounts a
LEFT JOIN v_posted_lines l ON l.account_id = a.id
WHERE a.type = 'asset'
GROUP BY a.id
UNION ALL
SELECT 'liability', a.id, a.code, a.name_ar,
       COALESCE(SUM(l.base_credit), 0) - COALESCE(SUM(l.base_debit), 0)
FROM accounts a
LEFT JOIN v_posted_lines l ON l.account_id = a.id
WHERE a.type = 'liability'
GROUP BY a.id
UNION ALL
SELECT 'equity', a.id, a.code, a.name_ar,
       COALESCE(SUM(l.base_credit), 0) - COALESCE(SUM(l.base_debit), 0)
FROM accounts a
LEFT JOIN v_posted_lines l ON l.account_id = a.id
WHERE a.type = 'equity'
GROUP BY a.id
UNION ALL
SELECT 'net_income', NULL, NULL, 'صافي ربح الفترة',
       COALESCE(SUM(CASE WHEN type = 'revenue' THEN amount ELSE -amount END), 0)
FROM v_income_statement;

-- ---------------------------------------------------------------------------
-- Integrity triggers: journal entries
-- ---------------------------------------------------------------------------

CREATE TRIGGER trg_entries_insert_draft_only
BEFORE INSERT ON journal_entries
WHEN NEW.status <> 'draft'
BEGIN
    SELECT RAISE(ABORT, 'entry_must_start_as_draft');
END;

CREATE TRIGGER trg_entries_posted_no_update
BEFORE UPDATE ON journal_entries
WHEN OLD.status = 'posted'
BEGIN
    SELECT RAISE(ABORT, 'posted_entry_is_immutable');
END;

CREATE TRIGGER trg_entries_posted_no_delete
BEFORE DELETE ON journal_entries
WHEN OLD.status = 'posted'
BEGIN
    SELECT RAISE(ABORT, 'posted_entry_is_immutable');
END;

CREATE TRIGGER trg_entries_void_no_delete
BEFORE DELETE ON journal_entries
WHEN OLD.status = 'void'
BEGIN
    SELECT RAISE(ABORT, 'void_entry_is_immutable');
END;

CREATE TRIGGER trg_entries_status_transition
BEFORE UPDATE OF status ON journal_entries
WHEN OLD.status <> NEW.status
 AND NOT (OLD.status = 'draft' AND NEW.status IN ('posted', 'void'))
BEGIN
    SELECT RAISE(ABORT, 'invalid_entry_status_transition');
END;

CREATE TRIGGER trg_entries_void_terminal
BEFORE UPDATE ON journal_entries
WHEN OLD.status = 'void'
BEGIN
    SELECT RAISE(ABORT, 'void_entry_is_immutable');
END;

CREATE TRIGGER trg_entries_reversal_link_immutable
BEFORE UPDATE OF reversed_entry_id ON journal_entries
WHEN NEW.reversed_entry_id IS NOT OLD.reversed_entry_id
BEGIN
    SELECT RAISE(ABORT, 'reversal_link_is_immutable');
END;

CREATE TRIGGER trg_entries_reversal_valid_insert
BEFORE INSERT ON journal_entries
WHEN NEW.reversed_entry_id IS NOT NULL
 AND (
     NEW.source_type <> 'reversal'
     OR (SELECT status FROM journal_entries WHERE id = NEW.reversed_entry_id) <> 'posted'
 )
BEGIN
    SELECT RAISE(ABORT, 'invalid_reversal_target');
END;

CREATE TRIGGER trg_entries_period_match_insert
BEFORE INSERT ON journal_entries
WHEN NOT EXISTS (
    SELECT 1
    FROM fiscal_periods p
    WHERE p.id = NEW.fiscal_period_id
      AND p.fiscal_year_id = NEW.fiscal_year_id
      AND NEW.entry_date BETWEEN p.start_date AND p.end_date
)
BEGIN
    SELECT RAISE(ABORT, 'entry_period_mismatch');
END;

CREATE TRIGGER trg_entries_period_match_update
BEFORE UPDATE OF entry_date, fiscal_period_id, fiscal_year_id ON journal_entries
WHEN NOT EXISTS (
    SELECT 1
    FROM fiscal_periods p
    WHERE p.id = NEW.fiscal_period_id
      AND p.fiscal_year_id = NEW.fiscal_year_id
      AND NEW.entry_date BETWEEN p.start_date AND p.end_date
)
BEGIN
    SELECT RAISE(ABORT, 'entry_period_mismatch');
END;

CREATE TRIGGER trg_entries_post_guard
BEFORE UPDATE OF status ON journal_entries
WHEN NEW.status = 'posted' AND OLD.status = 'draft'
BEGIN
    SELECT CASE
        WHEN (SELECT COUNT(*) FROM journal_lines WHERE entry_id = NEW.id) < 2
            THEN RAISE(ABORT, 'entry_too_few_lines')
        WHEN (SELECT COALESCE(SUM(base_debit), 0) FROM journal_lines
              WHERE entry_id = NEW.id)
           <> (SELECT COALESCE(SUM(base_credit), 0) FROM journal_lines
               WHERE entry_id = NEW.id)
            THEN RAISE(ABORT, 'entry_not_balanced')
        WHEN EXISTS (
            SELECT 1
            FROM fiscal_periods p
            JOIN fiscal_years y ON y.id = p.fiscal_year_id
            WHERE p.id = NEW.fiscal_period_id
              AND (p.is_closed = 1 OR y.is_closed = 1
                   OR p.fiscal_year_id <> NEW.fiscal_year_id
                   OR NEW.entry_date < p.start_date
                   OR NEW.entry_date > p.end_date)
        )
            THEN RAISE(ABORT, 'entry_period_closed_or_invalid')
        WHEN EXISTS (
            SELECT 1
            FROM journal_lines l
            JOIN accounts a ON a.id = l.account_id
            WHERE l.entry_id = NEW.id
              AND (a.is_postable = 0 OR a.is_active = 0)
        )
            THEN RAISE(ABORT, 'entry_has_non_postable_account')
        WHEN NEW.number IS NULL
          OR NOT EXISTS (
              SELECT 1 FROM entry_counters
              WHERE fiscal_year_id = NEW.fiscal_year_id
                AND last_number = NEW.number
          )
            THEN RAISE(ABORT, 'entry_number_not_next')
        WHEN NEW.posted_by IS NULL OR NEW.posted_at IS NULL
            THEN RAISE(ABORT, 'entry_post_metadata_missing')
        WHEN (SELECT value FROM settings WHERE key = 'require_dual_approval') = '1'
          AND NEW.posted_by = NEW.created_by
            THEN RAISE(ABORT, 'dual_approval_required')
    END;
END;

CREATE TRIGGER trg_lines_posted_no_insert
BEFORE INSERT ON journal_lines
WHEN (SELECT status FROM journal_entries WHERE id = NEW.entry_id) <> 'draft'
BEGIN
    SELECT RAISE(ABORT, 'posted_entry_lines_are_immutable');
END;

CREATE TRIGGER trg_lines_posted_no_update
BEFORE UPDATE ON journal_lines
WHEN (SELECT status FROM journal_entries WHERE id = OLD.entry_id) <> 'draft'
  OR (SELECT status FROM journal_entries WHERE id = NEW.entry_id) <> 'draft'
BEGIN
    SELECT RAISE(ABORT, 'posted_entry_lines_are_immutable');
END;

CREATE TRIGGER trg_lines_posted_no_delete
BEFORE DELETE ON journal_lines
WHEN (SELECT status FROM journal_entries WHERE id = OLD.entry_id) <> 'draft'
BEGIN
    SELECT RAISE(ABORT, 'posted_entry_lines_are_immutable');
END;

-- ---------------------------------------------------------------------------
-- Integrity triggers: chart of accounts
-- ---------------------------------------------------------------------------

CREATE TRIGGER trg_accounts_parent_not_postable
BEFORE INSERT ON accounts
WHEN NEW.parent_id IS NOT NULL
 AND (SELECT is_postable FROM accounts WHERE id = NEW.parent_id) = 1
BEGIN
    SELECT RAISE(ABORT, 'parent_account_must_not_be_postable');
END;

CREATE TRIGGER trg_accounts_parent_not_postable_update
BEFORE UPDATE OF parent_id ON accounts
WHEN NEW.parent_id IS NOT NULL
 AND (SELECT is_postable FROM accounts WHERE id = NEW.parent_id) = 1
BEGIN
    SELECT RAISE(ABORT, 'parent_account_must_not_be_postable');
END;

CREATE TRIGGER trg_accounts_type_matches_parent
BEFORE INSERT ON accounts
WHEN NEW.parent_id IS NOT NULL
 AND (SELECT type FROM accounts WHERE id = NEW.parent_id) <> NEW.type
BEGIN
    SELECT RAISE(ABORT, 'account_type_must_match_parent');
END;

CREATE TRIGGER trg_accounts_type_matches_parent_update
BEFORE UPDATE OF parent_id, type ON accounts
WHEN NEW.parent_id IS NOT NULL
 AND (SELECT type FROM accounts WHERE id = NEW.parent_id) <> NEW.type
BEGIN
    SELECT RAISE(ABORT, 'account_type_must_match_parent');
END;

CREATE TRIGGER trg_accounts_children_type
BEFORE UPDATE OF type ON accounts
WHEN EXISTS (
    SELECT 1 FROM accounts
    WHERE parent_id = OLD.id AND type <> NEW.type
)
BEGIN
    SELECT RAISE(ABORT, 'account_type_must_match_children');
END;

CREATE TRIGGER trg_accounts_no_postable_with_children
BEFORE UPDATE OF is_postable ON accounts
WHEN NEW.is_postable = 1
 AND EXISTS (SELECT 1 FROM accounts WHERE parent_id = NEW.id)
BEGIN
    SELECT RAISE(ABORT, 'postable_account_has_children');
END;

CREATE TRIGGER trg_accounts_type_immutable_after_lines
BEFORE UPDATE OF type, normal_balance ON accounts
WHEN EXISTS (SELECT 1 FROM journal_lines WHERE account_id = OLD.id)
 AND (NEW.type <> OLD.type OR NEW.normal_balance <> OLD.normal_balance)
BEGIN
    SELECT RAISE(ABORT, 'account_type_locked_after_lines');
END;

CREATE TRIGGER trg_accounts_no_cycle
BEFORE UPDATE OF parent_id ON accounts
WHEN NEW.parent_id IS NOT NULL
 AND EXISTS (
     WITH RECURSIVE ancestors (id) AS (
         SELECT NEW.parent_id
         UNION ALL
         SELECT a.parent_id
         FROM accounts a
         JOIN ancestors ON a.id = ancestors.id
         WHERE a.parent_id IS NOT NULL
     )
     SELECT 1 FROM ancestors WHERE id = NEW.id
 )
BEGIN
    SELECT RAISE(ABORT, 'account_cycle');
END;

-- ---------------------------------------------------------------------------
-- Integrity triggers: fiscal years and periods
-- ---------------------------------------------------------------------------

CREATE TRIGGER trg_periods_within_year_insert
BEFORE INSERT ON fiscal_periods
WHEN NOT EXISTS (
    SELECT 1 FROM fiscal_years y
    WHERE y.id = NEW.fiscal_year_id
      AND NEW.start_date >= y.start_date
      AND NEW.end_date <= y.end_date
)
BEGIN
    SELECT RAISE(ABORT, 'period_outside_year');
END;

CREATE TRIGGER trg_periods_within_year_update
BEFORE UPDATE OF fiscal_year_id, start_date, end_date ON fiscal_periods
WHEN NOT EXISTS (
    SELECT 1 FROM fiscal_years y
    WHERE y.id = NEW.fiscal_year_id
      AND NEW.start_date >= y.start_date
      AND NEW.end_date <= y.end_date
)
BEGIN
    SELECT RAISE(ABORT, 'period_outside_year');
END;

CREATE TRIGGER trg_periods_no_overlap_insert
BEFORE INSERT ON fiscal_periods
WHEN EXISTS (
    SELECT 1 FROM fiscal_periods p
    WHERE p.fiscal_year_id = NEW.fiscal_year_id
      AND p.id IS NOT NEW.id
      AND NEW.start_date <= p.end_date
      AND NEW.end_date >= p.start_date
)
BEGIN
    SELECT RAISE(ABORT, 'period_overlap');
END;

CREATE TRIGGER trg_periods_no_overlap_update
BEFORE UPDATE OF fiscal_year_id, start_date, end_date ON fiscal_periods
WHEN EXISTS (
    SELECT 1 FROM fiscal_periods p
    WHERE p.fiscal_year_id = NEW.fiscal_year_id
      AND p.id IS NOT NEW.id
      AND NEW.start_date <= p.end_date
      AND NEW.end_date >= p.start_date
)
BEGIN
    SELECT RAISE(ABORT, 'period_overlap');
END;

CREATE TRIGGER trg_periods_dates_locked_with_entries
BEFORE UPDATE OF fiscal_year_id, start_date, end_date ON fiscal_periods
WHEN (NEW.fiscal_year_id <> OLD.fiscal_year_id
      OR NEW.start_date <> OLD.start_date
      OR NEW.end_date <> OLD.end_date)
 AND EXISTS (SELECT 1 FROM journal_entries WHERE fiscal_period_id = OLD.id)
BEGIN
    SELECT RAISE(ABORT, 'period_has_entries');
END;

CREATE TRIGGER trg_periods_closed_immutable
BEFORE UPDATE ON fiscal_periods
WHEN OLD.is_closed = 1
BEGIN
    SELECT RAISE(ABORT, 'closed_period_is_immutable');
END;

CREATE TRIGGER trg_years_closed_immutable
BEFORE UPDATE ON fiscal_years
WHEN OLD.is_closed = 1
BEGIN
    SELECT RAISE(ABORT, 'closed_year_is_immutable');
END;

CREATE TRIGGER trg_years_close_requires_closed_periods
BEFORE UPDATE OF is_closed ON fiscal_years
WHEN NEW.is_closed = 1 AND OLD.is_closed = 0
 AND EXISTS (
     SELECT 1 FROM fiscal_periods
     WHERE fiscal_year_id = OLD.id AND is_closed = 0
 )
BEGIN
    SELECT RAISE(ABORT, 'year_has_open_periods');
END;

CREATE TRIGGER trg_years_dates_keep_periods
BEFORE UPDATE OF start_date, end_date ON fiscal_years
WHEN EXISTS (
    SELECT 1 FROM fiscal_periods p
    WHERE p.fiscal_year_id = OLD.id
      AND (p.start_date < NEW.start_date OR p.end_date > NEW.end_date)
)
BEGIN
    SELECT RAISE(ABORT, 'year_bounds_exclude_periods');
END;
