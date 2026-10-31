use crate::database::connection::DbManager;
use crate::services::report_service::{
    CashFlowReportRow, DashboardSummary, LaporanPenjualanSummary, PenjualanReportRow,
    ProductProfitReportRow, ReportService,
};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn get_dashboard_summary(
    db_manager: State<'_, Arc<DbManager>>,
) -> Result<DashboardSummary, String> {
    let conn = db_manager.get_connection();
    ReportService::get_dashboard_summary(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_sales_report(
    start_date: String,
    end_date: String,
    db_manager: State<'_, Arc<DbManager>>,
) -> Result<(LaporanPenjualanSummary, Vec<PenjualanReportRow>), String> {
    let conn = db_manager.get_connection();
    ReportService::get_sales_report(&conn, &start_date, &end_date).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_cash_flow_report(
    start_date: String,
    end_date: String,
    db_manager: State<'_, Arc<DbManager>>,
) -> Result<Vec<CashFlowReportRow>, String> {
    let conn = db_manager.get_connection();
    ReportService::get_cash_flow_report(&conn, &start_date, &end_date).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_product_profit_report(
    start_date: String,
    end_date: String,
    db_manager: State<'_, Arc<DbManager>>,
) -> Result<Vec<ProductProfitReportRow>, String> {
    let conn = db_manager.get_connection();
    ReportService::get_product_profit_report(&conn, &start_date, &end_date)
        .map_err(|e| e.to_string())
}
