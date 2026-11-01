use crate::database::connection::DbManager;
use crate::services::developer_service::{
    AuditLogRow, CreateUserInput, DeveloperService, QueryExecutionResult, RuntimeLogEntry,
    SystemDiagnostics, UserAdminItem,
};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn get_system_diagnostics(
    state: State<'_, Arc<DbManager>>,
) -> Result<SystemDiagnostics, String> {
    let conn_arc = state.get_connection();
    DeveloperService::get_system_diagnostics(&conn_arc).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_audit_tables(state: State<'_, Arc<DbManager>>) -> Result<Vec<String>, String> {
    let conn_arc = state.get_connection();
    DeveloperService::get_audit_tables(&conn_arc).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_developer_audit_logs(
    state: State<'_, Arc<DbManager>>,
    table_name: String,
    limit: Option<i64>,
    action: Option<String>,
    search: Option<String>,
) -> Result<Vec<AuditLogRow>, String> {
    let conn_arc = state.get_connection();
    let limit_val = limit.unwrap_or(50).clamp(1, 500);
    DeveloperService::get_audit_logs(
        &conn_arc,
        &table_name,
        limit_val,
        action.as_deref(),
        search.as_deref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_runtime_logs(
    limit: Option<usize>,
    level: Option<String>,
) -> Result<Vec<RuntimeLogEntry>, String> {
    let limit_val = limit.unwrap_or(100).clamp(1, 500);
    Ok(DeveloperService::get_runtime_logs(limit_val, level.as_deref()))
}

#[tauri::command]
pub fn export_system_logs(state: State<'_, Arc<DbManager>>) -> Result<String, String> {
    let conn_arc = state.get_connection();
    Ok(DeveloperService::generate_diagnostic_log_export(&conn_arc))
}

#[tauri::command]
pub fn record_client_error(
    level: String,
    source: String,
    message: String,
) -> Result<(), String> {
    DeveloperService::record_runtime_log(&level, &source, &message);
    Ok(())
}

#[tauri::command]
pub fn execute_developer_query(
    state: State<'_, Arc<DbManager>>,
    sql: String,
) -> Result<QueryExecutionResult, String> {
    let conn_arc = state.get_connection();
    DeveloperService::execute_readonly_query(&conn_arc, &sql)
}

#[tauri::command]
pub fn developer_list_users(
    state: State<'_, Arc<DbManager>>,
) -> Result<Vec<UserAdminItem>, String> {
    let conn_arc = state.get_connection();
    DeveloperService::list_all_users(&conn_arc).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn developer_create_user(
    state: State<'_, Arc<DbManager>>,
    input: CreateUserInput,
    admin_id: i64,
) -> Result<i64, String> {
    let conn_arc = state.get_connection();
    DeveloperService::create_system_user(&conn_arc, input, admin_id)
}

#[tauri::command]
pub fn developer_reset_password(
    state: State<'_, Arc<DbManager>>,
    user_id: i64,
    new_password: String,
    admin_id: i64,
) -> Result<(), String> {
    let conn_arc = state.get_connection();
    DeveloperService::reset_user_password(&conn_arc, user_id, &new_password, admin_id)
}

#[tauri::command]
pub fn developer_toggle_user_status(
    state: State<'_, Arc<DbManager>>,
    user_id: i64,
    active_st: i64,
    admin_id: i64,
) -> Result<(), String> {
    let conn_arc = state.get_connection();
    DeveloperService::toggle_user_status(&conn_arc, user_id, active_st, admin_id)
}

#[tauri::command]
pub fn developer_update_user_role(
    state: State<'_, Arc<DbManager>>,
    user_id: i64,
    peran: String,
    admin_id: i64,
) -> Result<(), String> {
    let conn_arc = state.get_connection();
    DeveloperService::update_user_role(&conn_arc, user_id, &peran, admin_id)
}
