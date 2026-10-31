use crate::services::crypto::get_app_data_dir;
use rusqlite::{Connection, Result};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub type DbConnection = Arc<Mutex<Connection>>;

#[derive(Debug, Clone)]
pub struct DatabasePaths {
    pub main_db: PathBuf,
    pub log_db: PathBuf,
}

impl DatabasePaths {
    pub fn default() -> std::io::Result<Self> {
        let app_dir = get_app_data_dir()?;
        Ok(Self {
            main_db: app_dir.join("kasir_pos.db"),
            log_db: app_dir.join("kasir_pos_log.db"),
        })
    }
}

pub struct DbManager {
    conn: DbConnection,
    paths: DatabasePaths,
}

impl DbManager {
    /// Initializes database connection with dual-DB attach configuration
    pub fn init(paths: DatabasePaths) -> Result<Self> {
        let conn = Connection::open(&paths.main_db)?;

        // Configure WAL mode & pragmas for performance and data safety
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;
             PRAGMA busy_timeout = 5000;",
        )?;

        // Attach audit log database atomically
        let log_path_str = paths.log_db.to_string_lossy();
        conn.execute("ATTACH DATABASE ?1 AS log_db;", [&log_path_str])?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            paths,
        })
    }

    /// Initializes in-memory dual databases for testing
    pub fn init_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             ATTACH DATABASE ':memory:' AS log_db;",
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            paths: DatabasePaths {
                main_db: PathBuf::from(":memory:"),
                log_db: PathBuf::from(":memory:"),
            },
        })
    }

    pub fn get_connection(&self) -> DbConnection {
        Arc::clone(&self.conn)
    }

    pub fn get_paths(&self) -> &DatabasePaths {
        &self.paths
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dual_db_attach_and_atomic_transaction() {
        let db_manager = DbManager::init_in_memory().expect("Failed to init in-memory dual DB");
        let conn_arc = db_manager.get_connection();
        let mut conn = conn_arc.lock().unwrap();

        // Create a test operational table in main DB
        conn.execute(
            "CREATE TABLE dat_test_penjualan (
                penjualan_id INTEGER PRIMARY KEY AUTOINCREMENT,
                nomor_struk TEXT NOT NULL,
                total INTEGER NOT NULL
            );",
            [],
        )
        .unwrap();

        // Create a test audit log table in log_db
        conn.execute(
            "CREATE TABLE log_db.dat_test_penjualan (
                log_id INTEGER PRIMARY KEY AUTOINCREMENT,
                primary_key_id TEXT NOT NULL,
                aksi TEXT NOT NULL,
                content TEXT NOT NULL
            );",
            [],
        )
        .unwrap();

        // Perform atomic transaction writing to BOTH databases
        {
            let tx = conn.transaction().unwrap();
            tx.execute(
                "INSERT INTO dat_test_penjualan (nomor_struk, total) VALUES (?1, ?2);",
                rusqlite::params!["TRX-001", 50000],
            )
            .unwrap();

            tx.execute(
                "INSERT INTO log_db.dat_test_penjualan (primary_key_id, aksi, content) VALUES (?1, ?2, ?3);",
                rusqlite::params!["1", "INSERT", r#"{"total":50000}"#],
            )
            .unwrap();

            tx.commit().unwrap();
        }

        // Verify record in main database
        let count_main: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM dat_test_penjualan WHERE nomor_struk = 'TRX-001';",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count_main, 1);

        // Verify record in log database
        let count_log: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM log_db.dat_test_penjualan WHERE primary_key_id = '1';",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count_log, 1);
    }

    #[test]
    fn test_dual_db_rollback_atomicity() {
        let db_manager = DbManager::init_in_memory().expect("Failed to init in-memory dual DB");
        let conn_arc = db_manager.get_connection();
        let mut conn = conn_arc.lock().unwrap();

        conn.execute("CREATE TABLE dat_produk (id INTEGER PRIMARY KEY, nama TEXT);", [])
            .unwrap();
        conn.execute("CREATE TABLE log_db.dat_produk_log (id INTEGER PRIMARY KEY, note TEXT);", [])
            .unwrap();

        // Simulate failed transaction
        let tx_result = (|| -> Result<()> {
            let tx = conn.transaction()?;
            tx.execute("INSERT INTO dat_produk VALUES (1, 'Barang A');", [])?;
            tx.execute("INSERT INTO log_db.dat_produk_log VALUES (1, 'Log A');", [])?;
            // Force error
            tx.execute("INSERT INTO dat_nonexistent_table VALUES (1);", [])?;
            tx.commit()?;
            Ok(())
        })();

        assert!(tx_result.is_err());

        // Verify rollback happened in BOTH databases
        let main_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM dat_produk;", [], |r| r.get(0))
            .unwrap();
        let log_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM log_db.dat_produk_log;", [], |r| {
                r.get(0)
            })
            .unwrap();

        assert_eq!(main_count, 0, "Main DB must be rolled back");
        assert_eq!(log_count, 0, "Log DB must be rolled back");
    }
}
