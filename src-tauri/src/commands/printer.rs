use crate::services::printer_service::{PrinterConfig, PrinterService, ReceiptPrintData};

#[tauri::command]
pub fn generate_receipt_preview(
    data: ReceiptPrintData,
    config: Option<PrinterConfig>,
) -> Result<String, String> {
    let cfg = config.unwrap_or_default();
    Ok(PrinterService::generate_text_preview(&data, &cfg))
}

#[tauri::command]
pub fn generate_esc_pos_bytes(
    data: ReceiptPrintData,
    config: Option<PrinterConfig>,
) -> Result<Vec<u8>, String> {
    let cfg = config.unwrap_or_default();
    Ok(PrinterService::generate_esc_pos_bytes(&data, &cfg))
}

#[tauri::command]
pub fn print_receipt_thermal(
    data: ReceiptPrintData,
    config: Option<PrinterConfig>,
) -> Result<(), String> {
    let cfg = config.unwrap_or_default();
    let bytes = PrinterService::generate_esc_pos_bytes(&data, &cfg);
    log::info!("Simulated thermal printer output: {} bytes generated", bytes.len());
    Ok(())
}
