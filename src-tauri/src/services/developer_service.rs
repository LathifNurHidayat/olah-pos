use crate::database::audit::{AuditAction, AuditLogger};
use crate::database::connection::{DatabasePaths, DbConnection};
use crate::services::auth::AuthService;
use crate::services::crypto::{self, get_or_create_app_key};
use rusqlite::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::{LazyLock, Mutex};
use std::time::Instant;

static START_TIME: LazyLock<Instant> = LazyLock::new(Instant::now);
static RUNTIME_LOGS: LazyLock<Mutex<Vec<RuntimeLogEntry>>> = LazyLock::new(|| Mutex::new(Vec::new()));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeLogEntry {
    pub id: usize,
    pub timestamp: String,
    pub level: String, // "INFO" | "WARN" | "ERROR"
    pub source: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemDiagnostics {
    pub app_name: String,
    pub app_version: String,
    pub os_info: String,
    pub main_db_path: String,
    pub main_db_size_bytes: u64,
    pub log_db_path: String,
    pub log_db_size_bytes: u64,
    pub sqlite_version: String,
    pub journal_mode: String,
    pub dual_db_attached: bool,
    pub db_integrity_status: String,
    pub encryption_key_status: String,
    pub uptime_seconds: u64,
    pub total_audit_tables_count: usize,
    pub total_audit_records_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogRow {
    pub log_id: i64,
    pub table_name: String,
    pub created_at: String,
    pub created_by: Option<i64>,
    pub primary_key_id: String,
    pub aksi: String,
    pub content: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAdminItem {
    pub pengguna_id: i64,
    pub nama_lengkap: String,
    pub nama_pengguna: String,
    pub peran: String,
    pub active_st: i64,
    pub harus_ganti_password: bool,
    pub terakhir_login: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserInput {
    pub nama_lengkap: String,
    pub nama_pengguna: String,
    pub password: String,
    pub peran: String, // "ADMIN" | "CASHIER"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryExecutionResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub row_count: usize,
    pub execution_time_ms: u128,
}

pub struct DeveloperService;

impl DeveloperService {
    /// Catat log runtime ke memory ring buffer
    pub fn record_runtime_log(level: &str, source: &str, message: &str) {
        let mut logs = RUNTIME_LOGS.lock().unwrap();
        let next_id = logs.len() + 1;
        let entry = RuntimeLogEntry {
            id: next_id,
            timestamp: chrono::Utc::now().to_rfc3339(),
            level: level.to_uppercase(),
            source: source.to_string(),
            message: message.to_string(),
        };

        logs.push(entry);
        // Pertahankan maksimal 500 log terakhir
        if logs.len() > 500 {
            logs.remove(0);
        }
    }

    /// Ambil daftar runtime logs
    pub fn get_runtime_logs(limit: usize, level_filter: Option<&str>) -> Vec<RuntimeLogEntry> {
        let logs = RUNTIME_LOGS.lock().unwrap();
        let mut filtered: Vec<RuntimeLogEntry> = logs
            .iter()
            .filter(|l| {
                if let Some(lvl) = level_filter {
                    if lvl != "ALL" && !lvl.is_empty() {
                        return l.level == lvl.to_uppercase();
                    }
                }
                true
            })
            .cloned()
            .collect();

        filtered.reverse(); // Terbaru di atas
        filtered.truncate(limit);
        filtered
    }

    /// Ambil diagnostik kesehatan sistem & dual database
    pub fn get_system_diagnostics(db: &DbConnection) -> Result<SystemDiagnostics> {
        let conn = db.lock().unwrap();

        // 1. Paths & File sizes
        let paths = DatabasePaths::default().unwrap_or(DatabasePaths {
            main_db: std::path::PathBuf::from("kasir_pos.db"),
            log_db: std::path::PathBuf::from("kasir_pos_log.db"),
        });

        let main_size = std::fs::metadata(&paths.main_db)
            .map(|m| m.len())
            .unwrap_or(0);
        let log_size = std::fs::metadata(&paths.log_db)
            .map(|m| m.len())
            .unwrap_or(0);

        // 2. SQLite Version & Journal Mode
        let sqlite_ver: String = conn.query_row("SELECT sqlite_version();", [], |r| r.get(0))?;
        let journal_mode: String = conn.query_row("PRAGMA journal_mode;", [], |r| r.get(0))?;

        // 3. Database List (Cek ATTACH log_db)
        let mut stmt = conn.prepare("PRAGMA database_list;")?;
        let db_names: Vec<String> = stmt
            .query_map([], |r| r.get::<_, String>(1))?
            .filter_map(|r| r.ok())
            .collect();
        let dual_db_attached = db_names.contains(&"log_db".to_string());

        // 4. Integrity Check
        let db_integrity_status: String = conn
            .query_row("PRAGMA integrity_check;", [], |r| r.get(0))
            .unwrap_or_else(|e| format!("Error: {}", e));

        // 5. Test Encryption Key AES-256-GCM Roundtrip
        let encryption_key_status = match get_or_create_app_key() {
            Ok(key) => {
                let test_text = "OlahPOS_Healthcheck_2026";
                match crypto::encrypt_string(test_text, &key) {
                    Ok(enc) => match crypto::decrypt_string(&enc, &key) {
                        Ok(dec) if dec == test_text => "VALID_AES256_GCM".to_string(),
                        _ => "DECRYPTION_FAILED".to_string(),
                    },
                    Err(_) => "ENCRYPTION_FAILED".to_string(),
                }
            }
            Err(e) => format!("KEY_LOAD_ERROR: {}", e),
        };

        // 6. Audit Tables & Counts
        let mut audit_tables_count = 0;
        let mut audit_records_count = 0;
        if dual_db_attached {
            let mut tbl_stmt = conn.prepare(
                "SELECT name FROM log_db.sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%';",
            )?;
            let tables: Vec<String> = tbl_stmt
                .query_map([], |r| r.get(0))?
                .filter_map(|r| r.ok())
                .collect();
            audit_tables_count = tables.len();

            for tbl in &tables {
                if AuditLogger::validate_identifier(tbl).is_ok() {
                    let count_sql = format!("SELECT COUNT(*) FROM log_db.{};", tbl);
                    let cnt: i64 = conn.query_row(&count_sql, [], |r| r.get(0)).unwrap_or(0);
                    audit_records_count += cnt;
                }
            }
        }

        let uptime_seconds = START_TIME.elapsed().as_secs();
        let os_info = format!("{} {}", std::env::consts::OS, std::env::consts::ARCH);

        Ok(SystemDiagnostics {
            app_name: "OlahPOS".to_string(),
            app_version: "0.1.0".to_string(),
            os_info,
            main_db_path: paths.main_db.to_string_lossy().to_string(),
            main_db_size_bytes: main_size,
            log_db_path: paths.log_db.to_string_lossy().to_string(),
            log_db_size_bytes: log_size,
            sqlite_version: sqlite_ver,
            journal_mode: journal_mode.to_uppercase(),
            dual_db_attached,
            db_integrity_status,
            encryption_key_status,
            uptime_seconds,
            total_audit_tables_count: audit_tables_count,
            total_audit_records_count: audit_records_count,
        })
    }

    /// Ambil daftar tabel yang ada di log_db
    pub fn get_audit_tables(db: &DbConnection) -> Result<Vec<String>> {
        let conn = db.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT name FROM log_db.sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name ASC;",
        )?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        let mut tables = Vec::new();
        for r in rows {
            tables.push(r?);
        }
        Ok(tables)
    }

    /// Ambil daftar data log dari tabel tertentu dengan filter
    pub fn get_audit_logs(
        db: &DbConnection,
        table_name: &str,
        limit: i64,
        action: Option<&str>,
        search: Option<&str>,
    ) -> Result<Vec<AuditLogRow>> {
        AuditLogger::validate_identifier(table_name)?;
        let conn = db.lock().unwrap();

        let mut conditions = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(act) = action {
            if act != "ALL" && !act.is_empty() {
                conditions.push("aksi = ?");
                params.push(Box::new(act.to_string()));
            }
        }

        if let Some(s) = search {
            if !s.trim().is_empty() {
                conditions.push("(primary_key_id LIKE ? OR content LIKE ?)");
                let pattern = format!("%{}%", s.trim());
                params.push(Box::new(pattern.clone()));
                params.push(Box::new(pattern));
            }
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let sql = format!(
            "SELECT log_id, created_at, created_by, primary_key_id, aksi, content 
             FROM log_db.{} 
             {} 
             ORDER BY log_id DESC 
             LIMIT {};",
            table_name, where_clause, limit
        );

        let mut stmt = conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

        let rows = stmt.query_map(param_refs.as_slice(), |r| {
            let log_id: i64 = r.get(0)?;
            let created_at: String = r.get(1)?;
            let created_by: Option<i64> = r.get(2)?;
            let primary_key_id: String = r.get(3)?;
            let aksi: String = r.get(4)?;
            let content_str: String = r.get(5)?;
            let content: serde_json::Value =
                serde_json::from_str(&content_str).unwrap_or(json!({ "raw": content_str }));

            Ok(AuditLogRow {
                log_id,
                table_name: table_name.to_string(),
                created_at,
                created_by,
                primary_key_id,
                aksi,
                content,
            })
        })?;

        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }

    /// Eksekusi SQL query aman (hanya SELECT, PRAGMA, EXPLAIN)
    pub fn execute_readonly_query(db: &DbConnection, sql: &str) -> std::result::Result<QueryExecutionResult, String> {
        let trimmed = sql.trim();
        let upper = trimmed.to_uppercase();

        let is_allowed = upper.starts_with("SELECT")
            || upper.starts_with("PRAGMA")
            || upper.starts_with("EXPLAIN")
            || upper.starts_with("WITH");

        if !is_allowed {
            return Err("Keamanan: Konsol developer hanya mengizinkan query read-only (SELECT, PRAGMA, EXPLAIN). Query yang mengubah data ditolak.".to_string());
        }

        let conn = db.lock().unwrap();
        let start = Instant::now();

        let mut stmt = conn.prepare(trimmed).map_err(|e| format!("SQL Syntax Error: {}", e))?;
        let col_count = stmt.column_count();
        let columns: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();

        let mut rows: Vec<Vec<serde_json::Value>> = Vec::new();
        let mut query_rows = stmt.query([]).map_err(|e| format!("Query Execution Error: {}", e))?;

        while let Some(row) = query_rows.next().map_err(|e| format!("Row Fetch Error: {}", e))? {
            let mut row_values = Vec::new();
            for i in 0..col_count {
                let val_ref = row.get_ref(i).unwrap();
                let json_val = match val_ref {
                    rusqlite::types::ValueRef::Null => serde_json::Value::Null,
                    rusqlite::types::ValueRef::Integer(v) => json!(v),
                    rusqlite::types::ValueRef::Real(v) => json!(v),
                    rusqlite::types::ValueRef::Text(v) => {
                        let s = String::from_utf8_lossy(v).to_string();
                        json!(s)
                    }
                    rusqlite::types::ValueRef::Blob(v) => {
                        json!(format!("<BLOB {} bytes>", v.len()))
                    }
                };
                row_values.push(json_val);
            }
            rows.push(row_values);
            if rows.len() >= 200 {
                break; // Maksimal 200 baris untuk keamanan UI memory
            }
        }

        let execution_time_ms = start.elapsed().as_millis();
        let row_count = rows.len();

        Ok(QueryExecutionResult {
            columns,
            rows,
            row_count,
            execution_time_ms,
        })
    }

    /// Menghasilkan berkas diagnostik lengkap untuk diunduh developer
    pub fn generate_diagnostic_log_export(db: &DbConnection) -> String {
        let diag = Self::get_system_diagnostics(db).ok();
        let runtime_logs = Self::get_runtime_logs(200, None);
        let audit_tables = Self::get_audit_tables(db).unwrap_or_default();

        let mut out = String::new();
        out.push_str("================================================================================\n");
        out.push_str("                        OLAHPOS SYSTEM DIAGNOSTIC REPORT                        \n");
        out.push_str(&format!("Generated At : {}\n", chrono::Utc::now().to_rfc3339()));
        out.push_str("================================================================================\n\n");

        if let Some(d) = diag {
            out.push_str("--- [1. SYSTEM & ENVIRONMENT] ---\n");
            out.push_str(&format!("App Name        : {}\n", d.app_name));
            out.push_str(&format!("App Version     : {}\n", d.app_version));
            out.push_str(&format!("OS Architecture : {}\n", d.os_info));
            out.push_str(&format!("SQLite Version  : {}\n", d.sqlite_version));
            out.push_str(&format!("Uptime Seconds  : {} s\n\n", d.uptime_seconds));

            out.push_str("--- [2. DATABASE HEALTH & INTEGRITY] ---\n");
            out.push_str(&format!("Main DB Path    : {}\n", d.main_db_path));
            out.push_str(&format!("Main DB Size    : {} bytes\n", d.main_db_size_bytes));
            out.push_str(&format!("Log DB Path     : {}\n", d.log_db_path));
            out.push_str(&format!("Log DB Size     : {} bytes\n", d.log_db_size_bytes));
            out.push_str(&format!("Dual DB Attached: {}\n", d.dual_db_attached));
            out.push_str(&format!("Journal Mode    : {}\n", d.journal_mode));
            out.push_str(&format!("Integrity Status: {}\n", d.db_integrity_status));
            out.push_str(&format!("Encryption Key  : {}\n", d.encryption_key_status));
            out.push_str(&format!("Audit Tables    : {}\n", d.total_audit_tables_count));
            out.push_str(&format!("Total Audit Logs: {}\n\n", d.total_audit_records_count));
        }

        out.push_str("--- [3. DETECTED AUDIT LOG TABLES] ---\n");
        for tbl in audit_tables {
            out.push_str(&format!("- log_db.{}\n", tbl));
        }
        out.push_str("\n");

        out.push_str("--- [4. RUNTIME SYSTEM LOGS (LAST 200 EVENTS)] ---\n");
        if runtime_logs.is_empty() {
            out.push_str("No runtime error or warning logs recorded.\n");
        } else {
            for log in runtime_logs {
                out.push_str(&format!(
                    "[{}] [{:5}] [{}] {}\n",
                    log.timestamp, log.level, log.source, log.message
                ));
            }
        }
        out.push_str("\n=========================== END OF DIAGNOSTIC LOG ===========================\n");

        out
    }

    // ==========================================
    // MANAJEMEN PENGGUNA & HAK AKSES
    // ==========================================

    /// Ambil daftar semua pengguna sistem
    pub fn list_all_users(db: &DbConnection) -> Result<Vec<UserAdminItem>> {
        let conn = db.lock().unwrap();
        let sql = "
            SELECT 
                pengguna_id, nama_lengkap, nama_pengguna, peran, 
                active_st, harus_ganti_password, terakhir_login, created_at 
            FROM mst_pengguna 
            WHERE deleted_st = 0 
            ORDER BY pengguna_id ASC;
        ";

        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map([], |r| {
            let harus_ganti: i64 = r.get(5)?;
            Ok(UserAdminItem {
                pengguna_id: r.get(0)?,
                nama_lengkap: r.get(1)?,
                nama_pengguna: r.get(2)?,
                peran: r.get(3)?,
                active_st: r.get(4)?,
                harus_ganti_password: harus_ganti == 1,
                terakhir_login: r.get(6)?,
                created_at: r.get(7)?,
            })
        })?;

        let mut users = Vec::new();
        for r in rows {
            users.push(r?);
        }
        Ok(users)
    }

    /// Tambah pengguna baru oleh Developer / Admin
    pub fn create_system_user(
        db: &DbConnection,
        input: CreateUserInput,
        admin_id: i64,
    ) -> std::result::Result<i64, String> {
        let username_clean = input.nama_pengguna.trim().to_lowercase();
        if username_clean.len() < 3 {
            return Err("Nama pengguna minimal 3 karakter".to_string());
        }

        if input.password.len() < 6 {
            return Err("Kata sandi minimal 6 karakter".to_string());
        }

        let peran_upper = input.peran.trim().to_uppercase();
        if peran_upper != "ADMIN" && peran_upper != "CASHIER" {
            return Err("Peran hanya boleh ADMIN atau CASHIER".to_string());
        }

        let hash = AuthService::hash_password(&input.password)
            .map_err(|e| format!("Gagal melakukan hash password: {}", e))?;

        let mut conn = db.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();

        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM mst_pengguna WHERE nama_pengguna = ?1 AND deleted_st = 0;",
                [&username_clean],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;

        if exists > 0 {
            return Err("Nama pengguna sudah digunakan".to_string());
        }

        let tx = conn.transaction().map_err(|e| e.to_string())?;

        tx.execute(
            "INSERT INTO mst_pengguna (
                created_at, created_by, nama_lengkap, nama_pengguna, 
                password_hash, peran, harus_ganti_password, active_st
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, 1);",
            rusqlite::params![
                now,
                admin_id,
                input.nama_lengkap.trim(),
                username_clean,
                hash,
                peran_upper
            ],
        )
        .map_err(|e| format!("Gagal menyimpan pengguna: {}", e))?;

        let new_user_id = tx.last_insert_rowid();

        AuditLogger::record(
            &tx,
            "mst_pengguna",
            &new_user_id.to_string(),
            AuditAction::Insert,
            &json!({
                "action": "CREATE_USER_BY_DEVELOPER",
                "nama_pengguna": username_clean,
                "peran": peran_upper,
                "created_by": admin_id
            }),
            Some(admin_id),
        )
        .map_err(|e| e.to_string())?;

        tx.commit().map_err(|e| e.to_string())?;
        Self::record_runtime_log(
            "INFO",
            "DeveloperService",
            &format!("Pengguna baru '{}' ({}) berhasil dibuat", username_clean, peran_upper),
        );

        Ok(new_user_id)
    }

    /// Reset kata sandi pengguna oleh Developer
    pub fn reset_user_password(
        db: &DbConnection,
        user_id: i64,
        new_password: &str,
        admin_id: i64,
    ) -> std::result::Result<(), String> {
        if new_password.trim().len() < 6 {
            return Err("Kata sandi baru minimal 6 karakter".to_string());
        }

        let hash = AuthService::hash_password(new_password)
            .map_err(|e| format!("Gagal hash password: {}", e))?;

        let mut conn = db.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        let tx = conn.transaction().map_err(|e| e.to_string())?;

        let affected = tx
            .execute(
                "UPDATE mst_pengguna 
                 SET password_hash = ?1, harus_ganti_password = 1, updated_at = ?2, updated_by = ?3 
                 WHERE pengguna_id = ?4 AND deleted_st = 0;",
                rusqlite::params![hash, now, admin_id, user_id],
            )
            .map_err(|e| format!("Gagal reset password: {}", e))?;

        if affected == 0 {
            return Err("Pengguna tidak ditemukan".to_string());
        }

        AuditLogger::record(
            &tx,
            "mst_pengguna",
            &user_id.to_string(),
            AuditAction::Update,
            &json!({
                "action": "RESET_PASSWORD_BY_DEVELOPER",
                "target_user_id": user_id,
                "reset_by": admin_id
            }),
            Some(admin_id),
        )
        .map_err(|e| e.to_string())?;

        tx.commit().map_err(|e| e.to_string())?;
        Self::record_runtime_log(
            "WARN",
            "DeveloperService",
            &format!("Password untuk user ID {} direset oleh admin {}", user_id, admin_id),
        );

        Ok(())
    }

    /// Toggle status aktif/nonaktif akun pengguna
    pub fn toggle_user_status(
        db: &DbConnection,
        user_id: i64,
        active_st: i64,
        admin_id: i64,
    ) -> std::result::Result<(), String> {
        if user_id == admin_id && active_st == 0 {
            return Err("Anda tidak dapat menonaktifkan akun Anda sendiri".to_string());
        }

        let mut conn = db.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        let tx = conn.transaction().map_err(|e| e.to_string())?;

        let affected = tx
            .execute(
                "UPDATE mst_pengguna 
                 SET active_st = ?1, updated_at = ?2, updated_by = ?3 
                 WHERE pengguna_id = ?4 AND deleted_st = 0;",
                rusqlite::params![active_st, now, admin_id, user_id],
            )
            .map_err(|e| format!("Gagal update status pengguna: {}", e))?;

        if affected == 0 {
            return Err("Pengguna tidak ditemukan".to_string());
        }

        AuditLogger::record(
            &tx,
            "mst_pengguna",
            &user_id.to_string(),
            AuditAction::Update,
            &json!({
                "action": "TOGGLE_USER_STATUS",
                "target_user_id": user_id,
                "active_st": active_st,
                "updated_by": admin_id
            }),
            Some(admin_id),
        )
        .map_err(|e| e.to_string())?;

        tx.commit().map_err(|e| e.to_string())?;
        Self::record_runtime_log(
            "INFO",
            "DeveloperService",
            &format!("Status user ID {} diubah menjadi active_st={}", user_id, active_st),
        );

        Ok(())
    }

    /// Ubah peran pengguna (ADMIN / CASHIER)
    pub fn update_user_role(
        db: &DbConnection,
        user_id: i64,
        peran: &str,
        admin_id: i64,
    ) -> std::result::Result<(), String> {
        let peran_upper = peran.trim().to_uppercase();
        if peran_upper != "ADMIN" && peran_upper != "CASHIER" {
            return Err("Peran hanya boleh ADMIN atau CASHIER".to_string());
        }

        let mut conn = db.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        let tx = conn.transaction().map_err(|e| e.to_string())?;

        let affected = tx
            .execute(
                "UPDATE mst_pengguna 
                 SET peran = ?1, updated_at = ?2, updated_by = ?3 
                 WHERE pengguna_id = ?4 AND deleted_st = 0;",
                rusqlite::params![peran_upper, now, admin_id, user_id],
            )
            .map_err(|e| format!("Gagal ubah peran pengguna: {}", e))?;

        if affected == 0 {
            return Err("Pengguna tidak ditemukan".to_string());
        }

        AuditLogger::record(
            &tx,
            "mst_pengguna",
            &user_id.to_string(),
            AuditAction::Update,
            &json!({
                "action": "UPDATE_USER_ROLE",
                "target_user_id": user_id,
                "new_role": peran_upper,
                "updated_by": admin_id
            }),
            Some(admin_id),
        )
        .map_err(|e| e.to_string())?;

        tx.commit().map_err(|e| e.to_string())?;
        Self::record_runtime_log(
            "INFO",
            "DeveloperService",
            &format!("Peran user ID {} diubah menjadi {}", user_id, peran_upper),
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::connection::DbManager;
    use crate::database::migrator::Migrator;

    fn setup_test_db() -> (DbManager, DbConnection) {
        let db_manager = DbManager::init_in_memory().unwrap();
        let conn_arc = db_manager.get_connection();
        Migrator::run(&mut conn_arc.lock().unwrap()).unwrap();
        AuthService::bootstrap_developer_if_needed(&conn_arc).unwrap();
        (db_manager, conn_arc)
    }

    #[test]
    fn test_system_diagnostics_and_runtime_logs() {
        let (_manager, conn_arc) = setup_test_db();

        DeveloperService::record_runtime_log("INFO", "UnitTest", "Testing info log");
        DeveloperService::record_runtime_log("ERROR", "UnitTest", "Testing error log");

        let logs = DeveloperService::get_runtime_logs(10, None);
        assert!(!logs.is_empty());
        assert_eq!(logs[0].level, "ERROR");

        let diag = DeveloperService::get_system_diagnostics(&conn_arc).unwrap();
        assert_eq!(diag.app_name, "OlahPOS");
        assert!(diag.journal_mode == "WAL" || diag.journal_mode == "MEMORY");
        assert_eq!(diag.db_integrity_status, "ok");
    }

    #[test]
    fn test_execute_readonly_query() {
        let (_manager, conn_arc) = setup_test_db();

        // 1. SELECT query valid
        let res = DeveloperService::execute_readonly_query(&conn_arc, "SELECT nama_lengkap, peran FROM mst_pengguna;").unwrap();
        assert!(!res.columns.is_empty());
        assert!(res.row_count >= 1);

        // 2. PRAGMA valid
        let pragma_res = DeveloperService::execute_readonly_query(&conn_arc, "PRAGMA journal_mode;").unwrap();
        assert_eq!(pragma_res.row_count, 1);

        // 3. Mutating queries must be rejected
        let drop_res = DeveloperService::execute_readonly_query(&conn_arc, "DROP TABLE mst_pengguna;");
        assert!(drop_res.is_err());

        let delete_res = DeveloperService::execute_readonly_query(&conn_arc, "DELETE FROM mst_pengguna;");
        assert!(delete_res.is_err());
    }

    #[test]
    fn test_user_management_lifecycle() {
        let (_manager, conn_arc) = setup_test_db();

        // 1. List users (should have developer)
        let users = DeveloperService::list_all_users(&conn_arc).unwrap();
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].nama_pengguna, "developer");

        // 2. Create new cashier user
        let new_user = CreateUserInput {
            nama_lengkap: "Kasir Toko 1".to_string(),
            nama_pengguna: "kasir1".to_string(),
            password: "kasirPassword123".to_string(),
            peran: "CASHIER".to_string(),
        };
        let new_id = DeveloperService::create_system_user(&conn_arc, new_user, 1).unwrap();
        assert!(new_id > 1);

        // 3. Check list again
        let users_after = DeveloperService::list_all_users(&conn_arc).unwrap();
        assert_eq!(users_after.len(), 2);

        // 4. Reset password
        DeveloperService::reset_user_password(&conn_arc, new_id, "NewKasirPass456", 1).unwrap();

        // 5. Toggle active status
        DeveloperService::toggle_user_status(&conn_arc, new_id, 0, 1).unwrap();
        let users_toggled = DeveloperService::list_all_users(&conn_arc).unwrap();
        let kasir = users_toggled.iter().find(|u| u.pengguna_id == new_id).unwrap();
        assert_eq!(kasir.active_st, 0);

        // 6. Update role
        DeveloperService::update_user_role(&conn_arc, new_id, "ADMIN", 1).unwrap();
        let users_role = DeveloperService::list_all_users(&conn_arc).unwrap();
        let kasir_admin = users_role.iter().find(|u| u.pengguna_id == new_id).unwrap();
        assert_eq!(kasir_admin.peran, "ADMIN");
    }

    #[test]
    fn test_generate_diagnostic_log_export() {
        let (_manager, conn_arc) = setup_test_db();
        DeveloperService::record_runtime_log("WARN", "TestModule", "Sample warning");
        let export_text = DeveloperService::generate_diagnostic_log_export(&conn_arc);
        assert!(export_text.contains("OLAHPOS SYSTEM DIAGNOSTIC REPORT"));
        assert!(export_text.contains("DATABASE HEALTH & INTEGRITY"));
        assert!(export_text.contains("Sample warning"));
    }
}
