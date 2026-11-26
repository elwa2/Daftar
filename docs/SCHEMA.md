# مخطط قاعدة البيانات — Daftar

> حالة الوثيقة: **مُنفَّذة في المرحلة 1** — الملفات الفعلية في
> `crates/db/migrations/`، والوثيقة تشرحها. أي تعديل على مخطط منشور يحتاج هجرة
> جديدة، لا تعديل ملف قائم.
> المخطط لكل شركة في ملف SQLite مستقل. كل المبالغ `INTEGER` بأصغر وحدة.
> كل التواريخ `TEXT` بصيغة `YYYY-MM-DD`، والتوقيتات ISO-8601 UTC.
>
> ترتيب التنفيذ في الهجرة: `currencies` قبل `company_info`، و`cost_centers`
> قبل `journal_lines`، و`fiscal_years`/`fiscal_periods` قبل `journal_entries`.

## 1) إعدادات الاتصال

```sql
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;
PRAGMA synchronous = FULL;
PRAGMA busy_timeout = 5000;
PRAGMA trusted_schema = OFF;
```

`journal_mode` يُضبط مرة عند فتح القاعدة (لا داخل معاملة ترحيل). باقي الإعدادات
عند كل اتصال. `synchronous=FULL` مقصود للدفاتر؛ يمكن تخفيفه لـ NORMAL في
قواعد التقارير فقط بعد قياس.

## 2) الجداول الأساسية

### 2.1 الشركة والإعدادات

```sql
CREATE TABLE company_info (
    id             INTEGER PRIMARY KEY CHECK (id = 1),
    name_ar        TEXT NOT NULL,
    name_en        TEXT,
    tax_id         TEXT,
    address        TEXT,
    phone          TEXT,
    email          TEXT,
    logo_path      TEXT,
    base_currency  TEXT NOT NULL DEFAULT 'EGP',
    created_at     TEXT NOT NULL,
    updated_at     TEXT NOT NULL,
    FOREIGN KEY (base_currency) REFERENCES currencies(code)
);

CREATE TABLE settings (
    key         TEXT PRIMARY KEY,
    value       TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);
```

إعدادات مقترحة: `require_dual_approval`, `rounding_account_id`,
`retained_earnings_account_id`, `idle_timeout_minutes`, `theme`,
`entry_number_prefix`, `auto_backup_enabled`.

### 2.2 العملات وأسعار الصرف

```sql
CREATE TABLE currencies (
    code         TEXT PRIMARY KEY,
    name_ar      TEXT NOT NULL,
    name_en      TEXT,
    minor_units  INTEGER NOT NULL DEFAULT 2 CHECK (minor_units BETWEEN 0 AND 6),
    is_active    INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0,1))
);

CREATE TABLE exchange_rates (
    id             INTEGER PRIMARY KEY,
    rate_date      TEXT NOT NULL CHECK (date(rate_date) = rate_date),
    from_currency  TEXT NOT NULL REFERENCES currencies(code),
    to_currency    TEXT NOT NULL REFERENCES currencies(code),
    rate_scaled    INTEGER NOT NULL CHECK (rate_scaled > 0),
    created_at     TEXT NOT NULL,
    UNIQUE (rate_date, from_currency, to_currency),
    CHECK (from_currency <> to_currency)
);
```

- العملة الأساسية سعرها 1,000,000 دائماً، ولا تحتاج صفاً.
- `rate_scaled` = عدد وحدات العملة `to` لكل وحدة من `from` × 1,000,000.
- بذرة أولية: `EGP` (2)، `USD` (2)، `EUR` (2).

### 2.3 المستخدمون والأدوار

```sql
CREATE TABLE users (
    id                   INTEGER PRIMARY KEY,
    username             TEXT NOT NULL COLLATE NOCASE UNIQUE,
    password_hash        TEXT NOT NULL,
    full_name            TEXT NOT NULL,
    is_active            INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0,1)),
    must_change_password INTEGER NOT NULL DEFAULT 0 CHECK (must_change_password IN (0,1)),
    failed_attempts      INTEGER NOT NULL DEFAULT 0 CHECK (failed_attempts >= 0),
    locked_until         TEXT,
    last_login_at        TEXT,
    created_at           TEXT NOT NULL,
    updated_at           TEXT NOT NULL
);

CREATE TABLE roles (
    id        INTEGER PRIMARY KEY,
    code      TEXT NOT NULL UNIQUE,
    name_ar   TEXT NOT NULL,
    is_system INTEGER NOT NULL DEFAULT 0 CHECK (is_system IN (0,1))
);

CREATE TABLE permissions (
    code    TEXT PRIMARY KEY,
    name_ar TEXT NOT NULL
);

CREATE TABLE role_permissions (
    role_id         INTEGER NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    permission_code TEXT NOT NULL REFERENCES permissions(code),
    PRIMARY KEY (role_id, permission_code)
);

CREATE TABLE user_roles (
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role_id INTEGER NOT NULL REFERENCES roles(id) ON DELETE RESTRICT,
    PRIMARY KEY (user_id, role_id)
);

CREATE TABLE sessions_log (
    id          INTEGER PRIMARY KEY,
    user_id     INTEGER REFERENCES users(id),
    event       TEXT NOT NULL,              -- login_success/login_failed/logout/lock
    at          TEXT NOT NULL,
    success     INTEGER NOT NULL CHECK (success IN (0,1)),
    reason      TEXT,
    app_version TEXT
);
```

الصلاحيات المزروعة: `entries.create`, `entries.post`, `entries.void`,
`accounts.manage`, `reports.view`, `periods.close`, `users.manage`,
`settings.manage`, `backup.manage`. الأدوار: مدير، محاسب، مراجع، مشاهد.

### 2.4 دليل الحسابات

```sql
CREATE TABLE accounts (
    id              INTEGER PRIMARY KEY,
    code            TEXT NOT NULL UNIQUE,
    name_ar         TEXT NOT NULL,
    name_en         TEXT,
    type            TEXT NOT NULL CHECK (type IN
                        ('asset','liability','equity','revenue','expense')),
    normal_balance  TEXT NOT NULL CHECK (normal_balance IN ('debit','credit')),
    parent_id       INTEGER REFERENCES accounts(id) ON DELETE RESTRICT,
    is_postable     INTEGER NOT NULL DEFAULT 0 CHECK (is_postable IN (0,1)),
    is_active       INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0,1)),
    currency        TEXT REFERENCES currencies(code),
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL,
    CHECK (parent_id IS NULL OR parent_id <> id)
);
```

قواعد إضافية (Triggers/Service):

- الأب يجب أن يكون تجميعياً (`is_postable = 0`).
- لا يمكن تحويل حساب له أبناء إلى `is_postable = 1`.
- نوع الحساب ونوع الأب يجب أن يتطابقا.
- لا يُحذف حساب له أبناء أو حركات (FK + Trigger).
- لا يتغير `type` أو `normal_balance` بعد وجود أي حركة.
- منع الدوران في الشجرة (Service + فحص CTE عند الحفظ).

### 2.5 السنوات والفترات

```sql
CREATE TABLE fiscal_years (
    id         INTEGER PRIMARY KEY,
    name       TEXT NOT NULL,
    start_date TEXT NOT NULL CHECK (date(start_date) = start_date),
    end_date   TEXT NOT NULL CHECK (date(end_date) = end_date),
    is_closed  INTEGER NOT NULL DEFAULT 0 CHECK (is_closed IN (0,1)),
    closed_at  TEXT,
    closed_by  INTEGER REFERENCES users(id),
    created_at TEXT NOT NULL,
    CHECK (end_date >= start_date),
    CHECK (is_closed = 0 OR closed_at IS NOT NULL)
);

CREATE TABLE fiscal_periods (
    id             INTEGER PRIMARY KEY,
    fiscal_year_id INTEGER NOT NULL REFERENCES fiscal_years(id) ON DELETE RESTRICT,
    name           TEXT NOT NULL,
    start_date     TEXT NOT NULL CHECK (date(start_date) = start_date),
    end_date       TEXT NOT NULL CHECK (date(end_date) = end_date),
    is_closed      INTEGER NOT NULL DEFAULT 0 CHECK (is_closed IN (0,1)),
    closed_at      TEXT,
    closed_by      INTEGER REFERENCES users(id),
    CHECK (end_date >= start_date),
    CHECK (is_closed = 0 OR closed_at IS NOT NULL),
    UNIQUE (fiscal_year_id, start_date, end_date)
);
```

Triggers: منع تداخل الفترات داخل السنة، ومنع خروج الفترة عن حدود السنة، ومنع
إغلاق سنة ما زالت فتراتها مفتوحة، ومنع إعادة فتح سنة/فترة مغلقة (الإصلاح بقيد
تسوية، لا بإعادة الفتح).

### 2.6 القيود والسطور

```sql
CREATE TABLE journal_entries (
    id                INTEGER PRIMARY KEY,
    fiscal_year_id    INTEGER NOT NULL REFERENCES fiscal_years(id) ON DELETE RESTRICT,
    fiscal_period_id  INTEGER NOT NULL REFERENCES fiscal_periods(id) ON DELETE RESTRICT,
    number            INTEGER,
    entry_date        TEXT NOT NULL CHECK (date(entry_date) = entry_date),
    reference         TEXT,
    description       TEXT NOT NULL CHECK (length(trim(description)) > 0),
    currency          TEXT NOT NULL REFERENCES currencies(code),
    exchange_rate     INTEGER NOT NULL CHECK (exchange_rate > 0),
    status            TEXT NOT NULL DEFAULT 'draft'
                          CHECK (status IN ('draft','posted','void')),
    reversed_entry_id INTEGER REFERENCES journal_entries(id) ON DELETE RESTRICT,
    source_type       TEXT NOT NULL DEFAULT 'manual',
    source_id         TEXT,
    created_by        INTEGER NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    posted_by         INTEGER REFERENCES users(id) ON DELETE RESTRICT,
    posted_at         TEXT,
    created_at        TEXT NOT NULL,
    updated_at        TEXT NOT NULL,
    CHECK (reversed_entry_id IS NULL OR reversed_entry_id <> id),
    CHECK (status <> 'posted' OR
           (number IS NOT NULL AND posted_by IS NOT NULL AND posted_at IS NOT NULL)),
    CHECK (status = 'posted' OR number IS NULL)
);

CREATE UNIQUE INDEX ux_entries_number
    ON journal_entries (fiscal_year_id, number)
    WHERE number IS NOT NULL;

CREATE UNIQUE INDEX ux_entries_reversal
    ON journal_entries (reversed_entry_id)
    WHERE reversed_entry_id IS NOT NULL;
```

```sql
CREATE TABLE journal_lines (
    id              INTEGER PRIMARY KEY,
    entry_id        INTEGER NOT NULL REFERENCES journal_entries(id) ON DELETE CASCADE,
    line_no         INTEGER NOT NULL CHECK (line_no > 0),
    account_id      INTEGER NOT NULL REFERENCES accounts(id) ON DELETE RESTRICT,
    description     TEXT,
    currency        TEXT NOT NULL REFERENCES currencies(code),
    exchange_rate   INTEGER NOT NULL CHECK (exchange_rate > 0),
    debit           INTEGER NOT NULL DEFAULT 0 CHECK (debit >= 0),
    credit          INTEGER NOT NULL DEFAULT 0 CHECK (credit >= 0),
    base_debit      INTEGER NOT NULL DEFAULT 0 CHECK (base_debit >= 0),
    base_credit     INTEGER NOT NULL DEFAULT 0 CHECK (base_credit >= 0),
    cost_center_id  INTEGER REFERENCES cost_centers(id) ON DELETE RESTRICT,
    UNIQUE (entry_id, line_no),
    CHECK ((debit > 0 AND credit = 0) OR (credit > 0 AND debit = 0)),
    CHECK ((base_debit > 0 AND base_credit = 0) OR
           (base_credit > 0 AND base_debit = 0))
);
```

ملاحظات:

- `number` يُخصص عند الترحيل فقط؛ المسودة بلا رقم.
- `fiscal_period_id` مضاف لتسريع الفحص وضمان الاتساق مع التاريخ.
- `reversed_entry_id` على القيد **العكسي** يشير إلى الأصل (اتجاه واحد).
- `source_type` المقترح: `manual`, `reversal`, `treasury_receipt`,
  `treasury_payment`, `expense`, `opening_balance`, `year_close`, `inventory`,
  `payroll`.
- سطر واحد لا يكون مديناً ودائناً معاً، ولا صفراً، ولا سالباً.

### 2.7 عدّاد الترقيم

```sql
CREATE TABLE entry_counters (
    fiscal_year_id INTEGER PRIMARY KEY REFERENCES fiscal_years(id) ON DELETE RESTRICT,
    last_number    INTEGER NOT NULL DEFAULT 0 CHECK (last_number >= 0)
);
```

### 2.8 التدقيق

```sql
CREATE TABLE audit_log (
    id          INTEGER PRIMARY KEY,
    at          TEXT NOT NULL,
    user_id     INTEGER REFERENCES users(id) ON DELETE RESTRICT,
    action      TEXT NOT NULL,          -- insert/update/delete/post/reverse/...
    table_name  TEXT NOT NULL,
    record_id   TEXT NOT NULL,
    old         TEXT,                   -- JSON أو NULL
    new         TEXT,                   -- JSON أو NULL
    prev_hash   TEXT NOT NULL,
    hash        TEXT NOT NULL UNIQUE,
    hash_version INTEGER NOT NULL DEFAULT 1
);
```

- Append-only: Triggers تمنع UPDATE/DELETE على `audit_log`.
- `prev_hash` للصف الأول = 64 صفراً.
- `hash = sha256(prev_hash | at | user_id | action | table_name | record_id | old | new)`،
  حيث الفواصل `|` حرفية والقيم الناقصة تُستبدل بسلسلة فارغة.
- الدالة `audit_hash(prev, at, user_id, action, table, record, old, new)` و`app_user_id()`
  مسجلتان من Rust على كل اتصال، ونفس الدالة تُستخدم في التحقق من السلسلة.
- `audit_hash`: `SQLITE_UTF8 | SQLITE_DETERMINISTIC | SQLITE_INNOCUOUS`.
- `app_user_id`: `SQLITE_UTF8 | SQLITE_INNOCUOUS` (ليست Deterministic لأنها
  تتغير حسب الجلسة). هذا يضمن العمل مع `PRAGMA trusted_schema=OFF`.
- Triggers بعد INSERT/UPDATE/DELETE على الجداول الحساسة، مولَّدة بـ
  `tools/gen_audit_triggers.py`. مثال (مبسّط من الملف المولَّد):

```sql
CREATE TRIGGER trg_accounts_audit_update
AFTER UPDATE ON accounts
BEGIN
    INSERT INTO audit_log
        (at, user_id, action, table_name, record_id, old, new, prev_hash, hash)
    SELECT at, app_user_id(), 'update', 'accounts', CAST(NEW.id AS TEXT),
           old_json, new_json, prev,
           audit_hash(prev, at, app_user_id(), 'update', 'accounts',
                      CAST(NEW.id AS TEXT), old_json, new_json)
    FROM (
        SELECT
            strftime('%Y-%m-%dT%H:%M:%fZ', 'now') AS at,
            COALESCE((SELECT hash FROM audit_log ORDER BY id DESC LIMIT 1),
                     '0000000000000000000000000000000000000000000000000000000000000000') AS prev,
            json_object('id', OLD.id, 'code', OLD.code, 'name_ar', OLD.name_ar,
                        'type', OLD.type, 'normal_balance', OLD.normal_balance,
                        'parent_id', OLD.parent_id, 'is_postable', OLD.is_postable,
                        'is_active', OLD.is_active) AS old_json,
            json_object('id', NEW.id, 'code', NEW.code, 'name_ar', NEW.name_ar,
                        'type', NEW.type, 'normal_balance', NEW.normal_balance,
                        'parent_id', NEW.parent_id, 'is_postable', NEW.is_postable,
                        'is_active', NEW.is_active) AS new_json
    );
END;
```

يُطبَّق نفس النمط على: `company_info`, `settings`, `currencies`, `exchange_rates`,
`accounts`, `cost_centers`, `fiscal_years`, `fiscal_periods`, `journal_entries`,
`journal_lines`, `entry_counters`, `attachments`, `entry_templates`, `users`,
`roles`, `permissions`, `role_permissions`, `user_roles`.

**قواعد التدقيق:**

- Triggers هي المصدر الوحيد لتدقيق تغييرات الجداول. لا تكتب الخدمة سجلاً
  يدوياً لنفس التغيير، لتجنب الازدواج.
- الأحداث غير الجدولية (دخول/خروج، نسخ احتياطي، فتح شركة) تُسجل من الخدمة
  بدالة `db::audit::append_event` واحدة، و`sessions_log` لا يُدقَّق بـ trigger
  لتجنب الازدواج.
- **لا تُسجَّل كلمات المرور أو الأسرار** في `old`/`new`. لجدول `users` يُستبعد
  `password_hash` ويُسجل بدلاً منه `password_changed = true/false`.
- لا تُخزَّن مفاتيح التشفير أو أسرار النسخ في `settings`.

### 2.9 جداول مساندة

```sql
CREATE TABLE cost_centers (
    id         INTEGER PRIMARY KEY,
    code       TEXT NOT NULL UNIQUE,
    name_ar    TEXT NOT NULL,
    parent_id  INTEGER REFERENCES cost_centers(id) ON DELETE RESTRICT,
    is_active  INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0,1))
);

CREATE TABLE attachments (
    id            INTEGER PRIMARY KEY,
    entry_id      INTEGER NOT NULL REFERENCES journal_entries(id) ON DELETE RESTRICT,
    file_name     TEXT NOT NULL,
    relative_path TEXT NOT NULL,
    mime_type     TEXT,
    size_bytes    INTEGER NOT NULL CHECK (size_bytes >= 0),
    sha256        TEXT NOT NULL,
    created_by    INTEGER NOT NULL REFERENCES users(id),
    created_at    TEXT NOT NULL
);

CREATE TABLE entry_templates (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL,
    description TEXT,
    data_json   TEXT NOT NULL,
    created_by  INTEGER NOT NULL REFERENCES users(id),
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);
```

## 3) Triggers الحرجة

### 3.1 منع تعديل/حذف المرحّل

```sql
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
```

### 3.2 منع إنشاء قيد مرحّل مباشرة

```sql
CREATE TRIGGER trg_entries_insert_draft_only
BEFORE INSERT ON journal_entries
WHEN NEW.status <> 'draft'
BEGIN
    SELECT RAISE(ABORT, 'entry_must_start_as_draft');
END;
```

### 3.3 حارس الترحيل

```sql
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
```

### 3.4 انتقالات الحالة

```sql
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
```

### 3.5 حسابات

```sql
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
```

## 4) Views

```sql
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
```

> Views أعلاه شاملة كل الفترات. التقارير المُفلترة تستخدم استعلامات
> parametrized في `crates/reports`، وليس Views مباشرة، حتى لا تختلط فترات.

## 5) الفهارس

```sql
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
CREATE INDEX ix_rates_lookup ON exchange_rates (from_currency, to_currency, rate_date);
```

## 6) جداول الوحدات (مخطط مبدئي، تُنفّذ في مراحلها)

```text
parties(id, code, name_ar, name_en, type customer/supplier/both,
        control_account_id, is_active, ...)
treasury_accounts(id, name, account_id, kind cash/bank, currency, is_active)
treasury_vouchers(id, kind receipt/payment, number, voucher_date, party_id,
                  treasury_account_id, amount, currency, exchange_rate,
                  description, status, journal_entry_id, ...)
expenses(id, number, expense_date, account_id, treasury_account_id, amount,
         currency, exchange_rate, description, journal_entry_id, ...)
inventory_items / inventory_movements / inventory_costs
employees / payroll_runs / payroll_lines
```

كل هذه الوحدات **لا تكتب في `journal_lines` مباشرة**؛ تستدعي Posting Service
الموحد داخل نفس المعاملة.

## 7) مصفوفة الفرض

| القاعدة | Rust | SQLite |
|---|---|---|
| مدين = دائن عند الترحيل | Posting Service | Trigger `trg_entries_post_guard` |
| سطر مدين XOR دائن، غير سالب | `JournalLine::validate` | CHECK |
| المرحّل لا يُعدّل/يُحذف | Service يرفض | Triggers |
| حساب قابل للترحيل ونشط | Service | Trigger |
| فترة/سنة مفتوحة | Service | Trigger |
| ترقيم بلا فجوات | Counter في المعاملة | Unique + Trigger |
| الاعتماد المزدوج | Service | Trigger |
| سلسلة التدقيق | Verifier + كتابة | Triggers + `audit_hash` |
| الأصول = الالتزامات + الملكية + الربح | proptest/تقارير | Views + فحص تقرير |
| رصيد الأستاذ = الميزان | اختبارات | نفس `v_posted_lines` |

## 8) الهجرة والإصدار

- Runner داخلي في `crates/db/src/migrate.rs` (لا `rusqlite_migration`):
  `MIGRATIONS` قائمة `(version, include_str!(...))`، والنسخة في
  `PRAGMA user_version`.
- كل هجرة ثابتة، تعمل مرة واحدة داخل `BEGIN IMMEDIATE`، ولا تُعدّل بعد النشر.
- الترتيب الحالي:
  1. `0001_schema.sql`: الجداول والفهارس والـ views وtriggers السلامة.
  2. `0002_seed.sql`: العملات والصلاحيات والأدوار والإعدادات الافتراضية.
  3. `0003_audit.sql`: triggers التدقيق (مولَّدة بـ `tools/gen_audit_triggers.py`)،
     فلا تُدقَّق بيانات البذرة.
- دليل الحسابات الافتراضي والمدير الأول يُنشآن في Service أول تشغيل (يحتاج
  تجزئة كلمة مرور).
- قبل أي هجرة: نسخة احتياطية تلقائية.
- `PRAGMA integrity_check` بعد الهجرة.
- لا تُشغّل هجرة على قاعدة مفتوحة بمعاملات متداخلة.

## 9) نقاط تحتاج اعتماداً

نُفِّذت المرحلة 1 على الافتراضات التالية، وتبقى معلَّقة على اعتمادك:

1. `posted` نهائي + العكس (لا `posted → void`).
2. `minor_units` من 0 إلى 6 لكل عملة، مع تحويل بين وحدتين مختلفتين.
3. `fiscal_period_id` موجود في `journal_entries` ويُتحقق من مطابقته للتاريخ
   وللسنة عبر trigger.
4. المستخدمون داخل كل شركة.
5. لا إعادة فتح للفترة/السنة بعد الإغلاق (trigger يمنع أي تعديل بعد `is_closed=1`).
