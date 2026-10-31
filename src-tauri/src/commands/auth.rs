use crate::database::connection::DbManager;
use crate::services::auth::{AuthService, UserSession};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn login(
    username: String,
    password: String,
    db_manager: State<'_, Arc<DbManager>>,
) -> Result<UserSession, String> {
    let conn = db_manager.get_connection();
    AuthService::login(&conn, &username, &password).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn change_password(
    user_id: i64,
    old_password: String,
    new_password: String,
    db_manager: State<'_, Arc<DbManager>>,
) -> Result<(), String> {
    let conn = db_manager.get_connection();
    AuthService::change_password(&conn, user_id, &old_password, &new_password)
        .map_err(|e| e.to_string())
}
