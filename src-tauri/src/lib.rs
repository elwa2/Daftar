mod commands;
mod state;

use state::AppState;

pub fn run() -> tauri::Result<()> {
    tauri::Builder::default()
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap,
            commands::update_company,
            commands::list_accounts,
            commands::create_account,
            commands::list_journal,
            commands::get_journal_entry,
            commands::create_draft,
            commands::post_draft,
            commands::void_draft,
            commands::trial_balance,
            commands::income_statement,
            commands::balance_sheet_report,
            commands::account_ledger,
        ])
        .run(tauri::generate_context!())
}
