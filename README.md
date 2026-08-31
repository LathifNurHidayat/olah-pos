# OlahPOS

Desktop Point of Sale (POS) & Inventory Management System.

OlahPOS dirancang dengan pendekatan **local-first** untuk menjamin keandalan dan kecepatan operasional kasir dan manajemen stok tanpa ketergantungan koneksi internet terus-menerus.

---

## 🛠️ Tech Stack

- **Desktop Framework**: Tauri 2
- **Frontend**: React, TypeScript, Vite, Tailwind CSS
- **Backend Core**: Rust dengan `rusqlite`
- **Database**:
  - `kasir_pos.db`: Database operasional transaksi
  - `kasir_pos_log.db`: Database audit log perubahan data dinamis
- **Keamanan Integrasi**: Kredensial integrasi pihak ketiga dienkripsi secara simetris (AES-256-GCM) dengan encryption key pada konfigurasi aplikasi
- **Eksternal**: Midtrans QRIS (via Payment Bridge), Google Drive Backup API, Thermal Printer (ESC/POS)

---

## 📋 Fitur Utama

1. **Kasir (POS)**: Transaksi cepat dengan pemindaian barcode, hotkey keyboard, dan cetak struk thermal.
2. **Master Data**: Pengelolaan barang, kategori, satuan, jenis mutasi, dan supplier/pihak terkait.
3. **Inventori & Mutasi**: Pencatatan mutasi stok ketat untuk setiap pergerakan barang.
4. **Pembelian & Utang**: Pencatatan pembelian tunai dan kredit dengan pembayaran utang bertahap.
5. **Arus Kas & Laporan**: Laporan penjualan, mutasi stok, arus uang masuk/keluar, dan dashboard analitik bisnis.
6. **Backup Otomatis**: Snapshot berkala ke Google Drive dengan scheduler latar belakang sistem operasi.
7. **Audit Log Menyeluruh**: Pencatatan riwayat perubahan data (INSERT, UPDATE, DELETE, RESTORE) secara atomik.

---

## 🚀 Alur Pengembangan & Aturan Git Commit

Setiap task dikembangkan secara modular dan menghasilkan tepat satu commit tersendiri dengan format:

```text
<type> (<scope>) : <description>
```

Contoh:
- `init (chore) : initial core project`
- `init (chore) : initial react and vite setup`
- `init (chore) : initial tauri 2 and rust framework`
- `feat (database) : implement dual sqlite connection with attach database and crypto layer`
- `feat (inventory) : implement stock movement`
