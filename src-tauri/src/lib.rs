pub mod database;
pub mod services;

use database::connection::{DatabasePaths, DbManager};
use std::sync::Arc;
use tauri::Manager;

#[tauri::command]
fn get_system_status() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "status": "online",
        "version": "0.1.0",
        "app_name": "OlahPOS",
        "local_first": true,
        "database_engine": "SQLite Dual-DB (Attached)"
    }))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .setup(|app| {
            // Initialize database manager and encryption key
            let _app_key = services::crypto::get_or_create_app_key()
                .map_err(|e| format!("Failed to initialize encryption key: {}", e))?;

            let db_paths = DatabasePaths::default()
                .map_err(|e| format!("Failed to resolve database paths: {}", e))?;

            let db_manager = DbManager::init(db_paths)
                .map_err(|e| format!("Failed to initialize database: {}", e))?;

            app.manage(Arc::new(db_manager));
            log::info!("OlahPOS backend & dual-database initialized successfully");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_system_status])
        .run(tauri::generate_context!())
        .expect("error while running olahpos application");
}
