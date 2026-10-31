use crate::database::connection::DbConnection;
use rusqlite::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSummary {
    pub omset_hari_ini: i64,
    pub transaksi_hari_ini: i64,
    pub omset_bulan_ini: i64,
    pub laba_kotor_hari_ini: i64,
    pub total_barang_aktif: i64,
    pub stok_menipis: i64,
    pub stok_habis: i64,
    pub sisa_utang: i64,
    pub kas_masuk_hari_ini: i64,
    pub kas_keluar_hari_ini: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaporanPenjualanSummary {
    pub total_omset: i64,
    pub total_diskon: i64,
    pub total_transaksi: i64,
    pub total_laba_kotor: i64,
    pub transaksi_tunai: i64,
    pub transaksi_qris: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PenjualanReportRow {
    pub penjualan_id: i64,
    pub nomor_struk: String,
    pub tanggal_penjualan: String,
    pub total_kotor: i64,
    pub total_diskon: i64,
    pub total_bersih: i64,
    pub metode_pembayaran: String,
    pub total_hpp: i64,
    pub laba_kotor: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashFlowReportRow {
    pub pergerakan_uang_id: i64,
    pub tanggal_waktu: String,
    pub jenis_arus: String,
    pub kategori_transaksi: String,
    pub nominal: i64,
    pub metode_pembayaran: String,
    pub referensi_tabel: Option<String>,
    pub keterangan: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductProfitReportRow {
    pub barang_id: i64,
    pub kode_barang: String,
    pub nama_barang: String,
    pub nama_kategori: String,
    pub kuantitas_terjual: i64,
    pub total_omset: i64,
    pub total_hpp: i64,
    pub laba_kotor: i64,
    pub margin_persen: f64,
}

pub struct ReportService;

impl ReportService {
    /// Mengambil ringkasan metrik eksekutif untuk dashboard utama
    pub fn get_dashboard_summary(db: &DbConnection) -> Result<DashboardSummary> {
        let conn = db.lock().unwrap();
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let this_month = chrono::Utc::now().format("%Y-%m").to_string();

        // 1. Penjualan Hari Ini & Bulan Ini
        let (omset_hari_ini, transaksi_hari_ini): (i64, i64) = conn.query_row(
            "SELECT COALESCE(SUM(total_bersih), 0), COUNT(*) 
             FROM dat_penjualan 
             WHERE tanggal_penjualan = ?1 AND status_pembayaran = 'SUCCESS' AND active_st = 1;",
            [&today],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;

        let omset_bulan_ini: i64 = conn.query_row(
            "SELECT COALESCE(SUM(total_bersih), 0) 
             FROM dat_penjualan 
             WHERE tanggal_penjualan LIKE (?1 || '%') AND status_pembayaran = 'SUCCESS' AND active_st = 1;",
            [&this_month],
            |r| r.get(0),
        )?;

        // 2. Laba Kotor Hari Ini (Total Bersih - Total HPP Snapshot)
        let laba_kotor_hari_ini: i64 = conn.query_row(
            "SELECT COALESCE(SUM(dp.subtotal - (dp.kuantitas * dp.harga_beli_snapshot)), 0)
             FROM dat_detail_penjualan dp
             JOIN dat_penjualan p ON dp.penjualan_id = p.penjualan_id
             WHERE p.tanggal_penjualan = ?1 AND p.status_pembayaran = 'SUCCESS' AND p.active_st = 1;",
            [&today],
            |r| r.get(0),
        )?;

        // 3. Status Master Barang
        let (total_barang_aktif, stok_menipis, stok_habis): (i64, i64, i64) = conn.query_row(
            "SELECT 
                COUNT(*),
                COALESCE(SUM(CASE WHEN stok_saat_ini <= stok_minimum AND stok_saat_ini > 0 THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN stok_saat_ini <= 0 THEN 1 ELSE 0 END), 0)
             FROM mst_barang
             WHERE active_st = 1 AND deleted_st = 0;",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;

        // 4. Sisa Utang Aktif
        let sisa_utang: i64 = conn.query_row(
            "SELECT COALESCE(SUM(sisa_utang), 0) 
             FROM dat_utang 
             WHERE status_utang != 'LUNAS' AND active_st = 1 AND deleted_st = 0;",
            [],
            |r| r.get(0),
        )?;

        // 5. Arus Kas Hari Ini
        let (kas_masuk_hari_ini, kas_keluar_hari_ini): (i64, i64) = conn.query_row(
            "SELECT 
                COALESCE(SUM(CASE WHEN jenis_arus = 'MASUK' THEN nominal ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN jenis_arus = 'KELUAR' THEN nominal ELSE 0 END), 0)
             FROM dat_pergerakan_uang
             WHERE tanggal_waktu LIKE (?1 || '%') AND active_st = 1;",
            [&today],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;

        Ok(DashboardSummary {
            omset_hari_ini,
            transaksi_hari_ini,
            omset_bulan_ini,
            laba_kotor_hari_ini,
            total_barang_aktif,
            stok_menipis,
            stok_habis,
            sisa_utang,
            kas_masuk_hari_ini,
            kas_keluar_hari_ini,
        })
    }

    /// Laporan Penjualan terfilter rentang tanggal
    pub fn get_sales_report(
        db: &DbConnection,
        start_date: &str,
        end_date: &str,
    ) -> Result<(LaporanPenjualanSummary, Vec<PenjualanReportRow>)> {
        let conn = db.lock().unwrap();

        // 1. Ambil summary
        let summary_sql = "
            SELECT 
                COALESCE(SUM(total_bersih), 0),
                COALESCE(SUM(total_diskon), 0),
                COUNT(*),
                COALESCE(SUM(CASE WHEN metode_pembayaran = 'TUNAI' THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN metode_pembayaran = 'QRIS' THEN 1 ELSE 0 END), 0)
            FROM dat_penjualan
            WHERE tanggal_penjualan BETWEEN ?1 AND ?2 
              AND status_pembayaran = 'SUCCESS' 
              AND active_st = 1;
        ";

        let (total_omset, total_diskon, total_transaksi, transaksi_tunai, transaksi_qris): (
            i64,
            i64,
            i64,
            i64,
            i64,
        ) = conn.query_row(summary_sql, [start_date, end_date], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })?;

        // 2. Ambil list rincian struk & hitung HPP
        let list_sql = "
            SELECT 
                p.penjualan_id, p.nomor_struk, p.tanggal_penjualan,
                p.total_kotor, p.total_diskon, p.total_bersih, p.metode_pembayaran,
                COALESCE(SUM(dp.kuantitas * dp.harga_beli_snapshot), 0) as total_hpp
            FROM dat_penjualan p
            LEFT JOIN dat_detail_penjualan dp ON p.penjualan_id = dp.penjualan_id
            WHERE p.tanggal_penjualan BETWEEN ?1 AND ?2 
              AND p.status_pembayaran = 'SUCCESS' 
              AND p.active_st = 1
            GROUP BY p.penjualan_id
            ORDER BY p.penjualan_id DESC;
        ";

        let mut stmt = conn.prepare(list_sql)?;
        let rows = stmt.query_map([start_date, end_date], |r| {
            let total_bersih: i64 = r.get(5)?;
            let total_hpp: i64 = r.get(7)?;
            let laba_kotor = total_bersih - total_hpp;

            Ok(PenjualanReportRow {
                penjualan_id: r.get(0)?,
                nomor_struk: r.get(1)?,
                tanggal_penjualan: r.get(2)?,
                total_kotor: r.get(3)?,
                total_diskon: r.get(4)?,
                total_bersih,
                metode_pembayaran: r.get(6)?,
                total_hpp,
                laba_kotor,
            })
        })?;

        let mut list = Vec::new();
        let mut total_laba_kotor: i64 = 0;
        for r in rows {
            let row = r?;
            total_laba_kotor += row.laba_kotor;
            list.push(row);
        }

        let summary = LaporanPenjualanSummary {
            total_omset,
            total_diskon,
            total_transaksi,
            total_laba_kotor,
            transaksi_tunai,
            transaksi_qris,
        };

        Ok((summary, list))
    }

    /// Laporan Arus Kas (Cash Flow)
    pub fn get_cash_flow_report(
        db: &DbConnection,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<CashFlowReportRow>> {
        let conn = db.lock().unwrap();
        let sql = "
            SELECT 
                pergerakan_uang_id, tanggal_waktu, jenis_arus, 
                kategori_transaksi, nominal, metode_pembayaran, 
                referensi_tabel, keterangan
            FROM dat_pergerakan_uang
            WHERE substr(tanggal_waktu, 1, 10) BETWEEN ?1 AND ?2 
              AND active_st = 1
            ORDER BY pergerakan_uang_id DESC;
        ";

        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map([start_date, end_date], |r| {
            Ok(CashFlowReportRow {
                pergerakan_uang_id: r.get(0)?,
                tanggal_waktu: r.get(1)?,
                jenis_arus: r.get(2)?,
                kategori_transaksi: r.get(3)?,
                nominal: r.get(4)?,
                metode_pembayaran: r.get(5)?,
                referensi_tabel: r.get(6)?,
                keterangan: r.get(7)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    /// Laporan Laba/Rugi per Barang (Product Profitability)
    pub fn get_product_profit_report(
        db: &DbConnection,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<ProductProfitReportRow>> {
        let conn = db.lock().unwrap();
        let sql = "
            SELECT 
                b.barang_id, b.kode_barang, b.nama_barang, k.nama_kategori,
                SUM(dp.kuantitas) as qty_terjual,
                SUM(dp.subtotal) as omset,
                SUM(dp.kuantitas * dp.harga_beli_snapshot) as total_hpp
            FROM dat_detail_penjualan dp
            JOIN dat_penjualan p ON dp.penjualan_id = p.penjualan_id
            JOIN mst_barang b ON dp.barang_id = b.barang_id
            JOIN mst_kategori_barang k ON b.kategori_id = k.kategori_id
            WHERE p.tanggal_penjualan BETWEEN ?1 AND ?2 
              AND p.status_pembayaran = 'SUCCESS'
            GROUP BY b.barang_id
            ORDER BY omset DESC;
        ";

        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map([start_date, end_date], |r| {
            let total_omset: i64 = r.get(5)?;
            let total_hpp: i64 = r.get(6)?;
            let laba_kotor = total_omset - total_hpp;
            let margin_persen = if total_omset > 0 {
                (laba_kotor as f64 / total_omset as f64) * 100.0
            } else {
                0.0
            };

            Ok(ProductProfitReportRow {
                barang_id: r.get(0)?,
                kode_barang: r.get(1)?,
                nama_barang: r.get(2)?,
                nama_kategori: r.get(3)?,
                kuantitas_terjual: r.get(4)?,
                total_omset,
                total_hpp,
                laba_kotor,
                margin_persen,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
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
        (db_manager, conn_arc)
    }

    #[test]
    fn test_dashboard_summary_and_sales_report() {
        let (_manager, conn_arc) = setup_test_db();
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

        // 1. Seed barang
        {
            let conn = conn_arc.lock().unwrap();
            conn.execute(
                "INSERT INTO mst_barang (
                    created_at, created_by, kode_barang, nama_barang, 
                    kategori_id, satuan_id, harga_beli, harga_jual, stok_saat_ini, stok_minimum
                ) VALUES (
                    '2026-01-01T00:00:00Z', 1, 'TEST-BRG', 'Kopi Arabika',
                    1, 1, 20000, 30000, 15, 5
                );",
                [],
            )
            .unwrap();

            // Seed penjualan: 2 pcs @ 30.000 (total bersih 60.000, HPP 40.000, laba 20.000)
            conn.execute(
                "INSERT INTO dat_penjualan (
                    created_at, created_by, nomor_struk, tanggal_penjualan, 
                    total_kotor, total_diskon, total_bersih, metode_pembayaran, 
                    nominal_diterima, nominal_kembalian, status_pembayaran
                ) VALUES (
                    '2026-01-01T00:00:00Z', 1, 'TRX-REP-01', ?1,
                    60000, 0, 60000, 'TUNAI', 60000, 0, 'SUCCESS'
                );",
                [&today],
            )
            .unwrap();

            conn.execute(
                "INSERT INTO dat_detail_penjualan (
                    created_at, created_by, penjualan_id, barang_id, 
                    harga_beli_snapshot, harga_jual_satuan, kuantitas, diskon_item, subtotal
                ) VALUES (
                    '2026-01-01T00:00:00Z', 1, 1, 1, 20000, 30000, 2, 0, 60000
                );",
                [],
            )
            .unwrap();

            conn.execute(
                "INSERT INTO dat_pergerakan_uang (
                    created_at, created_by, tanggal_waktu, jenis_arus, 
                    kategori_transaksi, nominal, metode_pembayaran, referensi_tabel, referensi_id
                ) VALUES (
                    '2026-01-01T00:00:00Z', 1, ?1, 'MASUK', 'PENJUALAN_KASIR', 60000, 'TUNAI', 'dat_penjualan', 1
                );",
                [&today],
            )
            .unwrap();
        }

        // 2. Cek get_dashboard_summary
        let summary = ReportService::get_dashboard_summary(&conn_arc).unwrap();
        assert_eq!(summary.omset_hari_ini, 60000);
        assert_eq!(summary.transaksi_hari_ini, 1);
        assert_eq!(summary.laba_kotor_hari_ini, 20000);
        assert_eq!(summary.kas_masuk_hari_ini, 60000);

        // 3. Cek get_sales_report
        let (sales_sum, list) =
            ReportService::get_sales_report(&conn_arc, "2026-01-01", "2026-12-31").unwrap();
        assert_eq!(sales_sum.total_omset, 60000);
        assert_eq!(sales_sum.total_laba_kotor, 20000);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].laba_kotor, 20000);

        // 4. Cek product profit report
        let prod_report =
            ReportService::get_product_profit_report(&conn_arc, "2026-01-01", "2026-12-31").unwrap();
        assert_eq!(prod_report.len(), 1);
        assert_eq!(prod_report[0].kuantitas_terjual, 2);
        assert_eq!(prod_report[0].laba_kotor, 20000);
    }
}
