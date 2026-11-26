#!/usr/bin/env python3
"""Generates crates/db/migrations/0003_audit.sql.

Run from the repository root:

    python tools/gen_audit_triggers.py

The generated file is committed. Keeping the generator next to the schema makes
it cheap to add a column or a table without hand-writing 18 near-identical
trigger triplets.
"""

from pathlib import Path

GENESIS = "0" * 64
OUTPUT = Path("crates/db/migrations/0003_audit.sql")

# name, audited columns, record id expression for NEW, record id for OLD
TABLES = [
    (
        "company_info",
        ["id", "name_ar", "name_en", "tax_id", "address", "phone", "email",
         "logo_path", "base_currency", "created_at", "updated_at"],
        "CAST(NEW.id AS TEXT)",
        "CAST(OLD.id AS TEXT)",
    ),
    (
        "settings",
        ["key", "value", "updated_at"],
        "NEW.key",
        "OLD.key",
    ),
    (
        "currencies",
        ["code", "name_ar", "name_en", "minor_units", "is_active"],
        "NEW.code",
        "OLD.code",
    ),
    (
        "exchange_rates",
        ["id", "rate_date", "from_currency", "to_currency", "rate_scaled",
         "created_at"],
        "CAST(NEW.id AS TEXT)",
        "CAST(OLD.id AS TEXT)",
    ),
    (
        "accounts",
        ["id", "code", "name_ar", "name_en", "type", "normal_balance",
         "parent_id", "is_postable", "is_active", "currency", "created_at",
         "updated_at"],
        "CAST(NEW.id AS TEXT)",
        "CAST(OLD.id AS TEXT)",
    ),
    (
        "cost_centers",
        ["id", "code", "name_ar", "parent_id", "is_active"],
        "CAST(NEW.id AS TEXT)",
        "CAST(OLD.id AS TEXT)",
    ),
    (
        "fiscal_years",
        ["id", "name", "start_date", "end_date", "is_closed", "closed_at",
         "closed_by", "created_at"],
        "CAST(NEW.id AS TEXT)",
        "CAST(OLD.id AS TEXT)",
    ),
    (
        "fiscal_periods",
        ["id", "fiscal_year_id", "name", "start_date", "end_date", "is_closed",
         "closed_at", "closed_by"],
        "CAST(NEW.id AS TEXT)",
        "CAST(OLD.id AS TEXT)",
    ),
    (
        "journal_entries",
        ["id", "fiscal_year_id", "fiscal_period_id", "number", "entry_date",
         "reference", "description", "currency", "exchange_rate", "status",
         "reversed_entry_id", "source_type", "source_id", "created_by",
         "posted_by", "posted_at", "created_at", "updated_at"],
        "CAST(NEW.id AS TEXT)",
        "CAST(OLD.id AS TEXT)",
    ),
    (
        "journal_lines",
        ["id", "entry_id", "line_no", "account_id", "description", "currency",
         "exchange_rate", "debit", "credit", "base_debit", "base_credit",
         "cost_center_id"],
        "CAST(NEW.id AS TEXT)",
        "CAST(OLD.id AS TEXT)",
    ),
    (
        "entry_counters",
        ["fiscal_year_id", "last_number"],
        "CAST(NEW.fiscal_year_id AS TEXT)",
        "CAST(OLD.fiscal_year_id AS TEXT)",
    ),
    (
        "attachments",
        ["id", "entry_id", "file_name", "relative_path", "mime_type",
         "size_bytes", "sha256", "created_by", "created_at"],
        "CAST(NEW.id AS TEXT)",
        "CAST(OLD.id AS TEXT)",
    ),
    (
        "entry_templates",
        ["id", "name", "description", "data_json", "created_by", "created_at",
         "updated_at"],
        "CAST(NEW.id AS TEXT)",
        "CAST(OLD.id AS TEXT)",
    ),
    (
        "roles",
        ["id", "code", "name_ar", "is_system"],
        "CAST(NEW.id AS TEXT)",
        "CAST(OLD.id AS TEXT)",
    ),
    (
        "permissions",
        ["code", "name_ar"],
        "NEW.code",
        "OLD.code",
    ),
    (
        "role_permissions",
        ["role_id", "permission_code"],
        "CAST(NEW.role_id AS TEXT) || ':' || NEW.permission_code",
        "CAST(OLD.role_id AS TEXT) || ':' || OLD.permission_code",
    ),
    (
        "user_roles",
        ["user_id", "role_id"],
        "CAST(NEW.user_id AS TEXT) || ':' || CAST(NEW.role_id AS TEXT)",
        "CAST(OLD.user_id AS TEXT) || ':' || CAST(OLD.role_id AS TEXT)",
    ),
]

USER_COLUMNS = [
    "id", "username", "full_name", "is_active", "must_change_password",
    "failed_attempts", "locked_until", "last_login_at", "created_at",
    "updated_at",
]


def json_object(columns, side, extra=None):
    parts = [f"'{column}', {side}.{column}" for column in columns]
    if extra is not None:
        parts.append(extra)
    return "json_object(" + ", ".join(parts) + ")"


def trigger(table, action, record, json_expression, old_expression, new_expression):
    old_sql = "NULL" if old_expression is None else old_expression
    new_sql = "NULL" if new_expression is None else new_expression
    return f"""CREATE TRIGGER trg_{table}_audit_{action}
AFTER {action.upper()} ON {table}
BEGIN
    INSERT INTO audit_log
        (at, user_id, action, table_name, record_id, old, new, prev_hash, hash)
    SELECT at, app_user_id(), '{action}', '{table}', {record}, {old_sql}, {new_sql}, prev,
           audit_hash(prev, at, app_user_id(), '{action}', '{table}', {record},
                      {old_sql}, {new_sql})
    FROM (
        SELECT strftime('%Y-%m-%dT%H:%M:%fZ', 'now') AS at,
               COALESCE(
                   (SELECT hash FROM audit_log ORDER BY id DESC LIMIT 1),
                   '{GENESIS}'
               ) AS prev,
               {json_expression}
    );
END;
"""


def build() -> str:
    chunks = [
        "-- Generated by tools/gen_audit_triggers.py. Do not edit by hand.",
        "-- Schema version 3: audit triggers and append-only protection.",
        "",
    ]

    for table, columns, record_new, record_old in TABLES:
        chunks.append(trigger(
            table,
            "insert",
            record_new,
            json_object(columns, "NEW") + " AS new_json",
            None,
            "new_json",
        ))
        chunks.append(trigger(
            table,
            "update",
            record_new,
            json_object(columns, "OLD") + " AS old_json, "
            + json_object(columns, "NEW") + " AS new_json",
            "old_json",
            "new_json",
        ))
        chunks.append(trigger(
            table,
            "delete",
            record_old,
            json_object(columns, "OLD") + " AS old_json",
            "old_json",
            None,
        ))

    # Users: never store the password hash. Record only whether it changed.
    chunks.append(trigger(
        "users",
        "insert",
        "CAST(NEW.id AS TEXT)",
        json_object(USER_COLUMNS, "NEW", "'password_changed', 1") + " AS new_json",
        None,
        "new_json",
    ))
    chunks.append(trigger(
        "users",
        "update",
        "CAST(NEW.id AS TEXT)",
        json_object(USER_COLUMNS, "OLD", "'password_changed', 0")
        + " AS old_json, "
        + json_object(
            USER_COLUMNS,
            "NEW",
            "'password_changed', CASE WHEN OLD.password_hash = NEW.password_hash "
            "THEN 0 ELSE 1 END",
        )
        + " AS new_json",
        "old_json",
        "new_json",
    ))
    chunks.append(trigger(
        "users",
        "delete",
        "CAST(OLD.id AS TEXT)",
        json_object(USER_COLUMNS, "OLD", "'password_changed', 1") + " AS old_json",
        "old_json",
        None,
    ))

    chunks.append(
        """CREATE TRIGGER trg_audit_log_no_update
BEFORE UPDATE ON audit_log
BEGIN
    SELECT RAISE(ABORT, 'audit_log_is_append_only');
END;

CREATE TRIGGER trg_audit_log_no_delete
BEFORE DELETE ON audit_log
BEGIN
    SELECT RAISE(ABORT, 'audit_log_is_append_only');
END;
"""
    )

    return "\n".join(chunks)


def main() -> None:
    OUTPUT.write_text(build(), encoding="utf-8")
    print(f"wrote {OUTPUT}")


if __name__ == "__main__":
    main()
