use crate::database::audit::{AuditAction, AuditLogger};
use crate::database::connection::DbConnection;
use rusqlite::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryItem {
    pub barang_id: i64,
    pub kode_barang: String,
    pub barcode: Option<String>,
    pub nama_barang: String,
    pub kategori_id: i64,
    pub nama_kategori: String,
    pub satuan_id: i64,
    pub nama_satuan: String,
    pub harga_beli: i64,
    pub harga_jual: i64,
    pub stok_saat_ini: i64,
    pub stok_minimum: i64,
    pub status_stok: String, // "AMAN", "MENIPIS", "HABIS"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockMutationItem {
    pub mutasi_id: i64,
    pub barang_id: i64,
    pub kode_barang: String,
    pub nama_barang: String,
    pub jenis_mutasi_id: i64,
    pub kode_mutasi: String,
    pub nama_mutasi: String,
    pub tanggal_waktu: String,
    pub stok_masuk: i64,
    pub stok_keluar: i64,
    pub stok_sebelum: i64,
    pub stok_sesudah: i64,
    pub referensi_tabel: Option<String>,
    pub referensi_id: Option<i64>,
    pub keterangan: Option<String>,
    pub created_at: String,
    pub created_by: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordMutationInput {
    pub barang_id: i64,
    pub jenis_mutasi_id: i64,
    pub stok_masuk: i64,
    pub stok_keluar: i64,
    pub referensi_tabel: Option<String>,
    pub referensi_id: Option<i64>,
    pub keterangan: Option<String>,
    pub user_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockOpnameInput {
    pub barang_id: i64,
    pub stok_fisik: i64,
    pub keterangan: Option<String>,
    pub user_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryStats {
    pub total_item: i64,
    pub total_stok_fisik: i64,
    pub stok_menipis_count: i64,
    pub stok_habis_count: i64,
    pub estimasi_nilai_aset: i64,
}

pub struct InventoryService;

impl InventoryService {
    /// Catat mutasi stok dan sesuaikan stok_saat_ini barang secara atomik di dalam transaksi yang sedang aktif.
    /// Memvalidasi agar stok tidak pernah negatif.
    pub fn record_mutation(
        tx: &rusqlite::Transaction,
        input: &RecordMutationInput,
    ) -> std::result::Result<i64, String> {
        let now = chrono::Utc::now().to_rfc3339();

        // 1. Ambil stok saat ini dari mst_barang
        let stok_sebelum: i64 = tx
            .query_row(
                "SELECT stok_saat_ini FROM mst_barang WHERE barang_id = ?1 AND active_st = 1 AND deleted_st = 0;",
                [input.barang_id],
                |row| row.get(0),
            )
            .map_err(|e| format!("Barang id {} tidak ditemukan: {}", input.barang_id, e))?;

        // 2. Hitung stok sesudah
        let stok_sesudah = stok_sebelum + input.stok_masuk - input.stok_keluar;
        if stok_sesudah < 0 {
            return Err(format!(
                "Stok barang tidak mencukupi! Stok saat ini: {}, pengurangan yang diminta: {}",
                stok_sebelum, input.stok_keluar
            ));
        }

        // 3. Update stok di mst_barang
        tx.execute(
            "UPDATE mst_barang 
             SET stok_saat_ini = ?1, updated_at = ?2, updated_by = ?3 
             WHERE barang_id = ?4;",
            rusqlite::params![stok_sesudah, now, input.user_id, input.barang_id],
        )
        .map_err(|e| format!("Gagal update stok di mst_barang: {}", e))?;

        // 4. Masukkan mutasi ke dat_mutasi_stok
        tx.execute(
            "INSERT INTO dat_mutasi_stok (
                created_at, created_by, barang_id, jenis_mutasi_id, 
                tanggal_waktu, stok_masuk, stok_keluar, 
                stok_sebelum, stok_sesudah, referensi_tabel, 
                referensi_id, keterangan
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12);",
            rusqlite::params![
                now,
                input.user_id,
                input.barang_id,
                input.jenis_mutasi_id,
                now,
                input.stok_masuk,
                input.stok_keluar,
                stok_sebelum,
                stok_sesudah,
                input.referensi_tabel,
                input.referensi_id,
                input.keterangan,
            ],
        )
        .map_err(|e| format!("Gagal mencatat dat_mutasi_stok: {}", e))?;

        let mutasi_id = tx.last_insert_rowid();

        // 5. Catat audit log untuk dat_mutasi_stok
        AuditLogger::record(
            tx,
            "dat_mutasi_stok",
            &mutasi_id.to_string(),
            AuditAction::Insert,
            &json!({
                "mutasi_id": mutasi_id,
                "barang_id": input.barang_id,
                "jenis_mutasi_id": input.jenis_mutasi_id,
                "stok_masuk": input.stok_masuk,
                "stok_keluar": input.stok_keluar,
                "stok_sebelum": stok_sebelum,
                "stok_sesudah": stok_sesudah,
                "referensi_tabel": input.referensi_tabel,
                "referensi_id": input.referensi_id,
                "keterangan": input.keterangan
            }),
            input.user_id,
        )
        .map_err(|e| format!("Gagal mencatat audit log mutasi stok: {}", e))?;

        // 6. Catat audit log untuk mst_barang (update stok)
        AuditLogger::record(
            tx,
            "mst_barang",
            &input.barang_id.to_string(),
            AuditAction::Update,
            &json!({
                "barang_id": input.barang_id,
                "stok_sebelum": stok_sebelum,
                "stok_sesudah": stok_sesudah,
                "aksi_pemicu": "MUTASI_STOK",
                "mutasi_id": mutasi_id
            }),
            input.user_id,
        )
        .map_err(|e| format!("Gagal mencatat audit log mst_barang: {}", e))?;

        Ok(mutasi_id)
    }

    /// Proses Stock Opname untuk penyesuaian stok fisik dengan data sistem.
    pub fn process_stock_opname(
        db: &DbConnection,
        input: &StockOpnameInput,
    ) -> std::result::Result<i64, String> {
        if input.stok_fisik < 0 {
            return Err("Stok fisik tidak boleh bernilai negatif!".to_string());
        }

        let mut conn = db.lock().unwrap();
        let tx = conn
            .transaction()
            .map_err(|e| format!("Gagal memulai transaksi: {}", e))?;

        let stok_sebelum: i64 = tx
            .query_row(
                "SELECT stok_saat_ini FROM mst_barang WHERE barang_id = ?1 AND active_st = 1 AND deleted_st = 0;",
                [input.barang_id],
                |row| row.get(0),
            )
            .map_err(|e| format!("Barang id {} tidak ditemukan: {}", input.barang_id, e))?;

        let selisih = input.stok_fisik - stok_sebelum;
        let (stok_masuk, stok_keluar) = if selisih > 0 {
            (selisih, 0)
        } else if selisih < 0 {
            (0, selisih.abs())
        } else {
            (0, 0)
        };

        let keterangan_opname = format!(
            "Stock Opname: fisik={}, sistem={}, selisih={}. {}",
            input.stok_fisik,
            stok_sebelum,
            selisih,
            input.keterangan.clone().unwrap_or_default()
        );

        // Jenis mutasi 4: PENYESUAIAN
        let mutation_input = RecordMutationInput {
            barang_id: input.barang_id,
            jenis_mutasi_id: 4,
            stok_masuk,
            stok_keluar,
            referensi_tabel: Some("STOCK_OPNAME".to_string()),
            referensi_id: None,
            keterangan: Some(keterangan_opname),
            user_id: input.user_id,
        };

        let mutasi_id = Self::record_mutation(&tx, &mutation_input)?;

        tx.commit()
            .map_err(|e| format!("Gagal commit transaksi opname: {}", e))?;

        Ok(mutasi_id)
    }

    /// Ambil ikhtisar semua inventori barang beserta status stoknya.
    pub fn get_inventory_overview(db: &DbConnection) -> Result<Vec<InventoryItem>> {
        let conn = db.lock().unwrap();
        let sql = "
            SELECT 
                b.barang_id, b.kode_barang, b.barcode, b.nama_barang,
                b.kategori_id, k.nama_kategori,
                b.satuan_id, s.nama_satuan,
                b.harga_beli, b.harga_jual,
                b.stok_saat_ini, b.stok_minimum
            FROM mst_barang b
            JOIN mst_kategori_barang k ON b.kategori_id = k.kategori_id
            JOIN mst_satuan_barang s ON b.satuan_id = s.satuan_id
            WHERE b.active_st = 1 AND b.deleted_st = 0
            ORDER BY 
                CASE 
                    WHEN b.stok_saat_ini <= 0 THEN 1 
                    WHEN b.stok_saat_ini <= b.stok_minimum THEN 2 
                    ELSE 3 
                END,
                b.nama_barang ASC;
        ";

        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map([], |row| {
            let stok_saat_ini: i64 = row.get(10)?;
            let stok_minimum: i64 = row.get(11)?;

            let status_stok = if stok_saat_ini <= 0 {
                "HABIS".to_string()
            } else if stok_saat_ini <= stok_minimum {
                "MENIPIS".to_string()
            } else {
                "AMAN".to_string()
            };

            Ok(InventoryItem {
                barang_id: row.get(0)?,
                kode_barang: row.get(1)?,
                barcode: row.get(2)?,
                nama_barang: row.get(3)?,
                kategori_id: row.get(4)?,
                nama_kategori: row.get(5)?,
                satuan_id: row.get(6)?,
                nama_satuan: row.get(7)?,
                harga_beli: row.get(8)?,
                harga_jual: row.get(9)?,
                stok_saat_ini,
                stok_minimum,
                status_stok,
            })
        })?;

        let mut items = Vec::new();
        for r in rows {
            items.push(r?);
        }
        Ok(items)
    }

    /// Dapatkan statistik inventori untuk KPI card
    pub fn get_inventory_stats(db: &DbConnection) -> Result<InventoryStats> {
        let conn = db.lock().unwrap();
        let sql = "
            SELECT 
                COUNT(*) as total_item,
                COALESCE(SUM(stok_saat_ini), 0) as total_stok,
                COALESCE(SUM(CASE WHEN stok_saat_ini <= stok_minimum AND stok_saat_ini > 0 THEN 1 ELSE 0 END), 0) as menipis,
                COALESCE(SUM(CASE WHEN stok_saat_ini <= 0 THEN 1 ELSE 0 END), 0) as habis,
                COALESCE(SUM(stok_saat_ini * harga_beli), 0) as nilai_aset
            FROM mst_barang
            WHERE active_st = 1 AND deleted_st = 0;
        ";

        conn.query_row(sql, [], |row| {
            Ok(InventoryStats {
                total_item: row.get(0)?,
                total_stok_fisik: row.get(1)?,
                stok_menipis_count: row.get(2)?,
                stok_habis_count: row.get(3)?,
                estimasi_nilai_aset: row.get(4)?,
            })
        })
    }

    /// Ambil riwayat pergerakan / mutasi stok.
    pub fn get_stock_mutations(
        db: &DbConnection,
        barang_id: Option<i64>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<StockMutationItem>> {
        let conn = db.lock().unwrap();
        let base_sql = "
            SELECT 
                m.mutasi_id, m.barang_id, b.kode_barang, b.nama_barang,
                m.jenis_mutasi_id, j.kode_mutasi, j.nama_mutasi,
                m.tanggal_waktu, m.stok_masuk, m.stok_keluar,
                m.stok_sebelum, m.stok_sesudah,
                m.referensi_tabel, m.referensi_id, m.keterangan,
                m.created_at, m.created_by
            FROM dat_mutasi_stok m
            JOIN mst_barang b ON m.barang_id = b.barang_id
            JOIN mst_jenis_mutasi j ON m.jenis_mutasi_id = j.jenis_mutasi_id
        ";

        let (sql, params_vec): (String, Vec<rusqlite::types::Value>) = match barang_id {
            Some(bid) => (
                format!("{} WHERE m.barang_id = ?1 ORDER BY m.mutasi_id DESC LIMIT ?2 OFFSET ?3;", base_sql),
                vec![
                    rusqlite::types::Value::Integer(bid),
                    rusqlite::types::Value::Integer(limit as i64),
                    rusqlite::types::Value::Integer(offset as i64),
                ],
            ),
            None => (
                format!("{} ORDER BY m.mutasi_id DESC LIMIT ?1 OFFSET ?2;", base_sql),
                vec![
                    rusqlite::types::Value::Integer(limit as i64),
                    rusqlite::types::Value::Integer(offset as i64),
                ],
            ),
        };

        let mut stmt = conn.prepare(&sql)?;
        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|v| v as &dyn rusqlite::ToSql).collect();

        let rows = stmt.query_map(&params_refs[..], |row| {
            Ok(StockMutationItem {
                mutasi_id: row.get(0)?,
                barang_id: row.get(1)?,
                kode_barang: row.get(2)?,
                nama_barang: row.get(3)?,
                jenis_mutasi_id: row.get(4)?,
                kode_mutasi: row.get(5)?,
                nama_mutasi: row.get(6)?,
                tanggal_waktu: row.get(7)?,
                stok_masuk: row.get(8)?,
                stok_keluar: row.get(9)?,
                stok_sebelum: row.get(10)?,
                stok_sesudah: row.get(11)?,
                referensi_tabel: row.get(12)?,
                referensi_id: row.get(13)?,
                keterangan: row.get(14)?,
                created_at: row.get(15)?,
                created_by: row.get(16)?,
            })
        })?;

        let mut mutations = Vec::new();
        for r in rows {
            mutations.push(r?);
        }
        Ok(mutations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::connection::DbManager;
    use crate::database::migrator::Migrator;

    fn setup_test_db() -> (DbManager, DbConnection) {
        let db_manager = DbManager::init_in_memory().expect("Failed in-memory DB");
        let conn_arc = db_manager.get_connection();
        Migrator::run(&mut conn_arc.lock().unwrap()).expect("Failed running migrations");
        (db_manager, conn_arc)
    }

    #[test]
    fn test_stock_mutation_addition_and_reduction() {
        let (_manager, conn_arc) = setup_test_db();

        // 1. Tambah master barang
        {
            let conn = conn_arc.lock().unwrap();
            conn.execute(
                "INSERT INTO mst_barang (
                    created_at, created_by, kode_barang, barcode, nama_barang, 
                    kategori_id, satuan_id, harga_beli, harga_jual, stok_saat_ini, stok_minimum
                ) VALUES (
                    '2026-01-01T00:00:00Z', 1, 'TEST-BRG-1', '123456', 'Barang Uji 1',
                    1, 1, 10000, 15000, 10, 5
                );",
                [],
            )
            .unwrap();
        }

        let barang_id = 1;

        // 2. Tambah stok via mutasi
        {
            let mut conn = conn_arc.lock().unwrap();
            let tx = conn.transaction().unwrap();
            let input = RecordMutationInput {
                barang_id,
                jenis_mutasi_id: 1, // STOK_AWAL
                stok_masuk: 20,
                stok_keluar: 0,
                referensi_tabel: Some("TEST".to_string()),
                referensi_id: Some(101),
                keterangan: Some("Tambah stok 20".to_string()),
                user_id: Some(1),
            };
            let mutasi_id = InventoryService::record_mutation(&tx, &input).unwrap();
            assert!(mutasi_id > 0);
            tx.commit().unwrap();
        }

        // Cek stok setelah mutasi masuk: 10 + 20 = 30
        {
            let conn = conn_arc.lock().unwrap();
            let stok: i64 = conn
                .query_row(
                    "SELECT stok_saat_ini FROM mst_barang WHERE barang_id = ?1;",
                    [barang_id],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(stok, 30);
        }

        // 3. Kurangi stok via mutasi keluar
        {
            let mut conn = conn_arc.lock().unwrap();
            let tx = conn.transaction().unwrap();
            let input = RecordMutationInput {
                barang_id,
                jenis_mutasi_id: 3, // PENJUALAN
                stok_masuk: 0,
                stok_keluar: 5,
                referensi_tabel: Some("PENJUALAN".to_string()),
                referensi_id: Some(202),
                keterangan: Some("Penjualan 5 pcs".to_string()),
                user_id: Some(1),
            };
            InventoryService::record_mutation(&tx, &input).unwrap();
            tx.commit().unwrap();
        }

        let conn = conn_arc.lock().unwrap();
        let stok_akhir: i64 = conn
            .query_row(
                "SELECT stok_saat_ini FROM mst_barang WHERE barang_id = ?1;",
                [barang_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(stok_akhir, 25);
    }

    #[test]
    fn test_prevent_negative_stock() {
        let (_manager, conn_arc) = setup_test_db();

        {
            let conn = conn_arc.lock().unwrap();
            conn.execute(
                "INSERT INTO mst_barang (
                    created_at, created_by, kode_barang, nama_barang, 
                    kategori_id, satuan_id, harga_beli, harga_jual, stok_saat_ini, stok_minimum
                ) VALUES (
                    '2026-01-01T00:00:00Z', 1, 'TEST-NEG-1', 'Barang Stok 5',
                    1, 1, 10000, 15000, 5, 2
                );",
                [],
            )
            .unwrap();
        }
        let barang_id = 1;

        // Coba kurangi 10 pcs padahal stok cuma 5
        let mut conn = conn_arc.lock().unwrap();
        let tx = conn.transaction().unwrap();
        let input = RecordMutationInput {
            barang_id,
            jenis_mutasi_id: 3,
            stok_masuk: 0,
            stok_keluar: 10,
            referensi_tabel: None,
            referensi_id: None,
            keterangan: None,
            user_id: Some(1),
        };

        let result = InventoryService::record_mutation(&tx, &input);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Stok barang tidak mencukupi"));
    }

    #[test]
    fn test_stock_opname_adjustment() {
        let (_manager, conn_arc) = setup_test_db();

        {
            let conn = conn_arc.lock().unwrap();
            conn.execute(
                "INSERT INTO mst_barang (
                    created_at, created_by, kode_barang, nama_barang, 
                    kategori_id, satuan_id, harga_beli, harga_jual, stok_saat_ini, stok_minimum
                ) VALUES (
                    '2026-01-01T00:00:00Z', 1, 'TEST-OPN-1', 'Barang Opname',
                    1, 1, 5000, 7000, 50, 10
                );",
                [],
            )
            .unwrap();
        }
        let barang_id = 1;

        // 1. Stock Opname: fisik ternyata 45 (selisih -5)
        let opname_input = StockOpnameInput {
            barang_id,
            stok_fisik: 45,
            keterangan: Some("Ditemukan 5 pcs rusak saat hitung fisik".to_string()),
            user_id: Some(1),
        };
        let mutasi_id = InventoryService::process_stock_opname(&conn_arc, &opname_input).unwrap();
        assert!(mutasi_id > 0);

        let conn = conn_arc.lock().unwrap();
        let stok_baru: i64 = conn
            .query_row(
                "SELECT stok_saat_ini FROM mst_barang WHERE barang_id = ?1;",
                [barang_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(stok_baru, 45);

        // Periksa mutasi tercatat dengan benar
        let (masuk, keluar, sesudah): (i64, i64, i64) = conn
            .query_row(
                "SELECT stok_masuk, stok_keluar, stok_sesudah FROM dat_mutasi_stok WHERE mutasi_id = ?1;",
                [mutasi_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(masuk, 0);
        assert_eq!(keluar, 5);
        assert_eq!(sesudah, 45);

        // 2. Periksa audit log di log_db
        let count_log: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM log_db.dat_mutasi_stok WHERE primary_key_id = ?1;",
                [mutasi_id.to_string()],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count_log, 1);
    }
}
