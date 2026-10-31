use crate::database::connection::DbManager;
use crate::services::master_service::{
    BarangItem, JenisMutasiItem, KategoriItem, MasterService, SatuanItem, SaveBarangInput,
    SupplierItem,
};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn get_barang_list(
    db_manager: State<'_, Arc<DbManager>>,
) -> Result<Vec<BarangItem>, String> {
    let conn = db_manager.get_connection();
    MasterService::get_all_barang(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_barang(
    input: SaveBarangInput,
    user_id: Option<i64>,
    db_manager: State<'_, Arc<DbManager>>,
) -> Result<i64, String> {
    let conn = db_manager.get_connection();
    MasterService::save_barang(&conn, input, user_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_barang(
    id: i64,
    user_id: Option<i64>,
    db_manager: State<'_, Arc<DbManager>>,
) -> Result<(), String> {
    let conn = db_manager.get_connection();
    MasterService::delete_barang(&conn, id, user_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_kategori_list(
    db_manager: State<'_, Arc<DbManager>>,
) -> Result<Vec<KategoriItem>, String> {
    let conn = db_manager.get_connection();
    MasterService::get_all_kategori(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_satuan_list(
    db_manager: State<'_, Arc<DbManager>>,
) -> Result<Vec<SatuanItem>, String> {
    let conn = db_manager.get_connection();
    MasterService::get_all_satuan(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_supplier_list(
    db_manager: State<'_, Arc<DbManager>>,
) -> Result<Vec<SupplierItem>, String> {
    let conn = db_manager.get_connection();
    MasterService::get_all_supplier(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_jenis_mutasi_list(
    db_manager: State<'_, Arc<DbManager>>,
) -> Result<Vec<JenisMutasiItem>, String> {
    let conn = db_manager.get_connection();
    MasterService::get_all_jenis_mutasi(&conn).map_err(|e| e.to_string())
}
