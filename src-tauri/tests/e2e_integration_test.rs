use olahpos_lib::database::connection::DbManager;
use olahpos_lib::database::migrator::Migrator;
use olahpos_lib::services::auth::AuthService;
use olahpos_lib::services::backup_service::BackupService;
use olahpos_lib::services::integration_service::IntegrationService;
use olahpos_lib::services::inventory_service::{InventoryService, StockOpnameInput};
use olahpos_lib::services::master_service::{MasterService, SaveBarangInput};
use olahpos_lib::services::printer_service::{PrinterConfig, PrinterService, ReceiptPrintData, ReceiptPrintItem};
use olahpos_lib::services::purchase_service::{CreatePembelianInput, CreatePembelianItem, PurchaseService};
use olahpos_lib::services::report_service::ReportService;
use olahpos_lib::services::sales_service::{CartItemInput, CheckoutInput, SalesService};
use serde_json::json;

#[test]
fn test_full_e2e_pos_and_inventory_lifecycle() {
    // 1. Inisialisasi Database In-Memory & Migrasi
    let db_manager = DbManager::init_in_memory().expect("Failed in-memory DB");
    let conn_arc = db_manager.get_connection();
    {
        let mut conn = conn_arc.lock().unwrap();
        Migrator::run(&mut conn).expect("Failed running migrations");
    }

    // 2. Auth & Developer Bootstrap
    AuthService::bootstrap_developer_if_needed(&conn_arc).expect("Failed to bootstrap developer");
    let session = AuthService::login(&conn_arc, "developer", "developer123").expect("Login failed");
    assert_eq!(session.nama_pengguna, "developer");
    assert_eq!(session.peran, "ADMIN");
    let user_id = session.pengguna_id;

    // 3. Master Data: Tambah Barang
    let input_barang = SaveBarangInput {
        barang_id: None,
        kode_barang: "BRG-E2E-001".to_string(),
        barcode: Some("8991234567890".to_string()),
        nama_barang: "Teh Botol Melati 350ml".to_string(),
        kategori_id: 3, // Minuman (dari seed)
        satuan_id: 1,   // PCS (dari seed)
        harga_beli: 3000,
        harga_jual: 5000,
        stok_minimum: 10,
        catatan: Some("Produk uji coba e2e".to_string()),
    };
    let barang_id = MasterService::save_barang(&conn_arc, input_barang, Some(user_id)).expect("Failed to save barang");
    assert!(barang_id > 0);

    // Seed Supplier Pihak Ketiga
    {
        let conn = conn_arc.lock().unwrap();
        conn.execute(
            "INSERT INTO mst_supplier_pihak (
                created_at, created_by, kode_pihak, nama_pihak, jenis_pihak
            ) VALUES ('2026-01-01T00:00:00Z', 1, 'SUP-E2E-1', 'PT Sumber Harapan', 'PERUSAHAAN');",
            [],
        ).expect("Failed seed supplier");
    }

    // 4. Transaksi Pembelian Tunai (Beli 50 botol @ 3.000 = 150.000)
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let purchase_input = CreatePembelianInput {
        nomor_faktur: "FAK-E2E-2026-001".to_string(),
        tanggal_pembelian: today.clone(),
        supplier_id: 1,
        jenis_pembayaran: "TUNAI".to_string(),
        items: vec![CreatePembelianItem {
            barang_id,
            kuantitas: 50,
            harga_beli_satuan: 3000,
        }],
        nominal_dibayar: 150000,
        jatuh_tempo: None,
        catatan: Some("Restock awal 50 botol".to_string()),
        user_id: Some(user_id),
    };
    let pembelian_id = PurchaseService::create_pembelian(&conn_arc, purchase_input).expect("Purchase failed");
    assert!(pembelian_id > 0);

    // Cek stok setelah pembelian: 50
    {
        let conn = conn_arc.lock().unwrap();
        let stok: i64 = conn
            .query_row("SELECT stok_saat_ini FROM mst_barang WHERE barang_id = ?1;", [barang_id], |r| r.get(0))
            .unwrap();
        assert_eq!(stok, 50);

        // Arus kas keluar tercatat
        let kas_keluar: i64 = conn
            .query_row(
                "SELECT nominal FROM dat_pergerakan_uang WHERE referensi_tabel = 'dat_pembelian' AND referensi_id = ?1;",
                [pembelian_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(kas_keluar, 150000);
    }

    // 5. Transaksi Kasir POS (Jual 5 botol @ 5.000 = 25.000, bayar 50.000, kembali 25.000)
    let checkout_input = CheckoutInput {
        nomor_struk: "STRUK-E2E-001".to_string(),
        tanggal_penjualan: today.clone(),
        items: vec![CartItemInput {
            barang_id,
            kuantitas: 5,
            harga_jual_satuan: 5000,
            diskon_item: 0,
        }],
        diskon_transaksi: 0,
        metode_pembayaran: "TUNAI".to_string(),
        nominal_diterima: 50000,
        midtrans_transaction_id: None,
        catatan: Some("Pelanggan umum".to_string()),
        user_id: Some(user_id),
    };
    let receipt = SalesService::process_checkout(&conn_arc, checkout_input).expect("Checkout failed");
    assert_eq!(receipt.total_bersih, 25000);
    assert_eq!(receipt.nominal_kembalian, 25000);
    assert_eq!(receipt.status_pembayaran, "SUCCESS");

    // Cek stok setelah penjualan: 50 - 5 = 45
    {
        let conn = conn_arc.lock().unwrap();
        let stok: i64 = conn
            .query_row("SELECT stok_saat_ini FROM mst_barang WHERE barang_id = ?1;", [barang_id], |r| r.get(0))
            .unwrap();
        assert_eq!(stok, 45);

        // Arus kas masuk tercatat
        let kas_masuk: i64 = conn
            .query_row(
                "SELECT nominal FROM dat_pergerakan_uang WHERE referensi_tabel = 'dat_penjualan' AND referensi_id = ?1;",
                [receipt.penjualan_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(kas_masuk, 25000);
    }

    // 6. Cetak Struk Thermal (ESC/POS & Text Preview 58mm)
    let print_data = ReceiptPrintData {
        store_name: "OlahPOS Store".to_string(),
        store_address: "Jl. Sudirman No. 45".to_string(),
        store_phone: "081298765432".to_string(),
        receipt_footer: "Terima kasih atas kunjungan Anda!".to_string(),
        nomor_struk: receipt.nomor_struk.clone(),
        tanggal_penjualan: today.clone(),
        kasir_nama: "developer".to_string(),
        items: vec![ReceiptPrintItem {
            nama_barang: "Teh Botol Melati 350ml".to_string(),
            kuantitas: 5,
            harga_satuan: 5000,
            subtotal: 25000,
        }],
        total_kotor: 25000,
        total_diskon: 0,
        total_bersih: 25000,
        metode_pembayaran: "TUNAI".to_string(),
        nominal_diterima: 50000,
        nominal_kembalian: 25000,
    };
    let cfg = PrinterConfig::default();
    let preview_text = PrinterService::generate_text_preview(&print_data, &cfg);
    assert!(preview_text.contains("OlahPOS Store"));
    assert!(preview_text.contains("STRUK-E2E-001"));
    assert!(preview_text.contains("25.000"));

    let esc_bytes = PrinterService::generate_esc_pos_bytes(&print_data, &cfg);
    assert!(!esc_bytes.is_empty());

    // 7. Integrasi QRIS Midtrans Terenkripsi Simetris (AES-256-GCM)
    let midtrans_payload = json!({
        "server_key": "SB-Mid-server-test-key-xyz",
        "client_key": "SB-Mid-client-test-key-xyz",
        "merchant_id": "G999888777",
        "environment": "sandbox"
    });
    IntegrationService::save_config(
        &conn_arc,
        "MIDTRANS_QRIS",
        "Midtrans Payment Gateway",
        &midtrans_payload,
        Some("Konfigurasi Sandbox".to_string()),
        Some(user_id),
    ).expect("Save encrypted config failed");

    let loaded_config = IntegrationService::get_config(&conn_arc, "MIDTRANS_QRIS")
        .expect("Fetch config failed")
        .expect("Config should exist");
    assert_eq!(loaded_config["server_key"], "SB-Mid-server-test-key-xyz");

    // 8. Stock Opname (Fisik 40 botol karena 5 pecah)
    let opname_id = InventoryService::process_stock_opname(
        &conn_arc,
        &StockOpnameInput {
            barang_id,
            stok_fisik: 40,
            keterangan: Some("5 botol pecah di gudang".to_string()),
            user_id: Some(user_id),
        },
    ).expect("Opname failed");
    assert!(opname_id > 0);

    // Stok akhir adalah 40
    {
        let conn = conn_arc.lock().unwrap();
        let stok_akhir: i64 = conn
            .query_row("SELECT stok_saat_ini FROM mst_barang WHERE barang_id = ?1;", [barang_id], |r| r.get(0))
            .unwrap();
        assert_eq!(stok_akhir, 40);
    }

    // 9. Dashboard Analytics & Financial Summary
    let dashboard = ReportService::get_dashboard_summary(&conn_arc).expect("Dashboard failed");
    assert_eq!(dashboard.omset_hari_ini, 25000);
    assert_eq!(dashboard.transaksi_hari_ini, 1);
    assert_eq!(dashboard.laba_kotor_hari_ini, 10000); // 5 * (5000 - 3000)
    assert_eq!(dashboard.kas_masuk_hari_ini, 25000);
    assert_eq!(dashboard.kas_keluar_hari_ini, 150000);

    // 10. Backup Snapshot & Retention (Isolated temp directory)
    let temp_backup_dir = std::env::temp_dir().join(format!("olahpos_bkp_{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)));
    let temp_backups_dest = temp_backup_dir.join("backups");
    let _ = std::fs::create_dir_all(&temp_backups_dest);
    let dummy_main = temp_backup_dir.join("main.db");
    let dummy_log = temp_backup_dir.join("log.db");
    {
        let conn_main = rusqlite::Connection::open(&dummy_main).unwrap();
        conn_main.execute("CREATE TABLE test (id INT);", []).unwrap();
        let conn_log = rusqlite::Connection::open(&dummy_log).unwrap();
        conn_log.execute("CREATE TABLE test_log (id INT);", []).unwrap();
    }

    let backup_res = BackupService::create_backup_snapshot_in(&conn_arc, &dummy_main, &dummy_log, &temp_backups_dest)
        .expect("Backup snapshot failed");
    assert!(backup_res.path.contains("backup_"));

    let backups = BackupService::list_backups_in(&temp_backups_dest).expect("List backups failed");
    assert!(!backups.is_empty());

    let _ = std::fs::remove_dir_all(&temp_backup_dir);
}
