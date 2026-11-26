//! Journal entry service.

use daftar_core::Context;
use daftar_db::Database;
use rusqlite::{params, OptionalExtension};

use crate::error::{ServiceError, ServiceResult};

/// Summary row for journal list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalRow {
    pub id: i64,
    pub number: Option<i64>,
    pub entry_date: String,
    pub description: String,
    pub status: String,
    pub total_base_debit: i64,
    pub total_base_credit: i64,
}

/// Line detail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalLineRow {
    pub line_no: i64,
    pub account_id: i64,
    pub account_code: String,
    pub account_name_ar: String,
    pub description: Option<String>,
    pub debit: i64,
    pub credit: i64,
    pub base_debit: i64,
    pub base_credit: i64,
}

/// Full entry detail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalEntryDetail {
    pub id: i64,
    pub number: Option<i64>,
    pub entry_date: String,
    pub description: String,
    pub reference: Option<String>,
    pub status: String,
    pub currency: String,
    pub exchange_rate: i64,
    pub created_by: i64,
    pub posted_by: Option<i64>,
    pub posted_at: Option<String>,
    pub lines: Vec<JournalLineRow>,
}

/// Input line for draft creation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftLineInput {
    pub account_id: i64,
    pub description: Option<String>,
    pub debit: i64,
    pub credit: i64,
}

/// Input for creating a draft journal entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateDraftInput {
    pub fiscal_year_id: i64,
    pub fiscal_period_id: i64,
    pub entry_date: String,
    pub description: String,
    pub reference: Option<String>,
    pub currency: String,
    pub exchange_rate: i64,
    pub lines: Vec<DraftLineInput>,
}

/// Lists journal entries, newest first.
pub fn list(db: &Database, ctx: &Context) -> ServiceResult<Vec<JournalRow>> {
    require_read(ctx)?;
    let mut stmt = db.conn.prepare(
        "SELECT e.id, e.number, e.entry_date, e.description, e.status,
                COALESCE(SUM(l.base_debit), 0), COALESCE(SUM(l.base_credit), 0)
         FROM journal_entries e
         LEFT JOIN journal_lines l ON l.entry_id = e.id
         GROUP BY e.id
         ORDER BY e.entry_date DESC, e.id DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(JournalRow {
            id: row.get(0)?,
            number: row.get(1)?,
            entry_date: row.get(2)?,
            description: row.get(3)?,
            status: row.get(4)?,
            total_base_debit: row.get(5)?,
            total_base_credit: row.get(6)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

/// Gets a journal entry with its lines.
pub fn get(db: &Database, ctx: &Context, id: i64) -> ServiceResult<JournalEntryDetail> {
    require_read(ctx)?;
    let entry = db
        .conn
        .query_row(
            "SELECT id, number, entry_date, description, reference, status,
                    currency, exchange_rate, created_by, posted_by, posted_at
             FROM journal_entries WHERE id = ?1",
            params![id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<i64>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, i64>(8)?,
                    row.get::<_, Option<i64>>(9)?,
                    row.get::<_, Option<String>>(10)?,
                ))
            },
        )
        .optional()?
        .ok_or(ServiceError::NotFound("القيد غير موجود"))?;

    let mut stmt = db.conn.prepare(
        "SELECT l.line_no, l.account_id, a.code, a.name_ar, l.description,
                l.debit, l.credit, l.base_debit, l.base_credit
         FROM journal_lines l
         JOIN accounts a ON a.id = l.account_id
         WHERE l.entry_id = ?1
         ORDER BY l.line_no",
    )?;
    let lines = stmt
        .query_map(params![id], |row| {
            Ok(JournalLineRow {
                line_no: row.get(0)?,
                account_id: row.get(1)?,
                account_code: row.get(2)?,
                account_name_ar: row.get(3)?,
                description: row.get(4)?,
                debit: row.get(5)?,
                credit: row.get(6)?,
                base_debit: row.get(7)?,
                base_credit: row.get(8)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(JournalEntryDetail {
        id: entry.0,
        number: entry.1,
        entry_date: entry.2,
        description: entry.3,
        reference: entry.4,
        status: entry.5,
        currency: entry.6,
        exchange_rate: entry.7,
        created_by: entry.8,
        posted_by: entry.9,
        posted_at: entry.10,
        lines,
    })
}

/// Creates a draft entry. Posting is a separate explicit action.
pub fn create_draft(db: &Database, ctx: &Context, input: CreateDraftInput) -> ServiceResult<i64> {
    ctx.require("entries.create")?;
    validate_draft(&input)?;

    db.set_user(Some(ctx.user_id));
    let now = daftar_core::clock::now_iso8601();
    let tx = db.conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO journal_entries
            (fiscal_year_id, fiscal_period_id, entry_date, reference, description,
             currency, exchange_rate, status, source_type, created_by, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'draft', 'manual', ?8, ?9, ?9)",
        params![
            input.fiscal_year_id,
            input.fiscal_period_id,
            input.entry_date.trim(),
            input.reference.as_deref().map(str::trim),
            input.description.trim(),
            input.currency.trim(),
            input.exchange_rate,
            ctx.user_id,
            now,
        ],
    )?;
    let entry_id = tx.last_insert_rowid();

    for (idx, line) in input.lines.iter().enumerate() {
        let line_no =
            i64::try_from(idx + 1).map_err(|_| ServiceError::Validation("عدد السطور كبير جداً"))?;
        let base_debit = line
            .debit
            .checked_mul(input.exchange_rate)
            .and_then(|v| v.checked_div(daftar_core::RATE_SCALE))
            .ok_or(ServiceError::Validation("فشل حساب المبلغ الأساسي"))?;
        let base_credit = line
            .credit
            .checked_mul(input.exchange_rate)
            .and_then(|v| v.checked_div(daftar_core::RATE_SCALE))
            .ok_or(ServiceError::Validation("فشل حساب المبلغ الأساسي"))?;
        tx.execute(
            "INSERT INTO journal_lines
                (entry_id, line_no, account_id, description, currency, exchange_rate,
                 debit, credit, base_debit, base_credit)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                entry_id,
                line_no,
                line.account_id,
                line.description.as_deref().map(str::trim),
                input.currency.trim(),
                input.exchange_rate,
                line.debit,
                line.credit,
                base_debit,
                base_credit,
            ],
        )?;
    }

    tx.commit()?;
    Ok(entry_id)
}

/// Posts a draft entry. DB triggers enforce balance, period, accounts, and numbering.
pub fn post_draft(db: &Database, ctx: &Context, entry_id: i64) -> ServiceResult<i64> {
    ctx.require("entries.post")?;
    db.set_user(Some(ctx.user_id));
    let now = daftar_core::clock::now_iso8601();
    let tx = db.conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO entry_counters (fiscal_year_id, last_number)
         SELECT fiscal_year_id, 1 FROM journal_entries WHERE id = ?1
         ON CONFLICT(fiscal_year_id) DO UPDATE SET last_number = last_number + 1",
        params![entry_id],
    )?;
    let number: i64 = tx.query_row(
        "SELECT last_number FROM entry_counters
         WHERE fiscal_year_id = (SELECT fiscal_year_id FROM journal_entries WHERE id = ?1)",
        params![entry_id],
        |row| row.get(0),
    )?;
    let changed = tx.execute(
        "UPDATE journal_entries
         SET status = 'posted', number = ?2, posted_by = ?3, posted_at = ?4, updated_at = ?4
         WHERE id = ?1 AND status = 'draft'",
        params![entry_id, number, ctx.user_id, now],
    )?;
    if changed == 0 {
        let status: Option<String> = tx
            .query_row(
                "SELECT status FROM journal_entries WHERE id = ?1",
                params![entry_id],
                |row| row.get(0),
            )
            .optional()?;
        tx.rollback()?;
        return match status.as_deref() {
            Some("posted") => Err(ServiceError::Conflict("القيد مرحّل بالفعل")),
            Some("void") => Err(ServiceError::Conflict("القيد مبطل ولا يمكن ترحيله")),
            Some(_) => Err(ServiceError::Conflict(
                "لا يمكن ترحيل هذا القيد في حالته الحالية",
            )),
            None => Err(ServiceError::NotFound("القيد غير موجود")),
        };
    }
    tx.commit()?;
    Ok(number)
}

/// Voids a draft entry. Posted entries must be reversed, not voided.
pub fn void_draft(db: &Database, ctx: &Context, entry_id: i64) -> ServiceResult<()> {
    ctx.require("entries.void")?;
    db.set_user(Some(ctx.user_id));
    let now = daftar_core::clock::now_iso8601();
    let changed = db.conn.execute(
        "UPDATE journal_entries SET status = 'void', updated_at = ?2
         WHERE id = ?1 AND status = 'draft'",
        params![entry_id, now],
    )?;
    if changed == 0 {
        let exists: Option<String> = db
            .conn
            .query_row(
                "SELECT status FROM journal_entries WHERE id = ?1",
                params![entry_id],
                |row| row.get(0),
            )
            .optional()?;
        return match exists.as_deref() {
            None => Err(ServiceError::NotFound("القيد غير موجود")),
            Some("posted") => Err(ServiceError::Conflict(
                "القيد المرحّل لا يُبطل؛ أنشئ قيداً عكسياً",
            )),
            Some("void") => Err(ServiceError::Conflict("القيد مبطل بالفعل")),
            Some(_) => Err(ServiceError::Conflict("لا يمكن إبطال هذا القيد")),
        };
    }
    Ok(())
}

fn require_read(ctx: &Context) -> ServiceResult<()> {
    if ctx.has("entries.create") || ctx.has("entries.post") || ctx.has("reports.view") {
        Ok(())
    } else {
        ctx.require("reports.view")
    }
}

fn validate_draft(input: &CreateDraftInput) -> ServiceResult<()> {
    if input.description.trim().is_empty() {
        return Err(ServiceError::Validation("وصف القيد مطلوب"));
    }
    daftar_core::Date::parse(&input.entry_date).map_err(ServiceError::Core)?;
    if input.currency.trim().is_empty() {
        return Err(ServiceError::Validation("عملة القيد مطلوبة"));
    }
    if input.exchange_rate <= 0 {
        return Err(ServiceError::Validation(
            "سعر الصرف يجب أن يكون أكبر من صفر",
        ));
    }
    if input.lines.len() < 2 {
        return Err(ServiceError::Validation("القيد يحتاج إلى سطرين على الأقل"));
    }
    let mut debit_total: i64 = 0;
    let mut credit_total: i64 = 0;
    for line in &input.lines {
        let valid_side =
            (line.debit > 0 && line.credit == 0) || (line.credit > 0 && line.debit == 0);
        if !valid_side {
            return Err(ServiceError::Validation(
                "كل سطر يجب أن يكون مديناً أو دائناً بقيمة موجبة",
            ));
        }
        debit_total = debit_total
            .checked_add(line.debit)
            .ok_or(ServiceError::Validation("إجمالي المدين خارج النطاق"))?;
        credit_total = credit_total
            .checked_add(line.credit)
            .ok_or(ServiceError::Validation("إجمالي الدائن خارج النطاق"))?;
    }
    if debit_total != credit_total {
        return Err(ServiceError::Validation(
            "القيد غير متوازن: المدين لا يساوي الدائن",
        ));
    }
    if debit_total == 0 {
        return Err(ServiceError::Validation("لا يمكن حفظ قيد بمبلغ صفر"));
    }
    Ok(())
}
