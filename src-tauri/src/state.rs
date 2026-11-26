use std::path::PathBuf;
use std::sync::Mutex;

use daftar_core::{CompanyId, Context};
use daftar_db::Database;
use tauri::{AppHandle, Manager};

pub struct AppState {
    pub db: Mutex<Option<Database>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            db: Mutex::new(None),
        }
    }
}

pub fn ensure_db(app: &AppHandle, state: &AppState) -> Result<(), String> {
    let mut guard = state
        .db
        .lock()
        .map_err(|_| "تعذر الوصول لحالة التطبيق".to_owned())?;
    if guard.is_some() {
        return Ok(());
    }
    let path = db_path(app)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("تعذر إنشاء مجلد البيانات: {e}"))?;
    }
    let db = Database::open(&path).map_err(map_db_err)?;
    daftar_services::company::bootstrap_demo(&db).map_err(map_service_err)?;
    *guard = Some(db);
    Ok(())
}

pub fn with_db<T>(
    app: &AppHandle,
    state: &AppState,
    f: impl FnOnce(&Database) -> Result<T, String>,
) -> Result<T, String> {
    ensure_db(app, state)?;
    let guard = state
        .db
        .lock()
        .map_err(|_| "تعذر الوصول لحالة التطبيق".to_owned())?;
    let db = guard
        .as_ref()
        .ok_or_else(|| "قاعدة البيانات غير جاهزة".to_owned())?;
    f(db)
}

pub fn demo_context() -> Context {
    // Temporary bootstrap context until auth/company switching UI is implemented.
    Context::new(
        CompanyId::new("demo"),
        1,
        [
            "accounts.manage",
            "entries.create",
            "entries.post",
            "entries.void",
            "reports.view",
            "settings.manage",
        ]
        .into_iter()
        .map(str::to_owned),
    )
}

fn db_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("تعذر تحديد مجلد البيانات: {e}"))?;
    Ok(dir.join("companies").join("demo.db"))
}

pub fn map_db_err(err: daftar_db::DbError) -> String {
    match &err {
        daftar_db::DbError::Sqlite(_) => "حدث خطأ في قاعدة البيانات".to_owned(),
        daftar_db::DbError::SchemaTooNew { .. } => {
            "إصدار قاعدة البيانات أحدث من إصدار التطبيق".to_owned()
        }
        daftar_db::DbError::InvalidSchemaVersion(_) => "إصدار قاعدة البيانات غير صالح".to_owned(),
        daftar_db::DbError::AuditChainBroken(_) => "تم اكتشاف تلف في سلسلة التدقيق".to_owned(),
    }
}

pub fn map_service_err(err: daftar_services::ServiceError) -> String {
    err.message_ar()
}
