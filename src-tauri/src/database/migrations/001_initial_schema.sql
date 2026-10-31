-- 001_initial_schema.sql
-- OlahPOS Initial Base Schema and Master Seeds

-- ==========================================
-- 1. MASTER TABLES (mst_*)
-- ==========================================

-- Pengguna & Hak Akses
CREATE TABLE IF NOT EXISTS mst_pengguna (
    created_at TEXT NOT NULL,
    created_by INTEGER,
    updated_at TEXT,
    updated_by INTEGER,
    deleted_at TEXT,
    deleted_by INTEGER,
    deleted_st INTEGER DEFAULT 0,
    active_st INTEGER DEFAULT 1,
    pengguna_id INTEGER PRIMARY KEY AUTOINCREMENT,
    nama_lengkap TEXT NOT NULL,
    nama_pengguna TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    peran TEXT NOT NULL CHECK(peran IN ('ADMIN', 'CASHIER')),
    harus_ganti_password INTEGER DEFAULT 0,
    terakhir_login TEXT
);

-- Kategori Barang
CREATE TABLE IF NOT EXISTS mst_kategori_barang (
    created_at TEXT NOT NULL,
    created_by INTEGER,
    updated_at TEXT,
    updated_by INTEGER,
    deleted_at TEXT,
    deleted_by INTEGER,
    deleted_st INTEGER DEFAULT 0,
    active_st INTEGER DEFAULT 1,
    kategori_id INTEGER PRIMARY KEY AUTOINCREMENT,
    kode_kategori TEXT NOT NULL UNIQUE,
    nama_kategori TEXT NOT NULL,
    keterangan TEXT
);

-- Satuan Barang
CREATE TABLE IF NOT EXISTS mst_satuan_barang (
    created_at TEXT NOT NULL,
    created_by INTEGER,
    updated_at TEXT,
    updated_by INTEGER,
    deleted_at TEXT,
    deleted_by INTEGER,
    deleted_st INTEGER DEFAULT 0,
    active_st INTEGER DEFAULT 1,
    satuan_id INTEGER PRIMARY KEY AUTOINCREMENT,
    kode_satuan TEXT NOT NULL UNIQUE,
    nama_satuan TEXT NOT NULL
);

-- Jenis Mutasi Stok
CREATE TABLE IF NOT EXISTS mst_jenis_mutasi (
    created_at TEXT NOT NULL,
    created_by INTEGER,
    updated_at TEXT,
    updated_by INTEGER,
    deleted_at TEXT,
    deleted_by INTEGER,
    deleted_st INTEGER DEFAULT 0,
    active_st INTEGER DEFAULT 1,
    jenis_mutasi_id INTEGER PRIMARY KEY AUTOINCREMENT,
    kode_mutasi TEXT NOT NULL UNIQUE,
    nama_mutasi TEXT NOT NULL,
    dampak_stok TEXT NOT NULL CHECK(dampak_stok IN ('TAMBAH', 'KURANG', 'TETAP')),
    keterangan TEXT
);

-- Supplier & Pihak Terkait
CREATE TABLE IF NOT EXISTS mst_supplier_pihak (
    created_at TEXT NOT NULL,
    created_by INTEGER,
    updated_at TEXT,
    updated_by INTEGER,
    deleted_at TEXT,
    deleted_by INTEGER,
    deleted_st INTEGER DEFAULT 0,
    active_st INTEGER DEFAULT 1,
    supplier_id INTEGER PRIMARY KEY AUTOINCREMENT,
    kode_pihak TEXT NOT NULL UNIQUE,
    nama_pihak TEXT NOT NULL,
    jenis_pihak TEXT NOT NULL CHECK(jenis_pihak IN ('PERUSAHAAN', 'PERORANGAN')),
    nama_kontak TEXT,
    nomor_telepon TEXT,
    alamat TEXT,
    catatan TEXT
);

-- Data Barang
CREATE TABLE IF NOT EXISTS mst_barang (
    created_at TEXT NOT NULL,
    created_by INTEGER,
    updated_at TEXT,
    updated_by INTEGER,
    deleted_at TEXT,
    deleted_by INTEGER,
    deleted_st INTEGER DEFAULT 0,
    active_st INTEGER DEFAULT 1,
    barang_id INTEGER PRIMARY KEY AUTOINCREMENT,
    kode_barang TEXT NOT NULL UNIQUE,
    barcode TEXT UNIQUE,
    nama_barang TEXT NOT NULL,
    kategori_id INTEGER NOT NULL REFERENCES mst_kategori_barang(kategori_id),
    satuan_id INTEGER NOT NULL REFERENCES mst_satuan_barang(satuan_id),
    harga_beli INTEGER NOT NULL DEFAULT 0,
    harga_jual INTEGER NOT NULL DEFAULT 0,
    stok_saat_ini INTEGER NOT NULL DEFAULT 0,
    stok_minimum INTEGER NOT NULL DEFAULT 0,
    catatan TEXT
);

-- ==========================================
-- 2. TRANSACTION TABLES (dat_*)
-- ==========================================

-- Pembelian Barang
CREATE TABLE IF NOT EXISTS dat_pembelian (
    created_at TEXT NOT NULL,
    created_by INTEGER,
    updated_at TEXT,
    updated_by INTEGER,
    deleted_at TEXT,
    deleted_by INTEGER,
    deleted_st INTEGER DEFAULT 0,
    active_st INTEGER DEFAULT 1,
    pembelian_id INTEGER PRIMARY KEY AUTOINCREMENT,
    nomor_faktur TEXT NOT NULL UNIQUE,
    tanggal_pembelian TEXT NOT NULL,
    supplier_id INTEGER NOT NULL REFERENCES mst_supplier_pihak(supplier_id),
    jenis_pembayaran TEXT NOT NULL CHECK(jenis_pembayaran IN ('TUNAI', 'KREDIT')),
    total_tagihan INTEGER NOT NULL DEFAULT 0,
    nominal_dibayar INTEGER NOT NULL DEFAULT 0,
    sisa_utang INTEGER NOT NULL DEFAULT 0,
    jatuh_tempo TEXT,
    catatan TEXT
);

-- Detail Pembelian
CREATE TABLE IF NOT EXISTS dat_detail_pembelian (
    created_at TEXT NOT NULL,
    created_by INTEGER,
    updated_at TEXT,
    updated_by INTEGER,
    deleted_at TEXT,
    deleted_by INTEGER,
    deleted_st INTEGER DEFAULT 0,
    active_st INTEGER DEFAULT 1,
    detail_pembelian_id INTEGER PRIMARY KEY AUTOINCREMENT,
    pembelian_id INTEGER NOT NULL REFERENCES dat_pembelian(pembelian_id),
    barang_id INTEGER NOT NULL REFERENCES mst_barang(barang_id),
    kuantitas INTEGER NOT NULL,
    harga_beli_satuan INTEGER NOT NULL,
    subtotal INTEGER NOT NULL
);

-- Kartu Utang Pembelian
CREATE TABLE IF NOT EXISTS dat_utang (
    created_at TEXT NOT NULL,
    created_by INTEGER,
    updated_at TEXT,
    updated_by INTEGER,
    deleted_at TEXT,
    deleted_by INTEGER,
    deleted_st INTEGER DEFAULT 0,
    active_st INTEGER DEFAULT 1,
    utang_id INTEGER PRIMARY KEY AUTOINCREMENT,
    nomor_referensi TEXT NOT NULL UNIQUE,
    pembelian_id INTEGER NOT NULL REFERENCES dat_pembelian(pembelian_id),
    supplier_id INTEGER NOT NULL REFERENCES mst_supplier_pihak(supplier_id),
    total_utang_awal INTEGER NOT NULL,
    total_dibayar INTEGER NOT NULL DEFAULT 0,
    sisa_utang INTEGER NOT NULL,
    status_utang TEXT NOT NULL CHECK(status_utang IN ('BELUM_DIBAYAR', 'DIBAYAR_SEBAGIAN', 'LUNAS')),
    tanggal_jatuh_tempo TEXT
);

-- Riwayat Pembayaran Utang
CREATE TABLE IF NOT EXISTS dat_pembayaran_utang (
    created_at TEXT NOT NULL,
    created_by INTEGER,
    updated_at TEXT,
    updated_by INTEGER,
    deleted_at TEXT,
    deleted_by INTEGER,
    deleted_st INTEGER DEFAULT 0,
    active_st INTEGER DEFAULT 1,
    pembayaran_utang_id INTEGER PRIMARY KEY AUTOINCREMENT,
    nomor_bukti_bayar TEXT NOT NULL UNIQUE,
    utang_id INTEGER NOT NULL REFERENCES dat_utang(utang_id),
    tanggal_bayar TEXT NOT NULL,
    nominal_bayar INTEGER NOT NULL,
    metode_bayar TEXT NOT NULL CHECK(metode_bayar IN ('TUNAI', 'TRANSFER')),
    keterangan TEXT
);

-- Penjualan (Kasir / POS)
CREATE TABLE IF NOT EXISTS dat_penjualan (
    created_at TEXT NOT NULL,
    created_by INTEGER,
    updated_at TEXT,
    updated_by INTEGER,
    deleted_at TEXT,
    deleted_by INTEGER,
    deleted_st INTEGER DEFAULT 0,
    active_st INTEGER DEFAULT 1,
    penjualan_id INTEGER PRIMARY KEY AUTOINCREMENT,
    nomor_struk TEXT NOT NULL UNIQUE,
    tanggal_penjualan TEXT NOT NULL,
    total_kotor INTEGER NOT NULL DEFAULT 0,
    total_diskon INTEGER NOT NULL DEFAULT 0,
    total_bersih INTEGER NOT NULL DEFAULT 0,
    metode_pembayaran TEXT NOT NULL CHECK(metode_pembayaran IN ('TUNAI', 'QRIS')),
    nominal_diterima INTEGER NOT NULL DEFAULT 0,
    nominal_kembalian INTEGER NOT NULL DEFAULT 0,
    status_pembayaran TEXT NOT NULL CHECK(status_pembayaran IN ('PENDING', 'SUCCESS', 'FAILED')),
    midtrans_transaction_id TEXT,
    catatan TEXT
);

-- Detail Penjualan
CREATE TABLE IF NOT EXISTS dat_detail_penjualan (
    created_at TEXT NOT NULL,
    created_by INTEGER,
    updated_at TEXT,
    updated_by INTEGER,
    deleted_at TEXT,
    deleted_by INTEGER,
    deleted_st INTEGER DEFAULT 0,
    active_st INTEGER DEFAULT 1,
    detail_penjualan_id INTEGER PRIMARY KEY AUTOINCREMENT,
    penjualan_id INTEGER NOT NULL REFERENCES dat_penjualan(penjualan_id),
    barang_id INTEGER NOT NULL REFERENCES mst_barang(barang_id),
    harga_beli_snapshot INTEGER NOT NULL,
    harga_jual_satuan INTEGER NOT NULL,
    kuantitas INTEGER NOT NULL,
    diskon_item INTEGER NOT NULL DEFAULT 0,
    subtotal INTEGER NOT NULL
);

-- Mutasi Stok Terpadu
CREATE TABLE IF NOT EXISTS dat_mutasi_stok (
    created_at TEXT NOT NULL,
    created_by INTEGER,
    updated_at TEXT,
    updated_by INTEGER,
    deleted_at TEXT,
    deleted_by INTEGER,
    deleted_st INTEGER DEFAULT 0,
    active_st INTEGER DEFAULT 1,
    mutasi_id INTEGER PRIMARY KEY AUTOINCREMENT,
    barang_id INTEGER NOT NULL REFERENCES mst_barang(barang_id),
    jenis_mutasi_id INTEGER NOT NULL REFERENCES mst_jenis_mutasi(jenis_mutasi_id),
    tanggal_waktu TEXT NOT NULL,
    stok_masuk INTEGER NOT NULL DEFAULT 0,
    stok_keluar INTEGER NOT NULL DEFAULT 0,
    stok_sebelum INTEGER NOT NULL,
    stok_sesudah INTEGER NOT NULL,
    referensi_tabel TEXT,
    referensi_id INTEGER,
    keterangan TEXT
);

-- Pergerakan Arus Uang (Cash Flow)
CREATE TABLE IF NOT EXISTS dat_pergerakan_uang (
    created_at TEXT NOT NULL,
    created_by INTEGER,
    updated_at TEXT,
    updated_by INTEGER,
    deleted_at TEXT,
    deleted_by INTEGER,
    deleted_st INTEGER DEFAULT 0,
    active_st INTEGER DEFAULT 1,
    pergerakan_uang_id INTEGER PRIMARY KEY AUTOINCREMENT,
    tanggal_waktu TEXT NOT NULL,
    jenis_arus TEXT NOT NULL CHECK(jenis_arus IN ('MASUK', 'KELUAR')),
    kategori_transaksi TEXT NOT NULL,
    nominal INTEGER NOT NULL,
    metode_pembayaran TEXT NOT NULL,
    referensi_tabel TEXT,
    referensi_id INTEGER,
    keterangan TEXT
);

-- Konfigurasi Integrasi Terenkripsi (AES-256-GCM)
CREATE TABLE IF NOT EXISTS dat_integrasi_config (
    created_at TEXT NOT NULL,
    created_by INTEGER,
    updated_at TEXT,
    updated_by INTEGER,
    deleted_at TEXT,
    deleted_by INTEGER,
    deleted_st INTEGER DEFAULT 0,
    active_st INTEGER DEFAULT 1,
    config_id INTEGER PRIMARY KEY AUTOINCREMENT,
    kode_layanan TEXT NOT NULL UNIQUE,
    nama_layanan TEXT NOT NULL,
    payload_terenkripsi TEXT NOT NULL,
    catatan TEXT
);

-- Indexing untuk Kecepatan Pencarian & Query POS
CREATE INDEX IF NOT EXISTS idx_barang_barcode ON mst_barang(barcode);
CREATE INDEX IF NOT EXISTS idx_barang_kode ON mst_barang(kode_barang);
CREATE INDEX IF NOT EXISTS idx_barang_nama ON mst_barang(nama_barang);
CREATE INDEX IF NOT EXISTS idx_mutasi_barang ON dat_mutasi_stok(barang_id, tanggal_waktu);
CREATE INDEX IF NOT EXISTS idx_penjualan_tanggal ON dat_penjualan(tanggal_penjualan);
CREATE INDEX IF NOT EXISTS idx_pembelian_tanggal ON dat_pembelian(tanggal_pembelian);
CREATE INDEX IF NOT EXISTS idx_utang_supplier ON dat_utang(supplier_id, status_utang);
CREATE INDEX IF NOT EXISTS idx_uang_tanggal ON dat_pergerakan_uang(tanggal_waktu, jenis_arus);

-- ==========================================
-- 3. INITIAL MASTER DATA SEEDS
-- ==========================================

-- Satuan Dasar
INSERT OR IGNORE INTO mst_satuan_barang (created_at, created_by, kode_satuan, nama_satuan) VALUES
('2026-01-01T00:00:00Z', 1, 'PCS', 'Pieces / Buah'),
('2026-01-01T00:00:00Z', 1, 'KG', 'Kilogram'),
('2026-01-01T00:00:00Z', 1, 'LITER', 'Liter'),
('2026-01-01T00:00:00Z', 1, 'BOX', 'Box / Kotak'),
('2026-01-01T00:00:00Z', 1, 'PACK', 'Pack / Bungkus'),
('2026-01-01T00:00:00Z', 1, 'DUS', 'Dus / Karton');

-- Kategori Dasar
INSERT OR IGNORE INTO mst_kategori_barang (created_at, created_by, kode_kategori, nama_kategori, keterangan) VALUES
('2026-01-01T00:00:00Z', 1, 'SEMBAKO', 'Sembako', 'Bahan kebutuhan pokok sehari-hari'),
('2026-01-01T00:00:00Z', 1, 'MAKANAN', 'Makanan Ringan', 'Camilan dan makanan olahan'),
('2026-01-01T00:00:00Z', 1, 'MINUMAN', 'Minuman', 'Air mineral, teh, kopi, dan minuman kemasan'),
('2026-01-01T00:00:00Z', 1, 'RUMAH_TANGGA', 'Kebutuhan Rumah Tangga', 'Sabun, deterjen, dan perlengkapan kebersihan'),
('2026-01-01T00:00:00Z', 1, 'LAIN_LAIN', 'Lain-lain', 'Produk campuran lainnya');

-- Jenis Mutasi Stok (Konfigurasi Dampak Stok Sesuai PRD Seksi 7.3)
INSERT OR IGNORE INTO mst_jenis_mutasi (created_at, created_by, kode_mutasi, nama_mutasi, dampak_stok, keterangan) VALUES
('2026-01-01T00:00:00Z', 1, 'STOK_AWAL', 'Stok Awal', 'TAMBAH', 'Input stok permulaan saat sistem diinisialisasi'),
('2026-01-01T00:00:00Z', 1, 'PEMBELIAN', 'Pembelian Barang', 'TAMBAH', 'Barang masuk dari supplier atau perorangan'),
('2026-01-01T00:00:00Z', 1, 'PENJUALAN', 'Penjualan Barang', 'KURANG', 'Barang keluar akibat transaksi penjualan kasir'),
('2026-01-01T00:00:00Z', 1, 'PENYESUAIAN', 'Penyesuaian Stok (Opname)', 'TETAP', 'Koreksi stok fisik vs sistem'),
('2026-01-01T00:00:00Z', 1, 'BARANG_RUSAK', 'Barang Rusak / Kadaluarsa', 'KURANG', 'Pengurangan stok barang yang tidak layak jual'),
('2026-01-01T00:00:00Z', 1, 'RETUR_PEMBELIAN', 'Retur Pembelian', 'KURANG', 'Pengembalian barang ke supplier'),
('2026-01-01T00:00:00Z', 1, 'RETUR_PENJUALAN', 'Retur Penjualan', 'TAMBAH', 'Pengembalian barang dari pelanggan');
