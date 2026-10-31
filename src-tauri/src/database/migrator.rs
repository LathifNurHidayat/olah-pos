use rusqlite::{Connection, Result};

pub struct Migration {
    pub version: i32,
    pub name: &'static str,
    pub sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "001_initial_schema",
    sql: include_str!("migrations/001_initial_schema.sql"),
}];

pub struct Migrator;

impl Migrator {
    /// Runs all pending database migrations in sequential order
    pub fn run(conn: &mut Connection) -> Result<()> {
        // Ensure migration tracking table exists
        conn.execute(
            "CREATE TABLE IF NOT EXISTS _migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                applied_at TEXT NOT NULL
            );",
            [],
        )?;

        for m in MIGRATIONS {
            let already_applied: bool = conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM _migrations WHERE version = ?1);",
                [m.version],
                |row| row.get(0),
            )?;

            if !already_applied {
                log::info!("Applying migration {} ({})", m.version, m.name);
                let tx = conn.transaction()?;
                tx.execute_batch(m.sql)?;
                tx.execute(
                    "INSERT INTO _migrations (version, name, applied_at) VALUES (?1, ?2, ?3);",
                    rusqlite::params![m.version, m.name, chrono::Utc::now().to_rfc3339()],
                )?;
                tx.commit()?;
                log::info!("Migration {} applied successfully", m.version);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migrator_runs_and_populates_base_tables() {
        let mut conn = Connection::open_in_memory().unwrap();
        Migrator::run(&mut conn).expect("Migration failed");

        // Verify migration tracker
        let version: i32 = conn
            .query_row(
                "SELECT version FROM _migrations WHERE version = 1;",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(version, 1);

        // Verify master tables exist and default seeds populated
        let satuan_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM mst_satuan_barang;", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert!(satuan_count >= 5);

        let kategori_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM mst_kategori_barang;", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert!(kategori_count >= 5);

        let mutasi_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM mst_jenis_mutasi;", [], |r| r.get(0))
            .unwrap();
        assert_eq!(mutasi_count, 7);

        // Verify transaction tables exist
        conn.execute(
            "INSERT INTO dat_integrasi_config (
                created_at, created_by, kode_layanan, nama_layanan, payload_terenkripsi
            ) VALUES ('2026-01-01T00:00:00Z', 1, 'MIDTRANS', 'Midtrans Gateway', 'ciphertext_dummy');",
            [],
        )
        .unwrap();

        let config_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM dat_integrasi_config;", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(config_count, 1);
    }

    #[test]
    fn test_migrator_idempotency() {
        let mut conn = Connection::open_in_memory().unwrap();
        Migrator::run(&mut conn).unwrap();
        // Second run should be a no-op and succeed without error
        Migrator::run(&mut conn).unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM _migrations;", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }
}
