pub mod commands;
pub mod database;
pub mod services;

use database::connection::{DatabasePaths, DbManager};
use database::migrator::Migrator;
use services::auth::AuthService;
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

            // Run pending migrations automatically
            let conn_arc = db_manager.get_connection();
            {
                let mut conn = conn_arc.lock().unwrap();
                Migrator::run(&mut conn)
                    .map_err(|e| format!("Failed to run database migrations: {}", e))?;
            }

            // Bootstrap developer account securely if no active users exist
            AuthService::bootstrap_developer_if_needed(&conn_arc)
                .map_err(|e| format!("Failed to bootstrap developer account: {}", e))?;

            app.manage(Arc::new(db_manager));
            log::info!("OlahPOS backend, database, migrations and auth ready");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_system_status,
            commands::auth::login,
            commands::auth::change_password,
            commands::master::get_barang_list,
            commands::master::save_barang,
            commands::master::delete_barang,
            commands::master::get_kategori_list,
            commands::master::get_satuan_list,
            commands::master::get_supplier_list,
            commands::master::get_jenis_mutasi_list,
            commands::inventory::get_inventory_overview,
            commands::inventory::get_inventory_stats,
            commands::inventory::get_stock_mutations,
            commands::inventory::process_stock_opname
        ])
        .run(tauri::generate_context!())
        .expect("error while running olahpos application");
}
