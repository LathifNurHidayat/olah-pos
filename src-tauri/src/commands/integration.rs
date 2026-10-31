use crate::database::connection::DbManager;
use crate::services::integration_service::{IntegrationService, MidtransQRISResponse};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn save_integration_config(
    kode_layanan: String,
    nama_layanan: String,
    payload: serde_json::Value,
    catatan: Option<String>,
    user_id: Option<i64>,
    db_manager: State<'_, Arc<DbManager>>,
) -> Result<(), String> {
    let conn = db_manager.get_connection();
    IntegrationService::save_config(
        &conn,
        &kode_layanan,
        &nama_layanan,
        &payload,
        catatan,
        user_id,
    )
}

#[tauri::command]
pub fn get_integration_config(
    kode_layanan: String,
    db_manager: State<'_, Arc<DbManager>>,
) -> Result<Option<serde_json::Value>, String> {
    let conn = db_manager.get_connection();
    IntegrationService::get_config(&conn, &kode_layanan)
}

#[tauri::command]
pub fn generate_qris(
    order_id: String,
    gross_amount: i64,
    db_manager: State<'_, Arc<DbManager>>,
) -> Result<MidtransQRISResponse, String> {
    let conn = db_manager.get_connection();
    IntegrationService::generate_qris(&conn, &order_id, gross_amount)
}
