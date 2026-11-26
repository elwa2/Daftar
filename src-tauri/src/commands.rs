use daftar_services::{accounts, company, journal, reports};
use serde::Serialize;
use tauri::{AppHandle, State};

use crate::state::{demo_context, map_service_err, with_db, AppState};

#[derive(Serialize)]
pub struct BootstrapInfo {
    pub company_name: String,
    pub fiscal_year_id: i64,
    pub fiscal_period_id: i64,
}

#[derive(Serialize)]
pub struct AccountDto {
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

#[derive(Serialize)]
pub struct JournalRowDto {
    pub id: i64,
    pub number: Option<i64>,
    pub entry_date: String,
    pub description: String,
    pub status: String,
    pub total_base_debit: i64,
    pub total_base_credit: i64,
}

#[derive(Serialize)]
pub struct JournalLineDto {
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

#[derive(Serialize)]
pub struct JournalEntryDto {
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
    pub lines: Vec<JournalLineDto>,
}

#[derive(serde::Deserialize)]
pub struct CreateAccountDto {
    pub code: String,
    pub name_ar: String,
    pub name_en: Option<String>,
    pub account_type: String,
    pub normal_balance: String,
    pub parent_id: Option<i64>,
    pub is_postable: bool,
    pub currency: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct DraftLineDto {
    pub account_id: i64,
    pub description: Option<String>,
    pub debit: i64,
    pub credit: i64,
}

#[derive(serde::Deserialize)]
pub struct CreateDraftDto {
    pub entry_date: String,
    pub description: String,
    pub reference: Option<String>,
    pub currency: String,
    pub exchange_rate: i64,
    pub lines: Vec<DraftLineDto>,
}

#[tauri::command]
pub fn bootstrap(app: AppHandle, state: State<'_, AppState>) -> Result<BootstrapInfo, String> {
    with_db(&app, &state, |db| {
        let today = &daftar_core::clock::now_iso8601()[0..10];
        let (year_id, period_id) = company::current_period(db, today).map_err(map_service_err)?;
        let company_name: String = db
            .conn
            .query_row("SELECT name_ar FROM company_info WHERE id = 1", [], |r| {
                r.get(0)
            })
            .map_err(|_| "تعذر قراءة بيانات الشركة".to_owned())?;
        Ok(BootstrapInfo {
            company_name,
            fiscal_year_id: year_id,
            fiscal_period_id: period_id,
        })
    })
}

#[tauri::command]
pub fn list_accounts(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<AccountDto>, String> {
    let ctx = demo_context();
    with_db(&app, &state, |db| {
        let rows = accounts::list(db, &ctx).map_err(map_service_err)?;
        Ok(rows
            .into_iter()
            .map(|r| AccountDto {
                id: r.id,
                code: r.code,
                name_ar: r.name_ar,
                name_en: r.name_en,
                account_type: r.account_type,
                normal_balance: r.normal_balance,
                parent_id: r.parent_id,
                is_postable: r.is_postable,
                is_active: r.is_active,
                currency: r.currency,
            })
            .collect())
    })
}

#[tauri::command]
pub fn create_account(
    app: AppHandle,
    state: State<'_, AppState>,
    input: CreateAccountDto,
) -> Result<i64, String> {
    let ctx = demo_context();
    with_db(&app, &state, |db| {
        accounts::create(
            db,
            &ctx,
            accounts::CreateAccountInput {
                code: input.code,
                name_ar: input.name_ar,
                name_en: input.name_en,
                account_type: input.account_type,
                normal_balance: input.normal_balance,
                parent_id: input.parent_id,
                is_postable: input.is_postable,
                currency: input.currency,
            },
        )
        .map_err(map_service_err)
    })
}

#[tauri::command]
pub fn list_journal(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<JournalRowDto>, String> {
    let ctx = demo_context();
    with_db(&app, &state, |db| {
        let rows = journal::list(db, &ctx).map_err(map_service_err)?;
        Ok(rows
            .into_iter()
            .map(|r| JournalRowDto {
                id: r.id,
                number: r.number,
                entry_date: r.entry_date,
                description: r.description,
                status: r.status,
                total_base_debit: r.total_base_debit,
                total_base_credit: r.total_base_credit,
            })
            .collect())
    })
}

#[tauri::command]
pub fn get_journal_entry(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
) -> Result<JournalEntryDto, String> {
    let ctx = demo_context();
    with_db(&app, &state, |db| {
        let e = journal::get(db, &ctx, id).map_err(map_service_err)?;
        Ok(JournalEntryDto {
            id: e.id,
            number: e.number,
            entry_date: e.entry_date,
            description: e.description,
            reference: e.reference,
            status: e.status,
            currency: e.currency,
            exchange_rate: e.exchange_rate,
            created_by: e.created_by,
            posted_by: e.posted_by,
            posted_at: e.posted_at,
            lines: e
                .lines
                .into_iter()
                .map(|l| JournalLineDto {
                    line_no: l.line_no,
                    account_id: l.account_id,
                    account_code: l.account_code,
                    account_name_ar: l.account_name_ar,
                    description: l.description,
                    debit: l.debit,
                    credit: l.credit,
                    base_debit: l.base_debit,
                    base_credit: l.base_credit,
                })
                .collect(),
        })
    })
}

#[tauri::command]
pub fn create_draft(
    app: AppHandle,
    state: State<'_, AppState>,
    input: CreateDraftDto,
) -> Result<i64, String> {
    let ctx = demo_context();
    with_db(&app, &state, |db| {
        let today = &daftar_core::clock::now_iso8601()[0..10];
        let (fiscal_year_id, fiscal_period_id) =
            company::current_period(db, today).map_err(map_service_err)?;
        journal::create_draft(
            db,
            &ctx,
            journal::CreateDraftInput {
                fiscal_year_id,
                fiscal_period_id,
                entry_date: input.entry_date,
                description: input.description,
                reference: input.reference,
                currency: input.currency,
                exchange_rate: input.exchange_rate,
                lines: input
                    .lines
                    .into_iter()
                    .map(|l| journal::DraftLineInput {
                        account_id: l.account_id,
                        description: l.description,
                        debit: l.debit,
                        credit: l.credit,
                    })
                    .collect(),
            },
        )
        .map_err(map_service_err)
    })
}

#[tauri::command]
pub fn post_draft(app: AppHandle, state: State<'_, AppState>, id: i64) -> Result<i64, String> {
    let ctx = demo_context();
    with_db(&app, &state, |db| {
        journal::post_draft(db, &ctx, id).map_err(map_service_err)
    })
}

#[tauri::command]
pub fn void_draft(app: AppHandle, state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let ctx = demo_context();
    with_db(&app, &state, |db| {
        journal::void_draft(db, &ctx, id).map_err(map_service_err)
    })
}

#[derive(Serialize)]
pub struct TrialBalanceDto {
    pub account_id: i64,
    pub code: String,
    pub name_ar: String,
    pub account_type: String,
    pub normal_balance: String,
    pub debit: i64,
    pub credit: i64,
    pub net_debit: i64,
}

#[derive(Serialize)]
pub struct StatementDto {
    pub account_id: i64,
    pub code: String,
    pub name_ar: String,
    pub account_type: String,
    pub amount: i64,
}

#[derive(Serialize)]
pub struct BalanceSheetDto {
    pub section: String,
    pub account_id: i64,
    pub code: String,
    pub name_ar: String,
    pub amount: i64,
}

#[tauri::command]
pub fn trial_balance(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<TrialBalanceDto>, String> {
    let ctx = demo_context();
    with_db(&app, &state, |db| {
        Ok(reports::trial_balance(db, &ctx)
            .map_err(map_service_err)?
            .into_iter()
            .map(|r| TrialBalanceDto {
                account_id: r.account_id,
                code: r.code,
                name_ar: r.name_ar,
                account_type: r.account_type,
                normal_balance: r.normal_balance,
                debit: r.debit,
                credit: r.credit,
                net_debit: r.net_debit,
            })
            .collect())
    })
}

#[tauri::command]
pub fn income_statement(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<StatementDto>, String> {
    let ctx = demo_context();
    with_db(&app, &state, |db| {
        Ok(reports::income_statement(db, &ctx)
            .map_err(map_service_err)?
            .into_iter()
            .map(|r| StatementDto {
                account_id: r.account_id,
                code: r.code,
                name_ar: r.name_ar,
                account_type: r.account_type,
                amount: r.amount,
            })
            .collect())
    })
}

#[tauri::command]
pub fn balance_sheet_report(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<BalanceSheetDto>, String> {
    let ctx = demo_context();
    with_db(&app, &state, |db| {
        Ok(reports::balance_sheet(db, &ctx)
            .map_err(map_service_err)?
            .into_iter()
            .map(|r| BalanceSheetDto {
                section: r.section,
                account_id: r.account_id,
                code: r.code,
                name_ar: r.name_ar,
                amount: r.amount,
            })
            .collect())
    })
}
