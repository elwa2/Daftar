//! Financial reports built on top of posted journal lines/views.

use daftar_core::Context;
use daftar_db::Database;
use rusqlite::params;

use crate::error::ServiceResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrialBalanceRow {
    pub account_id: i64,
    pub code: String,
    pub name_ar: String,
    pub account_type: String,
    pub normal_balance: String,
    pub debit: i64,
    pub credit: i64,
    pub net_debit: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatementRow {
    pub account_id: i64,
    pub code: String,
    pub name_ar: String,
    pub account_type: String,
    pub amount: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BalanceSheetRow {
    pub section: String,
    pub account_id: i64,
    pub code: String,
    pub name_ar: String,
    pub amount: i64,
}

pub fn trial_balance(db: &Database, ctx: &Context) -> ServiceResult<Vec<TrialBalanceRow>> {
    ctx.require("reports.view")?;
    let mut stmt = db.conn.prepare(
        "SELECT account_id, code, name_ar, type, normal_balance, debit, credit, net_debit
         FROM v_trial_balance
         ORDER BY code",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(TrialBalanceRow {
            account_id: row.get(0)?,
            code: row.get(1)?,
            name_ar: row.get(2)?,
            account_type: row.get(3)?,
            normal_balance: row.get(4)?,
            debit: row.get(5)?,
            credit: row.get(6)?,
            net_debit: row.get(7)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn income_statement(db: &Database, ctx: &Context) -> ServiceResult<Vec<StatementRow>> {
    ctx.require("reports.view")?;
    let mut stmt = db.conn.prepare(
        "SELECT account_id, code, name_ar, type, amount
         FROM v_income_statement
         ORDER BY type, code",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(StatementRow {
            account_id: row.get(0)?,
            code: row.get(1)?,
            name_ar: row.get(2)?,
            account_type: row.get(3)?,
            amount: row.get(4)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn balance_sheet(db: &Database, ctx: &Context) -> ServiceResult<Vec<BalanceSheetRow>> {
    ctx.require("reports.view")?;
    let mut stmt = db.conn.prepare(
        "SELECT section, account_id, code, name_ar, amount
         FROM v_balance_sheet
         ORDER BY section, code",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(BalanceSheetRow {
            section: row.get(0)?,
            account_id: row.get(1)?,
            code: row.get(2)?,
            name_ar: row.get(3)?,
            amount: row.get(4)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

/// Simple account ledger from posted lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerRow {
    pub entry_id: i64,
    pub entry_number: Option<i64>,
    pub entry_date: String,
    pub description: String,
    pub debit: i64,
    pub credit: i64,
    pub running_base_balance: i64,
}

pub fn account_ledger(
    db: &Database,
    ctx: &Context,
    account_id: i64,
) -> ServiceResult<Vec<LedgerRow>> {
    ctx.require("reports.view")?;
    let mut stmt = db.conn.prepare(
        "SELECT entry_id, number, entry_date,
                COALESCE(description, entry_description), base_debit, base_credit, running_base_balance
         FROM v_ledger
         WHERE account_id = ?1
         ORDER BY entry_date, entry_id, line_no",
    )?;
    let rows = stmt.query_map(params![account_id], |row| {
        Ok(LedgerRow {
            entry_id: row.get(0)?,
            entry_number: row.get(1)?,
            entry_date: row.get(2)?,
            description: row.get(3)?,
            debit: row.get(4)?,
            credit: row.get(5)?,
            running_base_balance: row.get(6)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}
