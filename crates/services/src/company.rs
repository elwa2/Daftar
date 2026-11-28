//! Company bootstrap helpers.
//!
//! Creates the minimum data required for the UI to show accounts and journal:
//! base company info, a fiscal year/period, and a small default chart of
//! accounts. This is intentionally minimal; full setup wizard comes later.

use daftar_core::{clock::now_iso8601, Context};
use daftar_db::Database;
use rusqlite::{params, OptionalExtension};

use crate::error::{ServiceError, ServiceResult};

/// Ensures there is a company row, an open fiscal year/period covering today,
/// and a minimal chart of accounts. Safe to call repeatedly.
pub fn bootstrap_demo(db: &Database) -> ServiceResult<()> {
    let now = now_iso8601();
    let today = &now[0..10];

    db.conn.execute(
        "INSERT INTO company_info (id, name_ar, base_currency, created_at, updated_at)
         VALUES (1, 'شركة تجريبية', 'EGP', ?1, ?1)
         ON CONFLICT(id) DO NOTHING",
        params![now],
    )?;

    let year_id = ensure_fiscal_year(db, today, &now)?;
    let _period_id = ensure_period(db, year_id, today, &now)?;
    ensure_default_accounts(db, &now)?;
    Ok(())
}

/// editable company data shown in the app header and settings page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompanyProfile {
    pub name_ar: String,
    pub name_en: Option<String>,
    pub tax_id: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub base_currency: String,
}

/// Input for updating the company profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateCompanyInput {
    pub name_ar: String,
    pub name_en: Option<String>,
    pub tax_id: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub base_currency: String,
}

/// Returns the single company profile row.
pub fn get_profile(db: &Database, ctx: &Context) -> ServiceResult<CompanyProfile> {
    ctx.require("settings.manage")?;
    db.conn
        .query_row(
            "SELECT name_ar, name_en, tax_id, address, phone, email, base_currency
             FROM company_info WHERE id = 1",
            [],
            |row| {
                Ok(CompanyProfile {
                    name_ar: row.get(0)?,
                    name_en: row.get(1)?,
                    tax_id: row.get(2)?,
                    address: row.get(3)?,
                    phone: row.get(4)?,
                    email: row.get(5)?,
                    base_currency: row.get(6)?,
                })
            },
        )
        .optional()?
        .ok_or(ServiceError::NotFound("بيانات الشركة غير موجودة"))
}

/// Updates the company profile. Name and currency are validated against the
/// same rules used by the schema, so the UI gets an Arabic error before SQLite.
pub fn update_profile(
    db: &Database,
    ctx: &Context,
    input: UpdateCompanyInput,
) -> ServiceResult<()> {
    ctx.require("settings.manage")?;
    let name_ar = input.name_ar.trim();
    if name_ar.is_empty() {
        return Err(ServiceError::Validation("اسم الشركة مطلوب"));
    }
    let base_currency = input.base_currency.trim().to_uppercase();
    if base_currency.is_empty() {
        return Err(ServiceError::Validation("العملة الأساسية مطلوبة"));
    }
    let currency_exists: bool = db.conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM currencies WHERE code = ?1 AND is_active = 1)",
        params![base_currency],
        |row| row.get(0),
    )?;
    if !currency_exists {
        return Err(ServiceError::Validation("العملة الأساسية غير مدعومة"));
    }

    let now = now_iso8601();
    db.set_user(Some(ctx.user_id));
    let changed = db.conn.execute(
        "UPDATE company_info
         SET name_ar = ?1, name_en = ?2, tax_id = ?3, address = ?4,
             phone = ?5, email = ?6, base_currency = ?7, updated_at = ?8
         WHERE id = 1",
        params![
            name_ar,
            clean_optional(input.name_en.as_deref()),
            clean_optional(input.tax_id.as_deref()),
            clean_optional(input.address.as_deref()),
            clean_optional(input.phone.as_deref()),
            clean_optional(input.email.as_deref()),
            base_currency,
            now,
        ],
    )?;
    if changed == 0 {
        return Err(ServiceError::NotFound("بيانات الشركة غير موجودة"));
    }
    Ok(())
}

fn clean_optional(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

/// Returns the display name of a fiscal year.
pub fn fiscal_year_name(db: &Database, id: i64) -> ServiceResult<String> {
    db.conn
        .query_row(
            "SELECT name FROM fiscal_years WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .optional()?
        .ok_or(ServiceError::NotFound("السنة المالية غير موجودة"))
}

fn ensure_fiscal_year(db: &Database, today: &str, now: &str) -> ServiceResult<i64> {
    let existing: Option<i64> = db
        .conn
        .query_row(
            "SELECT id FROM fiscal_years WHERE start_date <= ?1 AND end_date >= ?1 ORDER BY id LIMIT 1",
            params![today],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(id) = existing {
        return Ok(id);
    }
    let year = &today[0..4];
    let start = format!("{year}-01-01");
    let end = format!("{year}-12-31");
    db.conn.execute(
        "INSERT INTO fiscal_years (name, start_date, end_date, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![year, start, end, now],
    )?;
    let id = db.conn.last_insert_rowid();
    db.conn.execute(
        "INSERT OR IGNORE INTO entry_counters (fiscal_year_id, last_number) VALUES (?1, 0)",
        params![id],
    )?;
    Ok(id)
}

fn ensure_period(db: &Database, year_id: i64, today: &str, now: &str) -> ServiceResult<i64> {
    let existing: Option<i64> = db
        .conn
        .query_row(
            "SELECT id FROM fiscal_periods WHERE fiscal_year_id = ?1 AND start_date <= ?2 AND end_date >= ?2 ORDER BY id LIMIT 1",
            params![year_id, today],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(id) = existing {
        return Ok(id);
    }
    let year = &today[0..4];
    let month = &today[5..7];
    let (start, end) = month_bounds(year, month);
    let name = format!("{year}-{month}");
    db.conn.execute(
        "INSERT INTO fiscal_periods (fiscal_year_id, name, start_date, end_date)
         VALUES (?1, ?2, ?3, ?4)",
        params![year_id, name, start, end],
    )?;
    let _ = now;
    Ok(db.conn.last_insert_rowid())
}

fn month_bounds(year: &str, month: &str) -> (String, String) {
    let start = format!("{year}-{month}-01");
    let end_day = match month {
        "01" | "03" | "05" | "07" | "08" | "10" | "12" => "31",
        "04" | "06" | "09" | "11" => "30",
        "02" => {
            let y: i32 = year.parse().unwrap_or(2000);
            if (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0) {
                "29"
            } else {
                "28"
            }
        }
        _ => "30",
    };
    (start, format!("{year}-{month}-{end_day}"))
}

/// `(code, name_ar, type, normal_balance, parent_code, is_postable)`.
type DefaultAccount<'a> = (&'a str, &'a str, &'a str, &'a str, Option<&'a str>, i64);

fn ensure_default_accounts(db: &Database, now: &str) -> ServiceResult<()> {
    let count: i64 = db
        .conn
        .query_row("SELECT COUNT(*) FROM accounts", [], |r| r.get(0))?;
    if count > 0 {
        return Ok(());
    }
    let accounts: &[DefaultAccount<'_>] = &[
        ("1", "الأصول", "asset", "debit", None, 0),
        ("11", "الأصول المتداولة", "asset", "debit", Some("1"), 0),
        ("1110", "الصندوق", "asset", "debit", Some("11"), 1),
        ("1120", "البنك", "asset", "debit", Some("11"), 1),
        ("1130", "العملاء", "asset", "debit", Some("11"), 1),
        ("2", "الالتزامات", "liability", "credit", None, 0),
        (
            "21",
            "الالتزامات المتداولة",
            "liability",
            "credit",
            Some("2"),
            0,
        ),
        ("2110", "الموردون", "liability", "credit", Some("21"), 1),
        ("3", "حقوق الملكية", "equity", "credit", None, 0),
        ("3100", "رأس المال", "equity", "credit", Some("3"), 1),
        ("4", "الإيرادات", "revenue", "credit", None, 0),
        ("4100", "إيراد المبيعات", "revenue", "credit", Some("4"), 1),
        ("5", "المصروفات", "expense", "debit", None, 0),
        ("5100", "مصروف الإيجار", "expense", "debit", Some("5"), 1),
        ("5200", "مصروف كهرباء", "expense", "debit", Some("5"), 1),
    ];
    for (code, name_ar, acc_type, normal, parent_code, postable) in accounts {
        let parent_id: Option<i64> = match parent_code {
            Some(pc) => db
                .conn
                .query_row(
                    "SELECT id FROM accounts WHERE code = ?1",
                    params![pc],
                    |r| r.get(0),
                )
                .optional()?,
            None => None,
        };
        db.conn.execute(
            "INSERT INTO accounts
                (code, name_ar, type, normal_balance, parent_id, is_postable, is_active, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, ?7)",
            params![code, name_ar, acc_type, normal, parent_id, postable, now],
        )?;
    }
    Ok(())
}

/// Returns an open fiscal period covering the given date.
pub fn period_for_date(db: &Database, date: &str) -> ServiceResult<(i64, i64)> {
    bootstrap_demo(db)?;
    db.conn
        .query_row(
            "SELECT p.fiscal_year_id, p.id
             FROM fiscal_periods p
             JOIN fiscal_years y ON y.id = p.fiscal_year_id
             WHERE p.start_date <= ?1 AND p.end_date >= ?1
               AND p.is_closed = 0 AND y.is_closed = 0
             ORDER BY p.id LIMIT 1",
            params![date],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?
        .ok_or(crate::error::ServiceError::NotFound(
            "لا توجد فترة مالية مفتوحة للتاريخ المحدد",
        ))
}

/// Compatibility wrapper.
pub fn current_period(db: &Database, today: &str) -> ServiceResult<(i64, i64)> {
    period_for_date(db, today)
}
