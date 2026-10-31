#[tauri::command]
fn get_system_status() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "status": "online",
        "version": "0.1.0",
        "app_name": "OlahPOS",
        "local_first": true,
        "database_engine": "SQLite Dual-DB"
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
        .invoke_handler(tauri::generate_handler![get_system_status])
        .run(tauri::generate_context!())
        .expect("error while running olahpos application");
}
