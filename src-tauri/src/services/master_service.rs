use crate::database::audit::{AuditAction, AuditLogger};
use crate::database::connection::DbConnection;
use rusqlite::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarangItem {
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
    pub catatan: Option<String>,
    pub active_st: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveBarangInput {
    pub barang_id: Option<i64>,
    pub kode_barang: String,
    pub barcode: Option<String>,
    pub nama_barang: String,
    pub kategori_id: i64,
    pub satuan_id: i64,
    pub harga_beli: i64,
    pub harga_jual: i64,
    pub stok_minimum: i64,
    pub catatan: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KategoriItem {
    pub kategori_id: i64,
    pub kode_kategori: String,
    pub nama_kategori: String,
    pub keterangan: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SatuanItem {
    pub satuan_id: i64,
    pub kode_satuan: String,
    pub nama_satuan: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupplierItem {
    pub supplier_id: i64,
    pub kode_pihak: String,
    pub nama_pihak: String,
    pub jenis_pihak: String,
    pub nama_kontak: Option<String>,
    pub nomor_telepon: Option<String>,
    pub alamat: Option<String>,
    pub catatan: Option<String>,
    pub active_st: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JenisMutasiItem {
    pub jenis_mutasi_id: i64,
    pub kode_mutasi: String,
    pub nama_mutasi: String,
    pub dampak_stok: String,
    pub keterangan: Option<String>,
}

pub struct MasterService;

impl MasterService {
    // ==========================================
    // BARANG
    // ==========================================

    pub fn get_all_barang(db: &DbConnection) -> Result<Vec<BarangItem>> {
        let conn = db.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT 
                b.barang_id, b.kode_barang, b.barcode, b.nama_barang, 
                b.kategori_id, k.nama_kategori, 
                b.satuan_id, s.nama_satuan, 
                b.harga_beli, b.harga_jual, b.stok_saat_ini, b.stok_minimum, 
                b.catatan, b.active_st
            FROM mst_barang b
            JOIN mst_kategori_barang k ON b.kategori_id = k.kategori_id
            JOIN mst_satuan_barang s ON b.satuan_id = s.satuan_id
            WHERE b.deleted_st = 0
            ORDER BY b.nama_barang ASC;",
        )?;

        let rows = stmt.query_map([], |r| {
            Ok(BarangItem {
                barang_id: r.get(0)?,
                kode_barang: r.get(1)?,
                barcode: r.get(2)?,
                nama_barang: r.get(3)?,
                kategori_id: r.get(4)?,
                nama_kategori: r.get(5)?,
                satuan_id: r.get(6)?,
                nama_satuan: r.get(7)?,
                harga_beli: r.get(8)?,
                harga_jual: r.get(9)?,
                stok_saat_ini: r.get(10)?,
                stok_minimum: r.get(11)?,
                catatan: r.get(12)?,
                active_st: r.get(13)?,
            })
        })?;

        let mut list = Vec::new();
        for item in rows {
            list.push(item?);
        }
        Ok(list)
    }

    pub fn save_barang(
        db: &DbConnection,
        input: SaveBarangInput,
        user_id: Option<i64>,
    ) -> Result<i64> {
        let mut conn = db.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        let tx = conn.transaction()?;

        let barang_id = match input.barang_id {
            Some(id) => {
                // UPDATE existing barang
                let before_snapshot: Option<serde_json::Value> = tx
                    .query_row(
                        "SELECT json_object('kode_barang', kode_barang, 'nama_barang', nama_barang, 'harga_beli', harga_beli, 'harga_jual', harga_jual)
                         FROM mst_barang WHERE barang_id = ?1;",
                        [id],
                        |r| {
                            let text: String = r.get(0)?;
                            Ok(serde_json::from_str(&text).unwrap_or(json!({})))
                        },
                    )
                    .ok();

                tx.execute(
                    "UPDATE mst_barang SET 
                        kode_barang = ?1, barcode = ?2, nama_barang = ?3, 
                        kategori_id = ?4, satuan_id = ?5, harga_beli = ?6, 
                        harga_jual = ?7, stok_minimum = ?8, catatan = ?9, 
                        updated_at = ?10, updated_by = ?11
                     WHERE barang_id = ?12;",
                    rusqlite::params![
                        input.kode_barang,
                        input.barcode,
                        input.nama_barang,
                        input.kategori_id,
                        input.satuan_id,
                        input.harga_beli,
                        input.harga_jual,
                        input.stok_minimum,
                        input.catatan,
                        now,
                        user_id,
                        id
                    ],
                )?;

                AuditLogger::record(
                    &tx,
                    "mst_barang",
                    &id.to_string(),
                    AuditAction::Update,
                    &json!({
                        "before": before_snapshot,
                        "after": {
                            "kode_barang": input.kode_barang,
                            "nama_barang": input.nama_barang,
                            "harga_beli": input.harga_beli,
                            "harga_jual": input.harga_jual
                        }
                    }),
                    user_id,
                )?;

                id
            }
            None => {
                // INSERT new barang
                tx.execute(
                    "INSERT INTO mst_barang (
                        created_at, created_by, kode_barang, barcode, 
                        nama_barang, kategori_id, satuan_id, harga_beli, 
                        harga_jual, stok_saat_ini, stok_minimum, catatan, active_st
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 0, ?10, ?11, 1);",
                    rusqlite::params![
                        now,
                        user_id,
                        input.kode_barang,
                        input.barcode,
                        input.nama_barang,
                        input.kategori_id,
                        input.satuan_id,
                        input.harga_beli,
                        input.harga_jual,
                        input.stok_minimum,
                        input.catatan
                    ],
                )?;

                let new_id = tx.last_insert_rowid();

                AuditLogger::record(
                    &tx,
                    "mst_barang",
                    &new_id.to_string(),
                    AuditAction::Insert,
                    &json!({
                        "kode_barang": input.kode_barang,
                        "nama_barang": input.nama_barang,
                        "harga_beli": input.harga_beli,
                        "harga_jual": input.harga_jual,
                        "kategori_id": input.kategori_id,
                        "satuan_id": input.satuan_id
                    }),
                    user_id,
                )?;

                new_id
            }
        };

        tx.commit()?;
        Ok(barang_id)
    }

    pub fn delete_barang(db: &DbConnection, id: i64, user_id: Option<i64>) -> Result<()> {
        let mut conn = db.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        let tx = conn.transaction()?;

        tx.execute(
            "UPDATE mst_barang SET deleted_st = 1, deleted_at = ?1, deleted_by = ?2 WHERE barang_id = ?3;",
            rusqlite::params![now, user_id, id],
        )?;

        AuditLogger::record(
            &tx,
            "mst_barang",
            &id.to_string(),
            AuditAction::Delete,
            &json!({"action": "SOFT_DELETE", "barang_id": id}),
            user_id,
        )?;

        tx.commit()?;
        Ok(())
    }

    // ==========================================
    // KATEGORI & SATUAN
    // ==========================================

    pub fn get_all_kategori(db: &DbConnection) -> Result<Vec<KategoriItem>> {
        let conn = db.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT kategori_id, kode_kategori, nama_kategori, keterangan 
             FROM mst_kategori_barang WHERE deleted_st = 0 ORDER BY nama_kategori ASC;",
        )?;

        let rows = stmt.query_map([], |r| {
            Ok(KategoriItem {
                kategori_id: r.get(0)?,
                kode_kategori: r.get(1)?,
                nama_kategori: r.get(2)?,
                keterangan: r.get(3)?,
            })
        })?;

        let mut list = Vec::new();
        for item in rows {
            list.push(item?);
        }
        Ok(list)
    }

    pub fn get_all_satuan(db: &DbConnection) -> Result<Vec<SatuanItem>> {
        let conn = db.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT satuan_id, kode_satuan, nama_satuan 
             FROM mst_satuan_barang WHERE deleted_st = 0 ORDER BY kode_satuan ASC;",
        )?;

        let rows = stmt.query_map([], |r| {
            Ok(SatuanItem {
                satuan_id: r.get(0)?,
                kode_satuan: r.get(1)?,
                nama_satuan: r.get(2)?,
            })
        })?;

        let mut list = Vec::new();
        for item in rows {
            list.push(item?);
        }
        Ok(list)
    }

    // ==========================================
    // SUPPLIER & JENIS MUTASI
    // ==========================================

    pub fn get_all_supplier(db: &DbConnection) -> Result<Vec<SupplierItem>> {
        let conn = db.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT supplier_id, kode_pihak, nama_pihak, jenis_pihak, nama_kontak, nomor_telepon, alamat, catatan, active_st 
             FROM mst_supplier_pihak WHERE deleted_st = 0 ORDER BY nama_pihak ASC;",
        )?;

        let rows = stmt.query_map([], |r| {
            Ok(SupplierItem {
                supplier_id: r.get(0)?,
                kode_pihak: r.get(1)?,
                nama_pihak: r.get(2)?,
                jenis_pihak: r.get(3)?,
                nama_kontak: r.get(4)?,
                nomor_telepon: r.get(5)?,
                alamat: r.get(6)?,
                catatan: r.get(7)?,
                active_st: r.get(8)?,
            })
        })?;

        let mut list = Vec::new();
        for item in rows {
            list.push(item?);
        }
        Ok(list)
    }

    pub fn get_all_jenis_mutasi(db: &DbConnection) -> Result<Vec<JenisMutasiItem>> {
        let conn = db.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT jenis_mutasi_id, kode_mutasi, nama_mutasi, dampak_stok, keterangan 
             FROM mst_jenis_mutasi WHERE deleted_st = 0 ORDER BY jenis_mutasi_id ASC;",
        )?;

        let rows = stmt.query_map([], |r| {
            Ok(JenisMutasiItem {
                jenis_mutasi_id: r.get(0)?,
                kode_mutasi: r.get(1)?,
                nama_mutasi: r.get(2)?,
                dampak_stok: r.get(3)?,
                keterangan: r.get(4)?,
            })
        })?;

        let mut list = Vec::new();
        for item in rows {
            list.push(item?);
        }
        Ok(list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::connection::DbManager;
    use crate::database::migrator::Migrator;

    #[test]
    fn test_crud_barang_and_audit_log() {
        let db_manager = DbManager::init_in_memory().unwrap();
        let conn_arc = db_manager.get_connection();

        {
            let mut conn = conn_arc.lock().unwrap();
            Migrator::run(&mut conn).unwrap();
        }

        // 1. Insert Barang
        let input = SaveBarangInput {
            barang_id: None,
            kode_barang: "BRG-001".to_string(),
            barcode: Some("899123456789".to_string()),
            nama_barang: "Kopi Gayo 250g".to_string(),
            kategori_id: 3, // Minuman
            satuan_id: 1,   // PCS
            harga_beli: 30000,
            harga_jual: 45000,
            stok_minimum: 5,
            catatan: Some("Kopi kemasan pouch".to_string()),
        };

        let barang_id = MasterService::save_barang(&conn_arc, input, Some(1)).unwrap();
        assert_eq!(barang_id, 1);

        // 2. Read Barang List
        let list = MasterService::get_all_barang(&conn_arc).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].nama_barang, "Kopi Gayo 250g");
        assert_eq!(list[0].harga_jual, 45000);

        // 3. Update Barang
        let update_input = SaveBarangInput {
            barang_id: Some(barang_id),
            kode_barang: "BRG-001".to_string(),
            barcode: Some("899123456789".to_string()),
            nama_barang: "Kopi Gayo Premium 250g".to_string(),
            kategori_id: 3,
            satuan_id: 1,
            harga_beli: 32000,
            harga_jual: 48000,
            stok_minimum: 10,
            catatan: None,
        };

        MasterService::save_barang(&conn_arc, update_input, Some(1)).unwrap();

        let list_updated = MasterService::get_all_barang(&conn_arc).unwrap();
        assert_eq!(list_updated[0].nama_barang, "Kopi Gayo Premium 250g");
        assert_eq!(list_updated[0].harga_jual, 48000);

        // 4. Verify Audit Logs
        let conn = conn_arc.lock().unwrap();
        let log_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM log_db.mst_barang WHERE primary_key_id = '1';",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(log_count, 2); // 1 INSERT + 1 UPDATE
    }
}
