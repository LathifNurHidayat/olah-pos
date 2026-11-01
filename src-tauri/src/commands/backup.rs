use crate::database::connection::DbManager;
use crate::services::backup_service::{BackupItem, BackupResult, BackupService};
use std::path::Path;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn create_database_backup(
    db_manager: State<'_, Arc<DbManager>>,
) -> Result<BackupResult, String> {
    let conn = db_manager.get_connection();
    let paths = db_manager.get_paths();
    BackupService::create_backup_snapshot(&conn, &paths.main_db, &paths.log_db)
}

#[tauri::command]
pub fn list_database_backups() -> Result<Vec<BackupItem>, String> {
    BackupService::list_backups()
}

#[tauri::command]
pub fn restore_database_backup(
    backup_folder: String,
    db_manager: State<'_, Arc<DbManager>>,
) -> Result<(), String> {
    let paths = db_manager.get_paths();
    BackupService::restore_backup_snapshot(
        Path::new(&backup_folder),
        &paths.main_db,
        &paths.log_db,
    )
}

#[tauri::command]
pub fn setup_backup_scheduler() -> Result<String, String> {
    BackupService::setup_os_scheduler()
}
