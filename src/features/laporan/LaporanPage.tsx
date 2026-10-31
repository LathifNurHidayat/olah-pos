import React, { useState, useEffect } from "react";
import {
  FileBarChart,
  Calendar,
  RefreshCw,
  Coins,
  Receipt,
  ArrowUpRight,
  ArrowDownLeft,
  Percent,
} from "lucide-react";
import { formatRupiah } from "@/lib/utils";

export interface LaporanPenjualanSummary {
  total_omset: number;
  total_diskon: number;
  total_transaksi: number;
  total_laba_kotor: number;
  transaksi_tunai: number;
  transaksi_qris: number;
}

export interface PenjualanReportRow {
  penjualan_id: number;
  nomor_struk: string;
  tanggal_penjualan: string;
  total_kotor: number;
  total_diskon: number;
  total_bersih: number;
  metode_pembayaran: string;
  total_hpp: number;
  laba_kotor: number;
}

export interface CashFlowReportRow {
  pergerakan_uang_id: number;
  tanggal_waktu: string;
  jenis_arus: "MASUK" | "KELUAR" | string;
  kategori_transaksi: string;
  nominal: number;
  metode_pembayaran: string;
  referensi_tabel: string | null;
  keterangan: string | null;
}

export interface ProductProfitReportRow {
  barang_id: number;
  kode_barang: string;
  nama_barang: string;
  nama_kategori: string;
  kuantitas_terjual: number;
  total_omset: number;
  total_hpp: number;
  laba_kotor: number;
  margin_persen: number;
}

export const LaporanPage: React.FC = () => {
  const [activeTab, setActiveTab] = useState<"sales" | "cashflow" | "products">("sales");

  // Date Filters (default: 30 days back to today)
  const todayStr = new Date().toISOString().split("T")[0];
  const lastMonthStr = new Date(Date.now() - 30 * 24 * 60 * 60 * 1000)
    .toISOString()
    .split("T")[0];

  const [startDate, setStartDate] = useState(lastMonthStr);
  const [endDate, setEndDate] = useState(todayStr);
  const [loading, setLoading] = useState(false);

  // Data States
  const [salesSummary, setSalesSummary] = useState<LaporanPenjualanSummary>({
    total_omset: 3450000,
    total_diskon: 50000,
    total_transaksi: 24,
    total_laba_kotor: 850000,
    transaksi_tunai: 18,
    transaksi_qris: 6,
  });
  const [salesRows, setSalesRows] = useState<PenjualanReportRow[]>([]);
  const [cashFlowRows, setCashFlowRows] = useState<CashFlowReportRow[]>([]);
  const [productProfitRows, setProductProfitRows] = useState<ProductProfitReportRow[]>([]);

  const loadReportData = async () => {
    setLoading(true);
    if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
      try {
        const { invoke } = await import("@tauri-apps/api/core");

        if (activeTab === "sales") {
          const [summary, list] = await invoke<[LaporanPenjualanSummary, PenjualanReportRow[]]>(
            "get_sales_report",
            { startDate, endDate }
          );
          setSalesSummary(summary);
          setSalesRows(list);
        } else if (activeTab === "cashflow") {
          const list = await invoke<CashFlowReportRow[]>("get_cash_flow_report", {
            startDate,
            endDate,
          });
          setCashFlowRows(list);
        } else if (activeTab === "products") {
          const list = await invoke<ProductProfitReportRow[]>("get_product_profit_report", {
            startDate,
            endDate,
          });
          setProductProfitRows(list);
        }
      } catch (err) {
        console.error("Gagal load data laporan:", err);
      }
    }
    setLoading(false);
  };

  useEffect(() => {
    loadReportData();
  }, [activeTab, startDate, endDate]);

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h2 className="text-2xl font-bold tracking-tight text-slate-900 dark:text-white flex items-center gap-2">
            <FileBarChart className="h-6 w-6 text-indigo-600" />
            Laporan Keuangan & Analisis
          </h2>
          <p className="text-xs text-slate-500 mt-1">
            Laporan penjualan, arus kas (cash flow), dan estimasi margin laba kotor produk secara
            akurat dari SQLite lokal.
          </p>
        </div>

        {/* Date Filter Bar */}
        <div className="flex items-center gap-2 bg-white p-2 rounded-2xl border border-slate-200/90 shadow-sm dark:bg-slate-900 dark:border-slate-800 text-xs">
          <Calendar className="h-4 w-4 text-slate-400 shrink-0 ml-1" />
          <input
            type="date"
            value={startDate}
            onChange={(e) => setStartDate(e.target.value)}
            className="rounded-lg bg-slate-50 px-2 py-1 outline-none dark:bg-slate-800 dark:text-white"
          />
          <span className="text-slate-400">s/d</span>
          <input
            type="date"
            value={endDate}
            onChange={(e) => setEndDate(e.target.value)}
            className="rounded-lg bg-slate-50 px-2 py-1 outline-none dark:bg-slate-800 dark:text-white"
          />
          <button
            onClick={loadReportData}
            disabled={loading}
            className="rounded-xl bg-indigo-50 p-1.5 text-indigo-600 hover:bg-indigo-100 dark:bg-indigo-950/60 dark:text-indigo-400"
            title="Muat Ulang"
          >
            <RefreshCw className={`h-3.5 w-3.5 ${loading ? "animate-spin" : ""}`} />
          </button>
        </div>
      </div>

      {/* Tabs */}
      <div className="flex border-b border-slate-200 dark:border-slate-800">
        <button
          onClick={() => setActiveTab("sales")}
          className={`flex items-center gap-2 border-b-2 px-4 py-2.5 text-xs font-semibold transition-colors ${
            activeTab === "sales"
              ? "border-indigo-600 text-indigo-600 dark:border-indigo-400 dark:text-indigo-400"
              : "border-transparent text-slate-500 hover:text-slate-800 dark:hover:text-slate-200"
          }`}
        >
          <Receipt className="h-4 w-4" />
          Laporan Penjualan & Omset
        </button>

        <button
          onClick={() => setActiveTab("cashflow")}
          className={`flex items-center gap-2 border-b-2 px-4 py-2.5 text-xs font-semibold transition-colors ${
            activeTab === "cashflow"
              ? "border-indigo-600 text-indigo-600 dark:border-indigo-400 dark:text-indigo-400"
              : "border-transparent text-slate-500 hover:text-slate-800 dark:hover:text-slate-200"
          }`}
        >
          <Coins className="h-4 w-4" />
          Laporan Arus Kas (Cash Flow)
        </button>

        <button
          onClick={() => setActiveTab("products")}
          className={`flex items-center gap-2 border-b-2 px-4 py-2.5 text-xs font-semibold transition-colors ${
            activeTab === "products"
              ? "border-indigo-600 text-indigo-600 dark:border-indigo-400 dark:text-indigo-400"
              : "border-transparent text-slate-500 hover:text-slate-800 dark:hover:text-slate-200"
          }`}
        >
          <Percent className="h-4 w-4" />
          Analisis Laba Kotor Produk
        </button>
      </div>

      {/* Tab 1: Laporan Penjualan */}
      {activeTab === "sales" && (
        <div className="space-y-4">
          {/* Summary KPI Cards */}
          <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
            <div className="rounded-2xl border border-slate-200/80 bg-white p-4 shadow-sm dark:border-slate-800 dark:bg-slate-900">
              <span className="text-[11px] font-semibold uppercase tracking-wider text-slate-400">
                Total Omset Penjualan
              </span>
              <div className="mt-2 text-xl font-bold text-slate-900 dark:text-white truncate">
                {formatRupiah(salesSummary.total_omset)}
              </div>
              <div className="text-[11px] text-emerald-600 font-medium mt-0.5">
                {salesSummary.total_transaksi} Transaksi Berhasil
              </div>
            </div>

            <div className="rounded-2xl border border-slate-200/80 bg-white p-4 shadow-sm dark:border-slate-800 dark:bg-slate-900">
              <span className="text-[11px] font-semibold uppercase tracking-wider text-slate-400">
                Total Laba Kotor (Gross Margin)
              </span>
              <div className="mt-2 text-xl font-bold text-emerald-600 dark:text-emerald-400 truncate">
                {formatRupiah(salesSummary.total_laba_kotor)}
              </div>
              <div className="text-[11px] text-slate-400 mt-0.5">Setelah dikurangi modal HPP</div>
            </div>

            <div className="rounded-2xl border border-slate-200/80 bg-white p-4 shadow-sm dark:border-slate-800 dark:bg-slate-900">
              <span className="text-[11px] font-semibold uppercase tracking-wider text-slate-400">
                Total Diskon Diberikan
              </span>
              <div className="mt-2 text-xl font-bold text-amber-600 dark:text-amber-400 truncate">
                {formatRupiah(salesSummary.total_diskon)}
              </div>
              <div className="text-[11px] text-slate-400 mt-0.5">Diskon item & kupon</div>
            </div>

            <div className="rounded-2xl border border-slate-200/80 bg-white p-4 shadow-sm dark:border-slate-800 dark:bg-slate-900">
              <span className="text-[11px] font-semibold uppercase tracking-wider text-slate-400">
                Metode Pembayaran
              </span>
              <div className="mt-2 flex items-center gap-3 text-xs font-bold text-slate-800 dark:text-slate-200">
                <span className="text-emerald-600">Tunai: {salesSummary.transaksi_tunai}</span>
                <span>•</span>
                <span className="text-indigo-600">QRIS: {salesSummary.transaksi_qris}</span>
              </div>
              <div className="text-[11px] text-slate-400 mt-1">Pembagian kanal bayar</div>
            </div>
          </div>

          {/* Table Sales Report */}
          <div className="overflow-hidden rounded-2xl border border-slate-200/80 bg-white shadow-sm dark:border-slate-800 dark:bg-slate-900">
            <div className="overflow-x-auto">
              <table className="w-full text-left text-xs">
                <thead className="bg-slate-50/80 text-[11px] font-semibold uppercase tracking-wider text-slate-400 dark:bg-slate-800/50">
                  <tr>
                    <th className="px-4 py-3">No Struk</th>
                    <th className="px-4 py-3">Tanggal Penjualan</th>
                    <th className="px-4 py-3 text-center">Metode</th>
                    <th className="px-4 py-3 text-right">Total Kotor</th>
                    <th className="px-4 py-3 text-right">Diskon</th>
                    <th className="px-4 py-3 text-right">Total Bersih</th>
                    <th className="px-4 py-3 text-right">Total HPP</th>
                    <th className="px-4 py-3 text-right">Laba Kotor</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
                  {salesRows.length === 0 ? (
                    <tr>
                      <td colSpan={8} className="py-8 text-center text-slate-400">
                        Belum ada riwayat transaksi pada rentang tanggal ini.
                      </td>
                    </tr>
                  ) : (
                    salesRows.map((r) => (
                      <tr
                        key={r.penjualan_id}
                        className="hover:bg-slate-50/50 dark:hover:bg-slate-800/30 transition-colors"
                      >
                        <td className="px-4 py-3 font-mono font-semibold text-slate-900 dark:text-white">
                          {r.nomor_struk}
                        </td>
                        <td className="px-4 py-3 text-slate-500 font-mono">
                          {r.tanggal_penjualan}
                        </td>
                        <td className="px-4 py-3 text-center">
                          <span className="rounded-full bg-slate-100 px-2.5 py-0.5 text-[10px] font-bold dark:bg-slate-800">
                            {r.metode_pembayaran}
                          </span>
                        </td>
                        <td className="px-4 py-3 text-right font-mono text-slate-600 dark:text-slate-300">
                          {formatRupiah(r.total_kotor)}
                        </td>
                        <td className="px-4 py-3 text-right font-mono text-amber-600">
                          {formatRupiah(r.total_diskon)}
                        </td>
                        <td className="px-4 py-3 text-right font-mono font-bold text-slate-900 dark:text-white">
                          {formatRupiah(r.total_bersih)}
                        </td>
                        <td className="px-4 py-3 text-right font-mono text-slate-500">
                          {formatRupiah(r.total_hpp)}
                        </td>
                        <td className="px-4 py-3 text-right font-mono font-bold text-emerald-600 dark:text-emerald-400">
                          {formatRupiah(r.laba_kotor)}
                        </td>
                      </tr>
                    ))
                  )}
                </tbody>
              </table>
            </div>
          </div>
        </div>
      )}

      {/* Tab 2: Laporan Arus Kas */}
      {activeTab === "cashflow" && (
        <div className="space-y-4">
          <div className="overflow-hidden rounded-2xl border border-slate-200/80 bg-white shadow-sm dark:border-slate-800 dark:bg-slate-900">
            <div className="overflow-x-auto">
              <table className="w-full text-left text-xs">
                <thead className="bg-slate-50/80 text-[11px] font-semibold uppercase tracking-wider text-slate-400 dark:bg-slate-800/50">
                  <tr>
                    <th className="px-4 py-3">Waktu</th>
                    <th className="px-4 py-3 text-center">Jenis Arus</th>
                    <th className="px-4 py-3">Kategori Transaksi</th>
                    <th className="px-4 py-3 text-center">Metode</th>
                    <th className="px-4 py-3 text-right">Nominal</th>
                    <th className="px-4 py-3">Keterangan</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
                  {cashFlowRows.length === 0 ? (
                    <tr>
                      <td colSpan={6} className="py-8 text-center text-slate-400">
                        Belum ada pencatatan arus kas pada rentang tanggal ini.
                      </td>
                    </tr>
                  ) : (
                    cashFlowRows.map((cf) => (
                      <tr
                        key={cf.pergerakan_uang_id}
                        className="hover:bg-slate-50/50 dark:hover:bg-slate-800/30 transition-colors"
                      >
                        <td className="px-4 py-3 font-mono text-[11px] text-slate-500 whitespace-nowrap">
                          {new Date(cf.tanggal_waktu).toLocaleString("id-ID")}
                        </td>
                        <td className="px-4 py-3 text-center">
                          <span
                            className={`inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-[10px] font-bold ${
                              cf.jenis_arus === "MASUK"
                                ? "bg-emerald-50 text-emerald-600 dark:bg-emerald-950/60 dark:text-emerald-400"
                                : "bg-rose-50 text-rose-600 dark:bg-rose-950/60 dark:text-rose-400"
                            }`}
                          >
                            {cf.jenis_arus === "MASUK" ? (
                              <ArrowUpRight className="h-3 w-3" />
                            ) : (
                              <ArrowDownLeft className="h-3 w-3" />
                            )}
                            {cf.jenis_arus}
                          </span>
                        </td>
                        <td className="px-4 py-3 font-medium text-slate-800 dark:text-slate-200">
                          {cf.kategori_transaksi}
                        </td>
                        <td className="px-4 py-3 text-center font-mono text-slate-500">
                          {cf.metode_pembayaran}
                        </td>
                        <td className="px-4 py-3 text-right font-mono font-bold">
                          <span
                            className={
                              cf.jenis_arus === "MASUK"
                                ? "text-emerald-600 dark:text-emerald-400"
                                : "text-rose-600 dark:text-rose-400"
                            }
                          >
                            {cf.jenis_arus === "MASUK" ? "+" : "-"}
                            {formatRupiah(cf.nominal)}
                          </span>
                        </td>
                        <td className="px-4 py-3 text-slate-500 text-[11px]">
                          {cf.keterangan || "-"}
                        </td>
                      </tr>
                    ))
                  )}
                </tbody>
              </table>
            </div>
          </div>
        </div>
      )}

      {/* Tab 3: Analisis Laba Kotor Produk */}
      {activeTab === "products" && (
        <div className="space-y-4">
          <div className="overflow-hidden rounded-2xl border border-slate-200/80 bg-white shadow-sm dark:border-slate-800 dark:bg-slate-900">
            <div className="overflow-x-auto">
              <table className="w-full text-left text-xs">
                <thead className="bg-slate-50/80 text-[11px] font-semibold uppercase tracking-wider text-slate-400 dark:bg-slate-800/50">
                  <tr>
                    <th className="px-4 py-3">Kode / Barang</th>
                    <th className="px-4 py-3">Kategori</th>
                    <th className="px-4 py-3 text-center">Qty Terjual</th>
                    <th className="px-4 py-3 text-right">Total Omset</th>
                    <th className="px-4 py-3 text-right">Total HPP</th>
                    <th className="px-4 py-3 text-right">Laba Kotor</th>
                    <th className="px-4 py-3 text-center">Margin (%)</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
                  {productProfitRows.length === 0 ? (
                    <tr>
                      <td colSpan={7} className="py-8 text-center text-slate-400">
                        Belum ada penjualan produk pada rentang tanggal ini.
                      </td>
                    </tr>
                  ) : (
                    productProfitRows.map((p) => (
                      <tr
                        key={p.barang_id}
                        className="hover:bg-slate-50/50 dark:hover:bg-slate-800/30 transition-colors"
                      >
                        <td className="px-4 py-3">
                          <div className="font-semibold text-slate-800 dark:text-slate-200">
                            {p.nama_barang}
                          </div>
                          <div className="text-[10px] font-mono text-slate-400">
                            {p.kode_barang}
                          </div>
                        </td>
                        <td className="px-4 py-3 text-slate-500">{p.nama_kategori}</td>
                        <td className="px-4 py-3 text-center font-mono font-bold">
                          {p.kuantitas_terjual}
                        </td>
                        <td className="px-4 py-3 text-right font-mono">
                          {formatRupiah(p.total_omset)}
                        </td>
                        <td className="px-4 py-3 text-right font-mono text-slate-500">
                          {formatRupiah(p.total_hpp)}
                        </td>
                        <td className="px-4 py-3 text-right font-mono font-bold text-emerald-600 dark:text-emerald-400">
                          {formatRupiah(p.laba_kotor)}
                        </td>
                        <td className="px-4 py-3 text-center font-mono font-bold text-indigo-600">
                          {p.margin_persen.toFixed(1)}%
                        </td>
                      </tr>
                    ))
                  )}
                </tbody>
              </table>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
