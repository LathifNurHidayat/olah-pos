use rusqlite::{Connection, Result, Transaction};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditAction {
    #[serde(rename = "INSERT")]
    Insert,
    #[serde(rename = "UPDATE")]
    Update,
    #[serde(rename = "DELETE")]
    Delete,
    #[serde(rename = "RESTORE")]
    Restore,
}

impl AuditAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuditAction::Insert => "INSERT",
            AuditAction::Update => "UPDATE",
            AuditAction::Delete => "DELETE",
            AuditAction::Restore => "RESTORE",
        }
    }
}

pub struct AuditLogger;

impl AuditLogger {
    /// Validates table name to prevent SQL injection in dynamic DDL/DML statements
    pub fn validate_identifier(name: &str) -> Result<(), rusqlite::Error> {
        let is_valid = !name.is_empty()
            && (name.starts_with("mst_") || name.starts_with("dat_"))
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_');

        if !is_valid {
            return Err(rusqlite::Error::InvalidQuery);
        }
        Ok(())
    }

    /// Automatically ensures dynamic audit log table exists in log_db
    pub fn ensure_log_table(conn: &Connection, table_name: &str) -> Result<()> {
        Self::validate_identifier(table_name)?;

        let ddl = format!(
            "CREATE TABLE IF NOT EXISTS log_db.{table_name} (
                created_at TEXT NOT NULL,
                created_by INTEGER,
                updated_at TEXT,
                updated_by INTEGER,
                deleted_at TEXT,
                deleted_by INTEGER,
                deleted_st INTEGER DEFAULT 0,
                active_st INTEGER DEFAULT 1,
                log_id INTEGER PRIMARY KEY AUTOINCREMENT,
                primary_key_id TEXT NOT NULL,
                aksi TEXT NOT NULL CHECK(aksi IN ('INSERT', 'UPDATE', 'DELETE', 'RESTORE')),
                content TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS log_db.idx_{table_name}_pk 
                ON {table_name}(primary_key_id);
            CREATE INDEX IF NOT EXISTS log_db.idx_{table_name}_created 
                ON {table_name}(created_at);"
        );

        conn.execute_batch(&ddl)?;
        Ok(())
    }

    /// Records an audit log entry in log_db within an active transaction
    pub fn record(
        tx: &Transaction,
        table_name: &str,
        primary_key_id: &str,
        action: AuditAction,
        content: &serde_json::Value,
        user_id: Option<i64>,
    ) -> Result<()> {
        Self::validate_identifier(table_name)?;

        // Ensure log table exists before inserting
        Self::ensure_log_table(tx, table_name)?;

        let now = chrono::Utc::now().to_rfc3339();
        let content_str = serde_json::to_string(content)
            .map_err(|_| rusqlite::Error::InvalidQuery)?;

        let query = format!(
            "INSERT INTO log_db.{table_name} (
                created_at, created_by, primary_key_id, aksi, content
            ) VALUES (?1, ?2, ?3, ?4, ?5);"
        );

        tx.execute(
            &query,
            rusqlite::params![now, user_id, primary_key_id, action.as_str(), content_str],
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::connection::DbManager;
    use serde_json::json;

    #[test]
    fn test_dynamic_log_table_creation_and_insertion() {
        let db_manager = DbManager::init_in_memory().unwrap();
        let conn_arc = db_manager.get_connection();
        let mut conn = conn_arc.lock().unwrap();

        // 1. Create source table
        conn.execute(
            "CREATE TABLE mst_barang (
                barang_id INTEGER PRIMARY KEY AUTOINCREMENT,
                nama_barang TEXT NOT NULL,
                harga INTEGER NOT NULL
            );",
            [],
        )
        .unwrap();

        // 2. Perform INSERT and record audit log
        {
            let tx = conn.transaction().unwrap();
            tx.execute(
                "INSERT INTO mst_barang (nama_barang, harga) VALUES ('Beras', 60000);",
                [],
            )
            .unwrap();
            let new_id = tx.last_insert_rowid().to_string();

            AuditLogger::record(
                &tx,
                "mst_barang",
                &new_id,
                AuditAction::Insert,
                &json!({"nama_barang": "Beras", "harga": 60000}),
                Some(1),
            )
            .unwrap();

            tx.commit().unwrap();
        }

        // 3. Verify log table was dynamically created in log_db
        let count_log: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM log_db.mst_barang WHERE primary_key_id = '1' AND aksi = 'INSERT';",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count_log, 1);

        // 4. Perform UPDATE and record before/after
        {
            let tx = conn.transaction().unwrap();
            tx.execute(
                "UPDATE mst_barang SET harga = 65000 WHERE barang_id = 1;",
                [],
            )
            .unwrap();

            AuditLogger::record(
                &tx,
                "mst_barang",
                "1",
                AuditAction::Update,
                &json!({
                    "before": {"nama_barang": "Beras", "harga": 60000},
                    "after": {"nama_barang": "Beras", "harga": 65000}
                }),
                Some(1),
            )
            .unwrap();

            tx.commit().unwrap();
        }

        let update_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM log_db.mst_barang WHERE primary_key_id = '1' AND aksi = 'UPDATE';",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(update_count, 1);
    }

    #[test]
    fn test_table_name_validation() {
        assert!(AuditLogger::validate_identifier("mst_barang").is_ok());
        assert!(AuditLogger::validate_identifier("dat_penjualan").is_ok());

        // Invalid identifiers must be rejected
        assert!(AuditLogger::validate_identifier("users; DROP TABLE mst_barang;").is_err());
        assert!(AuditLogger::validate_identifier("random_table").is_err());
        assert!(AuditLogger::validate_identifier("mst_barang'--").is_err());
    }
}
