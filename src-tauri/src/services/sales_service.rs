use crate::database::audit::{AuditAction, AuditLogger};
use crate::database::connection::DbConnection;
use crate::services::inventory_service::{InventoryService, RecordMutationInput};
use rusqlite::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CartItemInput {
    pub barang_id: i64,
    pub kuantitas: i64,
    pub harga_jual_satuan: i64,
    pub diskon_item: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckoutInput {
    pub nomor_struk: String,
    pub tanggal_penjualan: String,
    pub items: Vec<CartItemInput>,
    pub diskon_transaksi: i64,
    pub metode_pembayaran: String, // "TUNAI" | "QRIS"
    pub nominal_diterima: i64,
    pub midtrans_transaction_id: Option<String>,
    pub catatan: Option<String>,
    pub user_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptItem {
    pub barang_id: i64,
    pub kode_barang: String,
    pub nama_barang: String,
    pub nama_satuan: String,
    pub kuantitas: i64,
    pub harga_jual_satuan: i64,
    pub diskon_item: i64,
    pub subtotal: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckoutReceiptResult {
    pub penjualan_id: i64,
    pub nomor_struk: String,
    pub tanggal_penjualan: String,
    pub total_kotor: i64,
    pub total_diskon: i64,
    pub total_bersih: i64,
    pub nominal_diterima: i64,
    pub nominal_kembalian: i64,
    pub metode_pembayaran: String,
    pub status_pembayaran: String,
    pub items: Vec<ReceiptItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PenjualanListItem {
    pub penjualan_id: i64,
    pub nomor_struk: String,
    pub tanggal_penjualan: String,
    pub total_kotor: i64,
    pub total_diskon: i64,
    pub total_bersih: i64,
    pub metode_pembayaran: String,
    pub nominal_diterima: i64,
    pub nominal_kembalian: i64,
    pub status_pembayaran: String,
    pub total_item: i64,
}

pub struct SalesService;

impl SalesService {
    /// Menjalankan transaksi penjualan POS secara atomik.
    /// Memotong stok barang, mencatat mutasi stok, mencatat arus kas masuk, dan menyimpan detail struk.
    pub fn process_checkout(
        db: &DbConnection,
        input: CheckoutInput,
    ) -> std::result::Result<CheckoutReceiptResult, String> {
        if input.items.is_empty() {
            return Err("Keranjang belanja tidak boleh kosong!".to_string());
        }

        let now = chrono::Utc::now().to_rfc3339();

        // 1. Hitung total kotor dan total diskon
        let mut total_kotor: i64 = 0;
        let mut total_diskon_item: i64 = 0;

        for it in &input.items {
            if it.kuantitas <= 0 {
                return Err("Kuantitas barang harus lebih dari 0!".to_string());
            }
            if it.harga_jual_satuan < 0 {
                return Err("Harga jual tidak boleh negatif!".to_string());
            }
            total_kotor += it.kuantitas * it.harga_jual_satuan;
            total_diskon_item += it.diskon_item;
        }

        let total_diskon = total_diskon_item + input.diskon_transaksi;
        let total_bersih = total_kotor - total_diskon;
        if total_bersih < 0 {
            return Err("Total bersih penjualan tidak boleh negatif!".to_string());
        }

        // Validasi nominal diterima jika TUNAI
        let (nominal_kembalian, status_pembayaran) = match input.metode_pembayaran.as_str() {
            "TUNAI" => {
                if input.nominal_diterima < total_bersih {
                    return Err(format!(
                        "Nominal pembayaran kurang! Tagihan: {}, Diterima: {}",
                        total_bersih, input.nominal_diterima
                    ));
                }
                (input.nominal_diterima - total_bersih, "SUCCESS".to_string())
            }
            "QRIS" => (0, "SUCCESS".to_string()),
            other => return Err(format!("Metode pembayaran '{}' tidak dikenali!", other)),
        };

        let mut conn = db.lock().unwrap();
        let tx = conn
            .transaction()
            .map_err(|e| format!("Gagal memulai transaksi penjualan: {}", e))?;

        // 2. Insert ke dat_penjualan
        tx.execute(
            "INSERT INTO dat_penjualan (
                created_at, created_by, nomor_struk, tanggal_penjualan, 
                total_kotor, total_diskon, total_bersih, metode_pembayaran, 
                nominal_diterima, nominal_kembalian, status_pembayaran, 
                midtrans_transaction_id, catatan
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13);",
            rusqlite::params![
                now,
                input.user_id,
                input.nomor_struk,
                input.tanggal_penjualan,
                total_kotor,
                total_diskon,
                total_bersih,
                input.metode_pembayaran,
                input.nominal_diterima,
                nominal_kembalian,
                status_pembayaran,
                input.midtrans_transaction_id,
                input.catatan,
            ],
        )
        .map_err(|e| format!("Gagal mencatat penjualan: {}", e))?;

        let penjualan_id = tx.last_insert_rowid();

        // 3. Simpan detail item, potong stok, dan catat mutasi stok
        let mut receipt_items = Vec::new();

        for it in &input.items {
            // Snapshot harga beli saat transaksi & ambil data nama/satuan barang
            let (harga_beli_snapshot, kode_barang, nama_barang, nama_satuan): (
                i64,
                String,
                String,
                String,
            ) = tx
                .query_row(
                    "SELECT b.harga_beli, b.kode_barang, b.nama_barang, s.nama_satuan 
                     FROM mst_barang b 
                     JOIN mst_satuan_barang s ON b.satuan_id = s.satuan_id 
                     WHERE b.barang_id = ?1;",
                    [it.barang_id],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
                )
                .map_err(|e| format!("Barang id {} tidak valid: {}", it.barang_id, e))?;

            let subtotal = (it.kuantitas * it.harga_jual_satuan) - it.diskon_item;

            tx.execute(
                "INSERT INTO dat_detail_penjualan (
                    created_at, created_by, penjualan_id, barang_id, 
                    harga_beli_snapshot, harga_jual_satuan, kuantitas, 
                    diskon_item, subtotal
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9);",
                rusqlite::params![
                    now,
                    input.user_id,
                    penjualan_id,
                    it.barang_id,
                    harga_beli_snapshot,
                    it.harga_jual_satuan,
                    it.kuantitas,
                    it.diskon_item,
                    subtotal,
                ],
            )
            .map_err(|e| format!("Gagal mencatat detail penjualan barang {}: {}", it.barang_id, e))?;

            // Mutasi Stok: jenis_mutasi_id 3 = PENJUALAN
            let mutation_input = RecordMutationInput {
                barang_id: it.barang_id,
                jenis_mutasi_id: 3,
                stok_masuk: 0,
                stok_keluar: it.kuantitas,
                referensi_tabel: Some("dat_penjualan".to_string()),
                referensi_id: Some(penjualan_id),
                keterangan: Some(format!("Penjualan kasir struk {}", input.nomor_struk)),
                user_id: input.user_id,
            };
            InventoryService::record_mutation(&tx, &mutation_input)?;

            receipt_items.push(ReceiptItem {
                barang_id: it.barang_id,
                kode_barang,
                nama_barang,
                nama_satuan,
                kuantitas: it.kuantitas,
                harga_jual_satuan: it.harga_jual_satuan,
                diskon_item: it.diskon_item,
                subtotal,
            });
        }

        // 4. Catat arus kas MASUK di dat_pergerakan_uang
        tx.execute(
            "INSERT INTO dat_pergerakan_uang (
                created_at, created_by, tanggal_waktu, jenis_arus, 
                kategori_transaksi, nominal, metode_pembayaran, 
                referensi_tabel, referensi_id, keterangan
            ) VALUES (?1, ?2, ?3, 'MASUK', 'PENJUALAN_KASIR', ?4, ?5, 'dat_penjualan', ?6, ?7);",
            rusqlite::params![
                now,
                input.user_id,
                input.tanggal_penjualan,
                total_bersih,
                input.metode_pembayaran,
                penjualan_id,
                format!("Penerimaan penjualan struk {}", input.nomor_struk),
            ],
        )
        .map_err(|e| format!("Gagal mencatat arus kas penjualan: {}", e))?;

        // 5. Catat audit log untuk dat_penjualan
        AuditLogger::record(
            &tx,
            "dat_penjualan",
            &penjualan_id.to_string(),
            AuditAction::Insert,
            &json!({
                "penjualan_id": penjualan_id,
                "nomor_struk": input.nomor_struk,
                "total_bersih": total_bersih,
                "metode_pembayaran": input.metode_pembayaran,
                "nominal_diterima": input.nominal_diterima,
                "nominal_kembalian": nominal_kembalian
            }),
            input.user_id,
        )
        .map_err(|e| format!("Gagal mencatat audit log penjualan: {}", e))?;

        tx.commit()
            .map_err(|e| format!("Gagal commit transaksi penjualan: {}", e))?;

        Ok(CheckoutReceiptResult {
            penjualan_id,
            nomor_struk: input.nomor_struk,
            tanggal_penjualan: input.tanggal_penjualan,
            total_kotor,
            total_diskon,
            total_bersih,
            nominal_diterima: input.nominal_diterima,
            nominal_kembalian,
            metode_pembayaran: input.metode_pembayaran,
            status_pembayaran,
            items: receipt_items,
        })
    }

    /// Mengambil daftar riwayat transaksi kasir
    pub fn get_penjualan_list(
        db: &DbConnection,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<PenjualanListItem>> {
        let conn = db.lock().unwrap();
        let sql = "
            SELECT 
                p.penjualan_id, p.nomor_struk, p.tanggal_penjualan, 
                p.total_kotor, p.total_diskon, p.total_bersih, 
                p.metode_pembayaran, p.nominal_diterima, p.nominal_kembalian, 
                p.status_pembayaran,
                (SELECT COUNT(*) FROM dat_detail_penjualan dp WHERE dp.penjualan_id = p.penjualan_id) as total_item
            FROM dat_penjualan p
            WHERE p.active_st = 1 AND p.deleted_st = 0
            ORDER BY p.penjualan_id DESC
            LIMIT ?1 OFFSET ?2;
        ";

        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map([limit, offset], |r| {
            Ok(PenjualanListItem {
                penjualan_id: r.get(0)?,
                nomor_struk: r.get(1)?,
                tanggal_penjualan: r.get(2)?,
                total_kotor: r.get(3)?,
                total_diskon: r.get(4)?,
                total_bersih: r.get(5)?,
                metode_pembayaran: r.get(6)?,
                nominal_diterima: r.get(7)?,
                nominal_kembalian: r.get(8)?,
                status_pembayaran: r.get(9)?,
                total_item: r.get(10)?,
            })
        })?;

        let mut list = Vec::new();
        for it in rows {
            list.push(it?);
        }
        Ok(list)
    }

    /// Mengambil detail barang dari sebuah struk penjualan
    pub fn get_receipt_detail(
        db: &DbConnection,
        penjualan_id: i64,
    ) -> Result<CheckoutReceiptResult> {
        let conn = db.lock().unwrap();
        let header_sql = "
            SELECT 
                penjualan_id, nomor_struk, tanggal_penjualan, 
                total_kotor, total_diskon, total_bersih, 
                nominal_diterima, nominal_kembalian, 
                metode_pembayaran, status_pembayaran
            FROM dat_penjualan 
            WHERE penjualan_id = ?1;
        ";

        let (
            penjualan_id,
            nomor_struk,
            tanggal_penjualan,
            total_kotor,
            total_diskon,
            total_bersih,
            nominal_diterima,
            nominal_kembalian,
            metode_pembayaran,
            status_pembayaran,
        ) = conn.query_row(header_sql, [penjualan_id], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
                r.get(7)?,
                r.get(8)?,
                r.get(9)?,
            ))
        })?;

        let detail_sql = "
            SELECT 
                dp.barang_id, b.kode_barang, b.nama_barang, s.nama_satuan, 
                dp.kuantitas, dp.harga_jual_satuan, dp.diskon_item, dp.subtotal
            FROM dat_detail_penjualan dp
            JOIN mst_barang b ON dp.barang_id = b.barang_id
            JOIN mst_satuan_barang s ON b.satuan_id = s.satuan_id
            WHERE dp.penjualan_id = ?1;
        ";

        let mut stmt = conn.prepare(detail_sql)?;
        let rows = stmt.query_map([penjualan_id], |r| {
            Ok(ReceiptItem {
                barang_id: r.get(0)?,
                kode_barang: r.get(1)?,
                nama_barang: r.get(2)?,
                nama_satuan: r.get(3)?,
                kuantitas: r.get(4)?,
                harga_jual_satuan: r.get(5)?,
                diskon_item: r.get(6)?,
                subtotal: r.get(7)?,
            })
        })?;

        let mut items = Vec::new();
        for it in rows {
            items.push(it?);
        }

        Ok(CheckoutReceiptResult {
            penjualan_id,
            nomor_struk,
            tanggal_penjualan,
            total_kotor,
            total_diskon,
            total_bersih,
            nominal_diterima,
            nominal_kembalian,
            metode_pembayaran,
            status_pembayaran,
            items,
        })
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

        // Seed 1 barang with 10 pcs stock
        {
            let conn = conn_arc.lock().unwrap();
            conn.execute(
                "INSERT INTO mst_barang (
                    created_at, created_by, kode_barang, barcode, nama_barang, 
                    kategori_id, satuan_id, harga_beli, harga_jual, stok_saat_ini, stok_minimum
                ) VALUES (
                    '2026-01-01T00:00:00Z', 1, 'BRG-POS-1', '8990001', 'Minyak Goreng 2L',
                    1, 1, 28000, 35000, 10, 2
                );",
                [],
            )
            .unwrap();
        }

        (db_manager, conn_arc)
    }

    #[test]
    fn test_successful_checkout_and_stock_reduction() {
        let (_manager, conn_arc) = setup_test_db();

        let input = CheckoutInput {
            nomor_struk: "TRX-20261101-0001".to_string(),
            tanggal_penjualan: "2026-11-01".to_string(),
            items: vec![CartItemInput {
                barang_id: 1,
                kuantitas: 3,
                harga_jual_satuan: 35000,
                diskon_item: 0,
            }],
            diskon_transaksi: 5000,
            metode_pembayaran: "TUNAI".to_string(),
            nominal_diterima: 150000,
            midtrans_transaction_id: None,
            catatan: None,
            user_id: Some(1),
        };

        // Total kotor: 3 * 35000 = 105.000
        // Diskon: 5.000 -> Total bersih = 100.000
        // Diterima: 150.000 -> Kembalian = 50.000
        let receipt = SalesService::process_checkout(&conn_arc, input).unwrap();

        assert_eq!(receipt.total_kotor, 105000);
        assert_eq!(receipt.total_diskon, 5000);
        assert_eq!(receipt.total_bersih, 100000);
        assert_eq!(receipt.nominal_kembalian, 50000);
        assert_eq!(receipt.status_pembayaran, "SUCCESS");

        // 1. Cek stok berkurang: 10 - 3 = 7
        let conn = conn_arc.lock().unwrap();
        let stok: i64 = conn
            .query_row(
                "SELECT stok_saat_ini FROM mst_barang WHERE barang_id = 1;",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(stok, 7);

        // 2. Cek mutasi stok tercatat
        let mutasi_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM dat_mutasi_stok WHERE referensi_tabel = 'dat_penjualan' AND stok_keluar = 3;",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(mutasi_count, 1);

        // 3. Cek arus kas masuk: 100.000
        let kas_masuk: i64 = conn
            .query_row(
                "SELECT nominal FROM dat_pergerakan_uang WHERE referensi_tabel = 'dat_penjualan' AND jenis_arus = 'MASUK';",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(kas_masuk, 100000);

        // 4. Cek audit log di log_db
        let log_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM log_db.dat_penjualan WHERE primary_key_id = ?1;",
                [receipt.penjualan_id.to_string()],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(log_count, 1);
    }

    #[test]
    fn test_insufficient_stock_prevents_checkout() {
        let (_manager, conn_arc) = setup_test_db();

        // Coba jual 15 pcs padahal stok hanya 10
        let input = CheckoutInput {
            nomor_struk: "TRX-FAIL-001".to_string(),
            tanggal_penjualan: "2026-11-01".to_string(),
            items: vec![CartItemInput {
                barang_id: 1,
                kuantitas: 15,
                harga_jual_satuan: 35000,
                diskon_item: 0,
            }],
            diskon_transaksi: 0,
            metode_pembayaran: "TUNAI".to_string(),
            nominal_diterima: 600000,
            midtrans_transaction_id: None,
            catatan: None,
            user_id: Some(1),
        };

        let result = SalesService::process_checkout(&conn_arc, input);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Stok barang tidak mencukupi"));

        // Pastikan tidak ada data tersimpan sama sekali (atomik rollback)
        let conn = conn_arc.lock().unwrap();
        let count_trx: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM dat_penjualan WHERE nomor_struk = 'TRX-FAIL-001';",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count_trx, 0);

        let stok: i64 = conn
            .query_row(
                "SELECT stok_saat_ini FROM mst_barang WHERE barang_id = 1;",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(stok, 10);
    }
}
