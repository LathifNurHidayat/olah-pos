use crate::database::connection::DbManager;
use crate::services::sales_service::{
    CheckoutInput, CheckoutReceiptResult, PenjualanListItem, SalesService,
};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn process_checkout(
    input: CheckoutInput,
    db_manager: State<'_, Arc<DbManager>>,
) -> Result<CheckoutReceiptResult, String> {
    let conn = db_manager.get_connection();
    SalesService::process_checkout(&conn, input)
}

#[tauri::command]
pub fn get_penjualan_list(
    limit: Option<u32>,
    offset: Option<u32>,
    db_manager: State<'_, Arc<DbManager>>,
) -> Result<Vec<PenjualanListItem>, String> {
    let conn = db_manager.get_connection();
    let lim = limit.unwrap_or(50);
    let off = offset.unwrap_or(0);
    SalesService::get_penjualan_list(&conn, lim, off).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_receipt_detail(
    penjualan_id: i64,
    db_manager: State<'_, Arc<DbManager>>,
) -> Result<CheckoutReceiptResult, String> {
    let conn = db_manager.get_connection();
    SalesService::get_receipt_detail(&conn, penjualan_id).map_err(|e| e.to_string())
}
