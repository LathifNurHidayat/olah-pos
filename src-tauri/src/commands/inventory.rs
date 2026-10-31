use crate::database::connection::DbManager;
use crate::services::inventory_service::{
    InventoryItem, InventoryService, InventoryStats, StockMutationItem, StockOpnameInput,
};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn get_inventory_overview(
    db_manager: State<'_, Arc<DbManager>>,
) -> Result<Vec<InventoryItem>, String> {
    let conn = db_manager.get_connection();
    InventoryService::get_inventory_overview(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_inventory_stats(
    db_manager: State<'_, Arc<DbManager>>,
) -> Result<InventoryStats, String> {
    let conn = db_manager.get_connection();
    InventoryService::get_inventory_stats(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_stock_mutations(
    barang_id: Option<i64>,
    limit: Option<u32>,
    offset: Option<u32>,
    db_manager: State<'_, Arc<DbManager>>,
) -> Result<Vec<StockMutationItem>, String> {
    let conn = db_manager.get_connection();
    let lim = limit.unwrap_or(50);
    let off = offset.unwrap_or(0);
    InventoryService::get_stock_mutations(&conn, barang_id, lim, off).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn process_stock_opname(
    input: StockOpnameInput,
    db_manager: State<'_, Arc<DbManager>>,
) -> Result<i64, String> {
    let conn = db_manager.get_connection();
    InventoryService::process_stock_opname(&conn, &input).map_err(|e| e.to_string())
}
