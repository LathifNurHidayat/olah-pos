use crate::database::audit::{AuditAction, AuditLogger};
use crate::database::connection::DbConnection;
use crate::services::inventory_service::{InventoryService, RecordMutationInput};
use rusqlite::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePembelianItem {
    pub barang_id: i64,
    pub kuantitas: i64,
    pub harga_beli_satuan: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePembelianInput {
    pub nomor_faktur: String,
    pub tanggal_pembelian: String,
    pub supplier_id: i64,
    pub jenis_pembayaran: String, // "TUNAI" | "KREDIT"
    pub items: Vec<CreatePembelianItem>,
    pub nominal_dibayar: i64,
    pub jatuh_tempo: Option<String>,
    pub catatan: Option<String>,
    pub user_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BayarUtangInput {
    pub utang_id: i64,
    pub tanggal_bayar: String,
    pub nominal_bayar: i64,
    pub metode_bayar: String, // "TUNAI" | "TRANSFER"
    pub keterangan: Option<String>,
    pub user_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PembelianListItem {
    pub pembelian_id: i64,
    pub nomor_faktur: String,
    pub tanggal_pembelian: String,
    pub supplier_id: i64,
    pub nama_supplier: String,
    pub jenis_pembayaran: String,
    pub total_tagihan: i64,
    pub nominal_dibayar: i64,
    pub sisa_utang: i64,
    pub jatuh_tempo: Option<String>,
    pub catatan: Option<String>,
    pub total_item: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailPembelianItem {
    pub detail_pembelian_id: i64,
    pub pembelian_id: i64,
    pub barang_id: i64,
    pub kode_barang: String,
    pub nama_barang: String,
    pub nama_satuan: String,
    pub kuantitas: i64,
    pub harga_beli_satuan: i64,
    pub subtotal: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtangItem {
    pub utang_id: i64,
    pub nomor_referensi: String,
    pub pembelian_id: i64,
    pub nomor_faktur: String,
    pub supplier_id: i64,
    pub nama_supplier: String,
    pub total_utang_awal: i64,
    pub total_dibayar: i64,
    pub sisa_utang: i64,
    pub status_utang: String, // "BELUM_DIBAYAR", "DIBAYAR_SEBAGIAN", "LUNAS"
    pub tanggal_jatuh_tempo: Option<String>,
    pub tanggal_pembelian: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PembayaranUtangItem {
    pub pembayaran_utang_id: i64,
    pub nomor_bukti_bayar: String,
    pub utang_id: i64,
    pub tanggal_bayar: String,
    pub nominal_bayar: i64,
    pub metode_bayar: String,
    pub keterangan: Option<String>,
    pub created_at: String,
}

pub struct PurchaseService;

impl PurchaseService {
    /// Membuat transaksi pembelian baru (Tunai / Kredit) secara atomik.
    /// Stok barang otomatis bertambah saat pembelian dibuat.
    pub fn create_pembelian(
        db: &DbConnection,
        input: CreatePembelianInput,
    ) -> std::result::Result<i64, String> {
        if input.items.is_empty() {
            return Err("Daftar barang pembelian tidak boleh kosong!".to_string());
        }

        let now = chrono::Utc::now().to_rfc3339();

        // Hitung total tagihan dari rincian item
        let mut total_tagihan: i64 = 0;
        for it in &input.items {
            if it.kuantitas <= 0 {
                return Err("Kuantitas barang harus lebih besar dari 0!".to_string());
            }
            if it.harga_beli_satuan < 0 {
                return Err("Harga beli satuan tidak boleh bernilai negatif!".to_string());
            }
            total_tagihan += it.kuantitas * it.harga_beli_satuan;
        }

        // Tentukan nominal dibayar dan sisa utang
        let (nominal_dibayar, sisa_utang) = match input.jenis_pembayaran.as_str() {
            "TUNAI" => (total_tagihan, 0),
            "KREDIT" => {
                let dp = input.nominal_dibayar;
                if dp > total_tagihan {
                    return Err(format!(
                        "Nominal dibayar ({}) melebihi total tagihan ({})!",
                        dp, total_tagihan
                    ));
                }
                (dp, total_tagihan - dp)
            }
            other => return Err(format!("Jenis pembayaran '{}' tidak valid!", other)),
        };

        let mut conn = db.lock().unwrap();
        let tx = conn
            .transaction()
            .map_err(|e| format!("Gagal memulai transaksi pembelian: {}", e))?;

        // 1. Insert ke dat_pembelian
        tx.execute(
            "INSERT INTO dat_pembelian (
                created_at, created_by, nomor_faktur, tanggal_pembelian, 
                supplier_id, jenis_pembayaran, total_tagihan, 
                nominal_dibayar, sisa_utang, jatuh_tempo, catatan
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11);",
            rusqlite::params![
                now,
                input.user_id,
                input.nomor_faktur,
                input.tanggal_pembelian,
                input.supplier_id,
                input.jenis_pembayaran,
                total_tagihan,
                nominal_dibayar,
                sisa_utang,
                input.jatuh_tempo,
                input.catatan,
            ],
        )
        .map_err(|e| format!("Gagal mencatat dat_pembelian: {}", e))?;

        let pembelian_id = tx.last_insert_rowid();

        // 2. Insert ke dat_detail_pembelian & Catat Mutasi Stok Masuk
        for it in &input.items {
            let subtotal = it.kuantitas * it.harga_beli_satuan;
            tx.execute(
                "INSERT INTO dat_detail_pembelian (
                    created_at, created_by, pembelian_id, barang_id, 
                    kuantitas, harga_beli_satuan, subtotal
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7);",
                rusqlite::params![
                    now,
                    input.user_id,
                    pembelian_id,
                    it.barang_id,
                    it.kuantitas,
                    it.harga_beli_satuan,
                    subtotal,
                ],
            )
            .map_err(|e| format!("Gagal mencatat detail pembelian barang {}: {}", it.barang_id, e))?;

            // Mutasi Stok: jenis_mutasi_id 2 = PEMBELIAN
            let mutasi_input = RecordMutationInput {
                barang_id: it.barang_id,
                jenis_mutasi_id: 2,
                stok_masuk: it.kuantitas,
                stok_keluar: 0,
                referensi_tabel: Some("dat_pembelian".to_string()),
                referensi_id: Some(pembelian_id),
                keterangan: Some(format!("Pembelian faktur {}", input.nomor_faktur)),
                user_id: input.user_id,
            };
            InventoryService::record_mutation(&tx, &mutasi_input)?;

            // Perbarui master harga beli jika ada harga baru
            if it.harga_beli_satuan > 0 {
                tx.execute(
                    "UPDATE mst_barang SET harga_beli = ?1, updated_at = ?2, updated_by = ?3 WHERE barang_id = ?4;",
                    rusqlite::params![it.harga_beli_satuan, now, input.user_id, it.barang_id],
                ).map_err(|e| format!("Gagal update harga beli master barang {}: {}", it.barang_id, e))?;
            }
        }

        // 3. Jika KREDIT dan ada sisa utang, buat Kartu Utang di dat_utang
        if input.jenis_pembayaran == "KREDIT" && sisa_utang > 0 {
            let nomor_ref_utang = format!("UTG-{}", pembelian_id);
            let status_utang = if nominal_dibayar > 0 {
                "DIBAYAR_SEBAGIAN"
            } else {
                "BELUM_DIBAYAR"
            };

            tx.execute(
                "INSERT INTO dat_utang (
                    created_at, created_by, nomor_referensi, pembelian_id, 
                    supplier_id, total_utang_awal, total_dibayar, 
                    sisa_utang, status_utang, tanggal_jatuh_tempo
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10);",
                rusqlite::params![
                    now,
                    input.user_id,
                    nomor_ref_utang,
                    pembelian_id,
                    input.supplier_id,
                    total_tagihan,
                    nominal_dibayar,
                    sisa_utang,
                    status_utang,
                    input.jatuh_tempo,
                ],
            )
            .map_err(|e| format!("Gagal mencatat kartu utang: {}", e))?;
        }

        // 4. Catat arus kas keluar untuk uang yang dibayarkan saat pembelian
        if nominal_dibayar > 0 {
            let kategori = if input.jenis_pembayaran == "TUNAI" {
                "PEMBELIAN_TUNAI"
            } else {
                "PEMBELIAN_DP"
            };

            tx.execute(
                "INSERT INTO dat_pergerakan_uang (
                    created_at, created_by, tanggal_waktu, jenis_arus, 
                    kategori_transaksi, nominal, metode_pembayaran, 
                    referensi_tabel, referensi_id, keterangan
                ) VALUES (?1, ?2, ?3, 'KELUAR', ?4, ?5, 'TUNAI', 'dat_pembelian', ?6, ?7);",
                rusqlite::params![
                    now,
                    input.user_id,
                    input.tanggal_pembelian,
                    kategori,
                    nominal_dibayar,
                    pembelian_id,
                    format!("Pembayaran pembelian faktur {}", input.nomor_faktur),
                ],
            )
            .map_err(|e| format!("Gagal mencatat arus kas pembelian: {}", e))?;
        }

        // 5. Catat audit log dat_pembelian
        AuditLogger::record(
            &tx,
            "dat_pembelian",
            &pembelian_id.to_string(),
            AuditAction::Insert,
            &json!({
                "pembelian_id": pembelian_id,
                "nomor_faktur": input.nomor_faktur,
                "supplier_id": input.supplier_id,
                "jenis_pembayaran": input.jenis_pembayaran,
                "total_tagihan": total_tagihan,
                "nominal_dibayar": nominal_dibayar,
                "sisa_utang": sisa_utang
            }),
            input.user_id,
        )
        .map_err(|e| format!("Gagal mencatat audit log pembelian: {}", e))?;

        tx.commit()
            .map_err(|e| format!("Gagal commit transaksi pembelian: {}", e))?;

        Ok(pembelian_id)
    }

    /// Bayar / Cicil Utang ke Supplier.
    /// Memotong sisa utang dan mencatat arus kas KELUAR, namun TIDAK mempengaruhi stok barang.
    pub fn bayar_utang(
        db: &DbConnection,
        input: BayarUtangInput,
    ) -> std::result::Result<i64, String> {
        if input.nominal_bayar <= 0 {
            return Err("Nominal pembayaran utang harus lebih besar dari 0!".to_string());
        }

        let now = chrono::Utc::now().to_rfc3339();

        let mut conn = db.lock().unwrap();
        let tx = conn
            .transaction()
            .map_err(|e| format!("Gagal memulai transaksi bayar utang: {}", e))?;

        // 1. Ambil data utang saat ini
        let (pembelian_id, _total_utang_awal, total_dibayar, sisa_utang, nomor_referensi): (
            i64,
            i64,
            i64,
            i64,
            String,
        ) = tx
            .query_row(
                "SELECT pembelian_id, total_utang_awal, total_dibayar, sisa_utang, nomor_referensi 
                 FROM dat_utang WHERE utang_id = ?1 AND active_st = 1 AND deleted_st = 0;",
                [input.utang_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .map_err(|e| format!("Data utang id {} tidak ditemukan: {}", input.utang_id, e))?;

        if input.nominal_bayar > sisa_utang {
            return Err(format!(
                "Nominal pembayaran ({}) melebihi sisa utang ({})!",
                input.nominal_bayar, sisa_utang
            ));
        }

        let total_dibayar_baru = total_dibayar + input.nominal_bayar;
        let sisa_utang_baru = sisa_utang - input.nominal_bayar;
        let status_utang = if sisa_utang_baru == 0 {
            "LUNAS"
        } else {
            "DIBAYAR_SEBAGIAN"
        };

        let nomor_bukti = format!("BYR-{}", chrono::Utc::now().timestamp_millis());

        // 2. Insert ke dat_pembayaran_utang
        tx.execute(
            "INSERT INTO dat_pembayaran_utang (
                created_at, created_by, nomor_bukti_bayar, utang_id, 
                tanggal_bayar, nominal_bayar, metode_bayar, keterangan
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8);",
            rusqlite::params![
                now,
                input.user_id,
                nomor_bukti,
                input.utang_id,
                input.tanggal_bayar,
                input.nominal_bayar,
                input.metode_bayar,
                input.keterangan,
            ],
        )
        .map_err(|e| format!("Gagal mencatat pembayaran utang: {}", e))?;

        let pembayaran_id = tx.last_insert_rowid();

        // 3. Update dat_utang
        tx.execute(
            "UPDATE dat_utang 
             SET total_dibayar = ?1, sisa_utang = ?2, status_utang = ?3, 
                 updated_at = ?4, updated_by = ?5 
             WHERE utang_id = ?6;",
            rusqlite::params![
                total_dibayar_baru,
                sisa_utang_baru,
                status_utang,
                now,
                input.user_id,
                input.utang_id,
            ],
        )
        .map_err(|e| format!("Gagal memperbarui kartu utang: {}", e))?;

        // 4. Update dat_pembelian
        tx.execute(
            "UPDATE dat_pembelian 
             SET nominal_dibayar = nominal_dibayar + ?1, sisa_utang = ?2, 
                 updated_at = ?3, updated_by = ?4 
             WHERE pembelian_id = ?5;",
            rusqlite::params![
                input.nominal_bayar,
                sisa_utang_baru,
                now,
                input.user_id,
                pembelian_id,
            ],
        )
        .map_err(|e| format!("Gagal memperbarui sisa utang faktur pembelian: {}", e))?;

        // 5. Catat arus kas keluar di dat_pergerakan_uang
        tx.execute(
            "INSERT INTO dat_pergerakan_uang (
                created_at, created_by, tanggal_waktu, jenis_arus, 
                kategori_transaksi, nominal, metode_pembayaran, 
                referensi_tabel, referensi_id, keterangan
            ) VALUES (?1, ?2, ?3, 'KELUAR', 'BAYAR_UTANG', ?4, ?5, 'dat_pembayaran_utang', ?6, ?7);",
            rusqlite::params![
                now,
                input.user_id,
                input.tanggal_bayar,
                input.nominal_bayar,
                input.metode_bayar,
                pembayaran_id,
                format!("Bayar utang {} - {}", nomor_referensi, input.keterangan.clone().unwrap_or_default()),
            ],
        )
        .map_err(|e| format!("Gagal mencatat arus kas bayar utang: {}", e))?;

        // 6. Catat audit logs
        AuditLogger::record(
            &tx,
            "dat_pembayaran_utang",
            &pembayaran_id.to_string(),
            AuditAction::Insert,
            &json!({
                "pembayaran_utang_id": pembayaran_id,
                "nomor_bukti_bayar": nomor_bukti,
                "utang_id": input.utang_id,
                "nominal_bayar": input.nominal_bayar,
                "metode_bayar": input.metode_bayar
            }),
            input.user_id,
        )
        .map_err(|e| format!("Gagal mencatat audit log pembayaran utang: {}", e))?;

        AuditLogger::record(
            &tx,
            "dat_utang",
            &input.utang_id.to_string(),
            AuditAction::Update,
            &json!({
                "utang_id": input.utang_id,
                "total_dibayar_sebelum": total_dibayar,
                "total_dibayar_sesudah": total_dibayar_baru,
                "sisa_utang_sebelum": sisa_utang,
                "sisa_utang_sesudah": sisa_utang_baru,
                "status_utang": status_utang
            }),
            input.user_id,
        )
        .map_err(|e| format!("Gagal mencatat audit log kartu utang: {}", e))?;

        tx.commit()
            .map_err(|e| format!("Gagal commit transaksi pembayaran utang: {}", e))?;

        Ok(pembayaran_id)
    }

    /// Mengambil daftar faktur pembelian
    pub fn get_pembelian_list(db: &DbConnection) -> Result<Vec<PembelianListItem>> {
        let conn = db.lock().unwrap();
        let sql = "
            SELECT 
                p.pembelian_id, p.nomor_faktur, p.tanggal_pembelian, 
                p.supplier_id, s.nama_pihak as nama_supplier, 
                p.jenis_pembayaran, p.total_tagihan, p.nominal_dibayar, 
                p.sisa_utang, p.jatuh_tempo, p.catatan,
                (SELECT COUNT(*) FROM dat_detail_pembelian dp WHERE dp.pembelian_id = p.pembelian_id) as total_item
            FROM dat_pembelian p
            JOIN mst_supplier_pihak s ON p.supplier_id = s.supplier_id
            WHERE p.active_st = 1 AND p.deleted_st = 0
            ORDER BY p.pembelian_id DESC;
        ";

        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map([], |r| {
            Ok(PembelianListItem {
                pembelian_id: r.get(0)?,
                nomor_faktur: r.get(1)?,
                tanggal_pembelian: r.get(2)?,
                supplier_id: r.get(3)?,
                nama_supplier: r.get(4)?,
                jenis_pembayaran: r.get(5)?,
                total_tagihan: r.get(6)?,
                nominal_dibayar: r.get(7)?,
                sisa_utang: r.get(8)?,
                jatuh_tempo: r.get(9)?,
                catatan: r.get(10)?,
                total_item: r.get(11)?,
            })
        })?;

        let mut list = Vec::new();
        for it in rows {
            list.push(it?);
        }
        Ok(list)
    }

    /// Mengambil rincian barang dari suatu pembelian
    pub fn get_detail_pembelian(
        db: &DbConnection,
        pembelian_id: i64,
    ) -> Result<Vec<DetailPembelianItem>> {
        let conn = db.lock().unwrap();
        let sql = "
            SELECT 
                dp.detail_pembelian_id, dp.pembelian_id, dp.barang_id,
                b.kode_barang, b.nama_barang, s.nama_satuan,
                dp.kuantitas, dp.harga_beli_satuan, dp.subtotal
            FROM dat_detail_pembelian dp
            JOIN mst_barang b ON dp.barang_id = b.barang_id
            JOIN mst_satuan_barang s ON b.satuan_id = s.satuan_id
            WHERE dp.pembelian_id = ?1 AND dp.active_st = 1 AND dp.deleted_st = 0;
        ";

        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map([pembelian_id], |r| {
            Ok(DetailPembelianItem {
                detail_pembelian_id: r.get(0)?,
                pembelian_id: r.get(1)?,
                barang_id: r.get(2)?,
                kode_barang: r.get(3)?,
                nama_barang: r.get(4)?,
                nama_satuan: r.get(5)?,
                kuantitas: r.get(6)?,
                harga_beli_satuan: r.get(7)?,
                subtotal: r.get(8)?,
            })
        })?;

        let mut list = Vec::new();
        for it in rows {
            list.push(it?);
        }
        Ok(list)
    }

    /// Mengambil daftar kartu utang
    pub fn get_utang_list(
        db: &DbConnection,
        status_filter: Option<String>,
    ) -> Result<Vec<UtangItem>> {
        let conn = db.lock().unwrap();
        let base_sql = "
            SELECT 
                u.utang_id, u.nomor_referensi, u.pembelian_id, 
                p.nomor_faktur, u.supplier_id, s.nama_pihak as nama_supplier, 
                u.total_utang_awal, u.total_dibayar, u.sisa_utang, 
                u.status_utang, u.tanggal_jatuh_tempo, p.tanggal_pembelian
            FROM dat_utang u
            JOIN dat_pembelian p ON u.pembelian_id = p.pembelian_id
            JOIN mst_supplier_pihak s ON u.supplier_id = s.supplier_id
            WHERE u.active_st = 1 AND u.deleted_st = 0
        ";

        let (sql, params_vec): (String, Vec<rusqlite::types::Value>) = match status_filter {
            Some(st) if st != "ALL" => (
                format!("{} AND u.status_utang = ?1 ORDER BY u.utang_id DESC;", base_sql),
                vec![rusqlite::types::Value::Text(st)],
            ),
            _ => (format!("{} ORDER BY u.utang_id DESC;", base_sql), vec![]),
        };

        let mut stmt = conn.prepare(&sql)?;
        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|v| v as &dyn rusqlite::ToSql).collect();

        let rows = stmt.query_map(&params_refs[..], |r| {
            Ok(UtangItem {
                utang_id: r.get(0)?,
                nomor_referensi: r.get(1)?,
                pembelian_id: r.get(2)?,
                nomor_faktur: r.get(3)?,
                supplier_id: r.get(4)?,
                nama_supplier: r.get(5)?,
                total_utang_awal: r.get(6)?,
                total_dibayar: r.get(7)?,
                sisa_utang: r.get(8)?,
                status_utang: r.get(9)?,
                tanggal_jatuh_tempo: r.get(10)?,
                tanggal_pembelian: r.get(11)?,
            })
        })?;

        let mut list = Vec::new();
        for it in rows {
            list.push(it?);
        }
        Ok(list)
    }

    /// Mengambil riwayat cicilan pembayaran suatu kartu utang
    pub fn get_pembayaran_utang_history(
        db: &DbConnection,
        utang_id: i64,
    ) -> Result<Vec<PembayaranUtangItem>> {
        let conn = db.lock().unwrap();
        let sql = "
            SELECT 
                pembayaran_utang_id, nomor_bukti_bayar, utang_id, 
                tanggal_bayar, nominal_bayar, metode_bayar, keterangan, created_at
            FROM dat_pembayaran_utang
            WHERE utang_id = ?1 AND active_st = 1 AND deleted_st = 0
            ORDER BY pembayaran_utang_id DESC;
        ";

        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map([utang_id], |r| {
            Ok(PembayaranUtangItem {
                pembayaran_utang_id: r.get(0)?,
                nomor_bukti_bayar: r.get(1)?,
                utang_id: r.get(2)?,
                tanggal_bayar: r.get(3)?,
                nominal_bayar: r.get(4)?,
                metode_bayar: r.get(5)?,
                keterangan: r.get(6)?,
                created_at: r.get(7)?,
            })
        })?;

        let mut list = Vec::new();
        for it in rows {
            list.push(it?);
        }
        Ok(list)
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

        // Seed supplier and item
        {
            let conn = conn_arc.lock().unwrap();
            conn.execute(
                "INSERT INTO mst_supplier_pihak (
                    created_at, created_by, kode_pihak, nama_pihak, jenis_pihak
                ) VALUES ('2026-01-01T00:00:00Z', 1, 'SUP-001', 'PT Sumber Pangan Sejahtera', 'PERUSAHAAN');",
                [],
            ).unwrap();

            conn.execute(
                "INSERT INTO mst_barang (
                    created_at, created_by, kode_barang, nama_barang, 
                    kategori_id, satuan_id, harga_beli, harga_jual, stok_saat_ini, stok_minimum
                ) VALUES (
                    '2026-01-01T00:00:00Z', 1, 'BRG-BMB-1', 'Beras Ramos 5kg',
                    1, 1, 60000, 72000, 10, 5
                );",
                [],
            ).unwrap();
        }

        (db_manager, conn_arc)
    }

    #[test]
    fn test_cash_purchase_increases_stock_and_records_cash_outflow() {
        let (_manager, conn_arc) = setup_test_db();

        let input = CreatePembelianInput {
            nomor_faktur: "FAK-20261101-001".to_string(),
            tanggal_pembelian: "2026-11-01".to_string(),
            supplier_id: 1,
            jenis_pembayaran: "TUNAI".to_string(),
            items: vec![CreatePembelianItem {
                barang_id: 1,
                kuantitas: 20,
                harga_beli_satuan: 60000,
            }],
            nominal_dibayar: 1200000,
            jatuh_tempo: None,
            catatan: Some("Beli beras tunai".to_string()),
            user_id: Some(1),
        };

        let pembelian_id = PurchaseService::create_pembelian(&conn_arc, input).unwrap();
        assert!(pembelian_id > 0);

        let conn = conn_arc.lock().unwrap();

        // 1. Stok bertambah: 10 + 20 = 30
        let stok: i64 = conn
            .query_row(
                "SELECT stok_saat_ini FROM mst_barang WHERE barang_id = 1;",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(stok, 30);

        // 2. Tidak ada utang untuk pembelian tunai
        let utang_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM dat_utang WHERE pembelian_id = ?1;",
                [pembelian_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(utang_count, 0);

        // 3. Arus kas keluar tercatat: 1.200.000
        let kas_keluar: i64 = conn
            .query_row(
                "SELECT nominal FROM dat_pergerakan_uang WHERE referensi_tabel = 'dat_pembelian' AND referensi_id = ?1;",
                [pembelian_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(kas_keluar, 1200000);
    }

    #[test]
    fn test_credit_purchase_and_debt_installment() {
        let (_manager, conn_arc) = setup_test_db();

        // 1. Pembelian Kredit: total 1.000.000, DP 200.000, sisa utang 800.000
        let input = CreatePembelianInput {
            nomor_faktur: "FAK-KREDIT-001".to_string(),
            tanggal_pembelian: "2026-11-01".to_string(),
            supplier_id: 1,
            jenis_pembayaran: "KREDIT".to_string(),
            items: vec![CreatePembelianItem {
                barang_id: 1,
                kuantitas: 20,
                harga_beli_satuan: 50000,
            }],
            nominal_dibayar: 200000,
            jatuh_tempo: Some("2026-11-30".to_string()),
            catatan: Some("Kredit tempo 30 hari".to_string()),
            user_id: Some(1),
        };

        let pembelian_id = PurchaseService::create_pembelian(&conn_arc, input).unwrap();
        assert!(pembelian_id > 0);

        // Periksa kartu utang terbuat
        let utang_list = PurchaseService::get_utang_list(&conn_arc, None).unwrap();
        assert_eq!(utang_list.len(), 1);
        let utang = &utang_list[0];
        assert_eq!(utang.total_utang_awal, 1000000);
        assert_eq!(utang.total_dibayar, 200000);
        assert_eq!(utang.sisa_utang, 800000);
        assert_eq!(utang.status_utang, "DIBAYAR_SEBAGIAN");

        // Periksa stok bertambah saat pembelian kredit diterima
        {
            let conn = conn_arc.lock().unwrap();
            let stok: i64 = conn
                .query_row(
                    "SELECT stok_saat_ini FROM mst_barang WHERE barang_id = 1;",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(stok, 30); // 10 awal + 20 kredit
        }

        // 2. Bayar cicilan utang pertama: 300.000
        let bayar_input = BayarUtangInput {
            utang_id: utang.utang_id,
            tanggal_bayar: "2026-11-05".to_string(),
            nominal_bayar: 300000,
            metode_bayar: "TRANSFER".to_string(),
            keterangan: Some("Cicilan tahap 1".to_string()),
            user_id: Some(1),
        };
        let bayar_id = PurchaseService::bayar_utang(&conn_arc, bayar_input).unwrap();
        assert!(bayar_id > 0);

        // Sisa utang sekarang: 800.000 - 300.000 = 500.000
        let utang_after = &PurchaseService::get_utang_list(&conn_arc, None).unwrap()[0];
        assert_eq!(utang_after.sisa_utang, 500000);
        assert_eq!(utang_after.total_dibayar, 500000);
        assert_eq!(utang_after.status_utang, "DIBAYAR_SEBAGIAN");

        // Stok barang TETAP 30 (pembayaran utang TIDAK mengubah stok!)
        {
            let conn = conn_arc.lock().unwrap();
            let stok: i64 = conn
                .query_row(
                    "SELECT stok_saat_ini FROM mst_barang WHERE barang_id = 1;",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(stok, 30);
        }

        // 3. Pelunasan sisa utang: 500.000
        let lunas_input = BayarUtangInput {
            utang_id: utang.utang_id,
            tanggal_bayar: "2026-11-10".to_string(),
            nominal_bayar: 500000,
            metode_bayar: "TRANSFER".to_string(),
            keterangan: Some("Pelunasan akhir".to_string()),
            user_id: Some(1),
        };
        PurchaseService::bayar_utang(&conn_arc, lunas_input).unwrap();

        let utang_lunas = &PurchaseService::get_utang_list(&conn_arc, None).unwrap()[0];
        assert_eq!(utang_lunas.sisa_utang, 0);
        assert_eq!(utang_lunas.total_dibayar, 1000000);
        assert_eq!(utang_lunas.status_utang, "LUNAS");
    }
}
