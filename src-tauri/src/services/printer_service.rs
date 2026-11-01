use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinterConfig {
    pub paper_width: String, // "58mm" | "80mm"
    pub auto_cut: bool,
    pub open_cash_drawer: bool,
    pub printer_name: Option<String>,
}

impl Default for PrinterConfig {
    fn default() -> Self {
        Self {
            paper_width: "58mm".to_string(),
            auto_cut: true,
            open_cash_drawer: false,
            printer_name: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptPrintItem {
    pub nama_barang: String,
    pub kuantitas: i64,
    pub harga_satuan: i64,
    pub subtotal: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptPrintData {
    pub store_name: String,
    pub store_address: String,
    pub store_phone: String,
    pub receipt_footer: String,
    pub nomor_struk: String,
    pub tanggal_penjualan: String,
    pub kasir_nama: String,
    pub items: Vec<ReceiptPrintItem>,
    pub total_kotor: i64,
    pub total_diskon: i64,
    pub total_bersih: i64,
    pub metode_pembayaran: String,
    pub nominal_diterima: i64,
    pub nominal_kembalian: i64,
}

pub struct PrinterService;

impl PrinterService {
    /// Format angka ke format mata uang Rupiah
    fn format_rupiah(val: i64) -> String {
        let s = val.abs().to_string();
        let mut result = String::new();
        let chars: Vec<char> = s.chars().rev().collect();
        for (i, c) in chars.iter().enumerate() {
            if i > 0 && i % 3 == 0 {
                result.push('.');
            }
            result.push(*c);
        }
        let formatted: String = result.chars().rev().collect();
        if val < 0 {
            format!("-Rp{}", formatted)
        } else {
            format!("Rp{}", formatted)
        }
    }

    /// Menghasilkan baris dua kolom (kiri rata kiri, kanan rata kanan)
    fn format_two_columns(left: &str, right: &str, width: usize) -> String {
        let left_len = left.chars().count();
        let right_len = right.chars().count();
        if left_len + right_len >= width {
            let max_left = if width > right_len + 1 { width - right_len - 1 } else { 0 };
            let truncated_left: String = left.chars().take(max_left).collect();
            let spaces = width.saturating_sub(truncated_left.chars().count() + right_len);
            format!("{}{}{}", truncated_left, " ".repeat(spaces), right)
        } else {
            let spaces = width - (left_len + right_len);
            format!("{}{}{}", left, " ".repeat(spaces), right)
        }
    }

    /// Menghasilkan representasi teks struk untuk preview visual di layar
    pub fn generate_text_preview(data: &ReceiptPrintData, config: &PrinterConfig) -> String {
        let width = if config.paper_width == "80mm" { 42 } else { 32 };
        let sep = "-".repeat(width);
        let mut out = Vec::new();

        // Header
        out.push(format!("{:^width$}", data.store_name, width = width));
        out.push(format!("{:^width$}", data.store_address, width = width));
        out.push(format!("{:^width$}", data.store_phone, width = width));
        out.push(sep.clone());

        // Transaksi info
        out.push(Self::format_two_columns("No Struk:", &data.nomor_struk, width));
        out.push(Self::format_two_columns("Waktu:", &data.tanggal_penjualan, width));
        out.push(Self::format_two_columns("Kasir:", &data.kasir_nama, width));
        out.push(sep.clone());

        // Items
        for it in &data.items {
            out.push(it.nama_barang.clone());
            let detail_left = format!(" {} x {}", it.kuantitas, Self::format_rupiah(it.harga_satuan));
            let detail_right = Self::format_rupiah(it.subtotal);
            out.push(Self::format_two_columns(&detail_left, &detail_right, width));
        }
        out.push(sep.clone());

        // Summary
        out.push(Self::format_two_columns("Subtotal:", &Self::format_rupiah(data.total_kotor), width));
        if data.total_diskon > 0 {
            out.push(Self::format_two_columns("Diskon:", &format!("-{}", Self::format_rupiah(data.total_diskon)), width));
        }
        out.push(Self::format_two_columns("Total:", &Self::format_rupiah(data.total_bersih), width));
        out.push(Self::format_two_columns("Metode:", &data.metode_pembayaran, width));
        out.push(Self::format_two_columns("Bayar:", &Self::format_rupiah(data.nominal_diterima), width));
        out.push(Self::format_two_columns("Kembali:", &Self::format_rupiah(data.nominal_kembalian), width));
        out.push(sep.clone());

        // Footer
        for line in data.receipt_footer.lines() {
            out.push(format!("{:^width$}", line, width = width));
        }

        out.join("\n")
    }

    /// Menghasilkan stream byte biner standar ESC/POS untuk dikirim langsung ke printer thermal
    pub fn generate_esc_pos_bytes(data: &ReceiptPrintData, config: &PrinterConfig) -> Vec<u8> {
        let width = if config.paper_width == "80mm" { 42 } else { 32 };
        let mut bytes = Vec::new();

        // 1. Initialize Printer: ESC @
        bytes.extend_from_slice(b"\x1B\x40");

        // 2. Open Cash Drawer if enabled: ESC p m t1 t2
        if config.open_cash_drawer {
            bytes.extend_from_slice(b"\x1B\x70\x00\x19\xFA");
        }

        // 3. Store Name (Centered, Bold, Double Height): ESC a 1, ESC E 1, GS ! 0x11
        bytes.extend_from_slice(b"\x1B\x61\x01"); // Center align
        bytes.extend_from_slice(b"\x1B\x45\x01"); // Bold ON
        bytes.extend_from_slice(b"\x1D\x21\x11"); // Double size
        bytes.extend_from_slice(data.store_name.as_bytes());
        bytes.extend_from_slice(b"\n");

        // Normal text size for address & phone
        bytes.extend_from_slice(b"\x1D\x21\x00"); // Normal size
        bytes.extend_from_slice(b"\x1B\x45\x00"); // Bold OFF
        bytes.extend_from_slice(data.store_address.as_bytes());
        bytes.extend_from_slice(b"\n");
        bytes.extend_from_slice(data.store_phone.as_bytes());
        bytes.extend_from_slice(b"\n");

        // Separator line
        let sep = "-".repeat(width);
        bytes.extend_from_slice(sep.as_bytes());
        bytes.extend_from_slice(b"\n");

        // Align Left for transaction details: ESC a 0
        bytes.extend_from_slice(b"\x1B\x61\x00");
        bytes.extend_from_slice(Self::format_two_columns("No Struk:", &data.nomor_struk, width).as_bytes());
        bytes.extend_from_slice(b"\n");
        bytes.extend_from_slice(Self::format_two_columns("Waktu:", &data.tanggal_penjualan, width).as_bytes());
        bytes.extend_from_slice(b"\n");
        bytes.extend_from_slice(Self::format_two_columns("Kasir:", &data.kasir_nama, width).as_bytes());
        bytes.extend_from_slice(b"\n");
        bytes.extend_from_slice(sep.as_bytes());
        bytes.extend_from_slice(b"\n");

        // Items
        for it in &data.items {
            bytes.extend_from_slice(it.nama_barang.as_bytes());
            bytes.extend_from_slice(b"\n");
            let detail_left = format!(" {} x {}", it.kuantitas, Self::format_rupiah(it.harga_satuan));
            let detail_right = Self::format_rupiah(it.subtotal);
            bytes.extend_from_slice(Self::format_two_columns(&detail_left, &detail_right, width).as_bytes());
            bytes.extend_from_slice(b"\n");
        }
        bytes.extend_from_slice(sep.as_bytes());
        bytes.extend_from_slice(b"\n");

        // Summary
        bytes.extend_from_slice(Self::format_two_columns("Subtotal:", &Self::format_rupiah(data.total_kotor), width).as_bytes());
        bytes.extend_from_slice(b"\n");
        if data.total_diskon > 0 {
            bytes.extend_from_slice(Self::format_two_columns("Diskon:", &format!("-{}", Self::format_rupiah(data.total_diskon)), width).as_bytes());
            bytes.extend_from_slice(b"\n");
        }

        // Total Tagihan (Bold)
        bytes.extend_from_slice(b"\x1B\x45\x01"); // Bold ON
        bytes.extend_from_slice(Self::format_two_columns("TOTAL:", &Self::format_rupiah(data.total_bersih), width).as_bytes());
        bytes.extend_from_slice(b"\n");
        bytes.extend_from_slice(b"\x1B\x45\x00"); // Bold OFF

        bytes.extend_from_slice(Self::format_two_columns("Metode:", &data.metode_pembayaran, width).as_bytes());
        bytes.extend_from_slice(b"\n");
        bytes.extend_from_slice(Self::format_two_columns("Bayar:", &Self::format_rupiah(data.nominal_diterima), width).as_bytes());
        bytes.extend_from_slice(b"\n");
        bytes.extend_from_slice(Self::format_two_columns("Kembali:", &Self::format_rupiah(data.nominal_kembalian), width).as_bytes());
        bytes.extend_from_slice(b"\n");
        bytes.extend_from_slice(sep.as_bytes());
        bytes.extend_from_slice(b"\n");

        // Footer (Centered): ESC a 1
        bytes.extend_from_slice(b"\x1B\x61\x01");
        for line in data.receipt_footer.lines() {
            bytes.extend_from_slice(line.as_bytes());
            bytes.extend_from_slice(b"\n");
        }

        // Feed 4 lines before cut
        bytes.extend_from_slice(b"\n\n\n\n");

        // Paper Cut: GS V 66 0 (partial cut)
        if config.auto_cut {
            bytes.extend_from_slice(b"\x1D\x56\x42\x00");
        }

        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_receipt_data() -> ReceiptPrintData {
        ReceiptPrintData {
            store_name: "OlahPOS Minimarket".to_string(),
            store_address: "Jl. Asia Afrika No. 10".to_string(),
            store_phone: "08123456789".to_string(),
            receipt_footer: "Terima kasih telah berbelanja!\nBarang tidak dapat ditukar.".to_string(),
            nomor_struk: "TRX-20261101-001".to_string(),
            tanggal_penjualan: "2026-11-01 10:30".to_string(),
            kasir_nama: "Kasir 1".to_string(),
            items: vec![
                ReceiptPrintItem {
                    nama_barang: "Beras Premium 5kg".to_string(),
                    kuantitas: 1,
                    harga_satuan: 72000,
                    subtotal: 72000,
                },
                ReceiptPrintItem {
                    nama_barang: "Minyak Goreng 2L".to_string(),
                    kuantitas: 2,
                    harga_satuan: 34000,
                    subtotal: 68000,
                },
            ],
            total_kotor: 140000,
            total_diskon: 0,
            total_bersih: 140000,
            metode_pembayaran: "TUNAI".to_string(),
            nominal_diterima: 150000,
            nominal_kembalian: 10000,
        }
    }

    #[test]
    fn test_text_preview_generation_58mm() {
        let data = sample_receipt_data();
        let config = PrinterConfig {
            paper_width: "58mm".to_string(),
            auto_cut: true,
            open_cash_drawer: false,
            printer_name: None,
        };

        let preview = PrinterService::generate_text_preview(&data, &config);
        assert!(preview.contains("OlahPOS Minimarket"));
        assert!(preview.contains("TRX-20261101-001"));
        assert!(preview.contains("Beras Premium 5kg"));
        assert!(preview.contains("Rp140.000"));
        assert!(preview.contains("Rp10.000"));
    }

    #[test]
    fn test_esc_pos_bytes_generation() {
        let data = sample_receipt_data();
        let config = PrinterConfig {
            paper_width: "58mm".to_string(),
            auto_cut: true,
            open_cash_drawer: true,
            printer_name: None,
        };

        let bytes = PrinterService::generate_esc_pos_bytes(&data, &config);
        assert!(!bytes.is_empty());
        // Check ESC @ init prefix
        assert_eq!(&bytes[0..2], b"\x1B\x40");
        // Check contains store name bytes
        assert!(bytes.windows(b"OlahPOS Minimarket".len()).any(|w| w == b"OlahPOS Minimarket"));
        // Check ends with paper cut sequence: GS V B 0
        let cut_seq = b"\x1D\x56\x42\x00";
        assert_eq!(&bytes[bytes.len() - cut_seq.len()..], cut_seq);
    }
}
