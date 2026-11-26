//! Chart of accounts service.

use daftar_core::Context;
use daftar_db::Database;
use rusqlite::{params, OptionalExtension};

use crate::error::{ServiceError, ServiceResult};

/// A row returned by [`list`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountRow {
    pub id: i64,
    pub code: String,
    pub name_ar: String,
    pub name_en: Option<String>,
    pub account_type: String,
    pub normal_balance: String,
    pub parent_id: Option<i64>,
    pub is_postable: bool,
    pub is_active: bool,
    pub currency: Option<String>,
}

/// Input for creating an account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateAccountInput {
    pub code: String,
    pub name_ar: String,
    pub name_en: Option<String>,
    pub account_type: String,
    pub normal_balance: String,
    pub parent_id: Option<i64>,
    pub is_postable: bool,
    pub currency: Option<String>,
}

/// Lists accounts ordered by code.
pub fn list(db: &Database, ctx: &Context) -> ServiceResult<Vec<AccountRow>> {
    require_read(ctx)?;
    let mut stmt = db.conn.prepare(
        "SELECT id, code, name_ar, name_en, type, normal_balance, parent_id,
                is_postable, is_active, currency
         FROM accounts
         ORDER BY code",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(AccountRow {
            id: row.get(0)?,
            code: row.get(1)?,
            name_ar: row.get(2)?,
            name_en: row.get(3)?,
            account_type: row.get(4)?,
            normal_balance: row.get(5)?,
            parent_id: row.get(6)?,
            is_postable: row.get::<_, i64>(7)? != 0,
            is_active: row.get::<_, i64>(8)? != 0,
            currency: row.get(9)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

/// Creates an account. DB triggers enforce parent/type/normal-balance rules.
pub fn create(db: &Database, ctx: &Context, input: CreateAccountInput) -> ServiceResult<i64> {
    ctx.require("accounts.manage")?;
    validate_input(&input)?;

    db.set_user(Some(ctx.user_id));
    let now = daftar_core::clock::now_iso8601();
    db.conn.execute(
        "INSERT INTO accounts
            (code, name_ar, name_en, type, normal_balance, parent_id,
             is_postable, is_active, currency, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, ?8, ?9, ?9)",
        params![
            input.code.trim(),
            input.name_ar.trim(),
            input.name_en.as_deref().map(str::trim),
            input.account_type,
            input.normal_balance,
            input.parent_id,
            i64::from(input.is_postable),
            input.currency.as_deref().map(str::trim),
            now,
        ],
    )?;
    Ok(db.conn.last_insert_rowid())
}

/// Returns a single account by id.
pub fn get(db: &Database, ctx: &Context, id: i64) -> ServiceResult<AccountRow> {
    require_read(ctx)?;
    db.conn
        .query_row(
            "SELECT id, code, name_ar, name_en, type, normal_balance, parent_id,
                    is_postable, is_active, currency
             FROM accounts WHERE id = ?1",
            params![id],
            |row| {
                Ok(AccountRow {
                    id: row.get(0)?,
                    code: row.get(1)?,
                    name_ar: row.get(2)?,
                    name_en: row.get(3)?,
                    account_type: row.get(4)?,
                    normal_balance: row.get(5)?,
                    parent_id: row.get(6)?,
                    is_postable: row.get::<_, i64>(7)? != 0,
                    is_active: row.get::<_, i64>(8)? != 0,
                    currency: row.get(9)?,
                })
            },
        )
        .optional()?
        .ok_or(ServiceError::NotFound("الحساب غير موجود"))
}

fn require_read(ctx: &Context) -> ServiceResult<()> {
    if ctx.has("accounts.manage") || ctx.has("reports.view") {
        Ok(())
    } else {
        ctx.require("reports.view")
    }
}

fn validate_input(input: &CreateAccountInput) -> ServiceResult<()> {
    if input.code.trim().is_empty() {
        return Err(ServiceError::Validation("كود الحساب مطلوب"));
    }
    if input.name_ar.trim().is_empty() {
        return Err(ServiceError::Validation("اسم الحساب مطلوب"));
    }
    let valid_type = matches!(
        input.account_type.as_str(),
        "asset" | "liability" | "equity" | "revenue" | "expense"
    );
    if !valid_type {
        return Err(ServiceError::Validation("نوع الحساب غير صالح"));
    }
    let valid_normal = matches!(input.normal_balance.as_str(), "debit" | "credit");
    if !valid_normal {
        return Err(ServiceError::Validation("الطبيعة المحاسبية غير صالحة"));
    }
    Ok(())
}
