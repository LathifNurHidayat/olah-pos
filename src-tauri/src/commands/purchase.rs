use crate::database::connection::DbManager;
use crate::services::purchase_service::{
    BayarUtangInput, CreatePembelianInput, DetailPembelianItem, PembayaranUtangItem,
    PembelianListItem, PurchaseService, UtangItem,
};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn create_pembelian(
    input: CreatePembelianInput,
    db_manager: State<'_, Arc<DbManager>>,
) -> Result<i64, String> {
    let conn = db_manager.get_connection();
    PurchaseService::create_pembelian(&conn, input)
}

#[tauri::command]
pub fn get_pembelian_list(
    db_manager: State<'_, Arc<DbManager>>,
) -> Result<Vec<PembelianListItem>, String> {
    let conn = db_manager.get_connection();
    PurchaseService::get_pembelian_list(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_detail_pembelian(
    pembelian_id: i64,
    db_manager: State<'_, Arc<DbManager>>,
) -> Result<Vec<DetailPembelianItem>, String> {
    let conn = db_manager.get_connection();
    PurchaseService::get_detail_pembelian(&conn, pembelian_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_utang_list(
    status_filter: Option<String>,
    db_manager: State<'_, Arc<DbManager>>,
) -> Result<Vec<UtangItem>, String> {
    let conn = db_manager.get_connection();
    PurchaseService::get_utang_list(&conn, status_filter).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn bayar_utang(
    input: BayarUtangInput,
    db_manager: State<'_, Arc<DbManager>>,
) -> Result<i64, String> {
    let conn = db_manager.get_connection();
    PurchaseService::bayar_utang(&conn, input)
}

#[tauri::command]
pub fn get_pembayaran_utang_history(
    utang_id: i64,
    db_manager: State<'_, Arc<DbManager>>,
) -> Result<Vec<PembayaranUtangItem>, String> {
    let conn = db_manager.get_connection();
    PurchaseService::get_pembayaran_utang_history(&conn, utang_id).map_err(|e| e.to_string())
}
