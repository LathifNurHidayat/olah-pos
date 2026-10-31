import React, { useState, useEffect } from "react";
import {
  CreditCard,
  Search,
  Filter,
  RefreshCw,
  Coins,
  CheckCircle2,
  Clock,
  AlertCircle,
  X,
  History,
  Receipt,
  ArrowDownLeft,
} from "lucide-react";
import { formatRupiah } from "@/lib/utils";

export interface UtangItem {
  utang_id: number;
  nomor_referensi: string;
  pembelian_id: number;
  nomor_faktur: string;
  supplier_id: number;
  nama_supplier: string;
  total_utang_awal: number;
  total_dibayar: number;
  sisa_utang: number;
  status_utang: "BELUM_DIBAYAR" | "DIBAYAR_SEBAGIAN" | "LUNAS" | string;
  tanggal_jatuh_tempo: string | null;
  tanggal_pembelian: string;
}

export interface PembayaranUtangItem {
  pembayaran_utang_id: number;
  nomor_bukti_bayar: string;
  utang_id: number;
  tanggal_bayar: string;
  nominal_bayar: number;
  metode_bayar: string;
  keterangan: string | null;
  created_at: string;
}

const DEMO_UTANG: UtangItem[] = [
  {
    utang_id: 1,
    nomor_referensi: "UTG-2",
    pembelian_id: 2,
    nomor_faktur: "FAK-20261028-004",
    supplier_id: 2,
    nama_supplier: "CV Distribusi Utama Nusantara",
    total_utang_awal: 2500000,
    total_dibayar: 500000,
    sisa_utang: 2000000,
    status_utang: "DIBAYAR_SEBAGIAN",
    tanggal_jatuh_tempo: "2026-11-28",
    tanggal_pembelian: "2026-10-28",
  },
];

export const UtangPage: React.FC = () => {
  const [utangList, setUtangList] = useState<UtangItem[]>(DEMO_UTANG);
  const [searchQuery, setSearchQuery] = useState("");
  const [statusFilter, setStatusFilter] = useState("ALL");
  const [loading, setLoading] = useState(false);

  // Modal Bayar
  const [modalBayarOpen, setModalBayarOpen] = useState(false);
  const [selectedUtang, setSelectedUtang] = useState<UtangItem | null>(null);
  const [nominalBayar, setNominalBayar] = useState<number>(0);
  const [metodeBayar, setMetodeBayar] = useState<"TUNAI" | "TRANSFER">("TUNAI");
  const [tanggalBayar, setTanggalBayar] = useState(new Date().toISOString().split("T")[0]);
  const [keteranganBayar, setKeteranganBayar] = useState("");
  const [submitting, setSubmitting] = useState(false);

  // Modal Riwayat
  const [modalHistoryOpen, setModalHistoryOpen] = useState(false);
  const [historyList, setHistoryList] = useState<PembayaranUtangItem[]>([]);

  const loadData = async () => {
    setLoading(true);
    if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        const data = await invoke<UtangItem[]>("get_utang_list", {
          statusFilter: statusFilter === "ALL" ? null : statusFilter,
        });
        if (data.length > 0) setUtangList(data);
      } catch (err) {
        console.error("Gagal memuat kartu utang:", err);
      }
    }
    setLoading(false);
  };

  useEffect(() => {
    loadData();
  }, [statusFilter]);

  const openBayarModal = (u: UtangItem) => {
    setSelectedUtang(u);
    setNominalBayar(u.sisa_utang);
    setMetodeBayar("TUNAI");
    setTanggalBayar(new Date().toISOString().split("T")[0]);
    setKeteranganBayar("");
    setModalBayarOpen(true);
  };

  const handleProcessBayar = async () => {
    if (!selectedUtang) return;
    if (nominalBayar <= 0) {
      alert("Nominal pembayaran harus lebih besar dari 0!");
      return;
    }
    if (nominalBayar > selectedUtang.sisa_utang) {
      alert("Nominal pembayaran melebihi sisa utang!");
      return;
    }

    setSubmitting(true);
    const payload = {
      utang_id: selectedUtang.utang_id,
      tanggal_bayar: tanggalBayar,
      nominal_bayar: Number(nominalBayar),
      metode_bayar: metodeBayar,
      keterangan: keteranganBayar || null,
      user_id: 1,
    };

    if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        await invoke("bayar_utang", { input: payload });
        await loadData();
        setModalBayarOpen(false);
      } catch (err) {
        alert("Gagal memproses pembayaran: " + err);
      }
    } else {
      // Local fallback
      setUtangList((prev) =>
        prev.map((item) => {
          if (item.utang_id === selectedUtang.utang_id) {
            const sisaBaru = item.sisa_utang - payload.nominal_bayar;
            return {
              ...item,
              total_dibayar: item.total_dibayar + payload.nominal_bayar,
              sisa_utang: sisaBaru,
              status_utang: sisaBaru === 0 ? "LUNAS" : "DIBAYAR_SEBAGIAN",
            };
          }
          return item;
        })
      );
      setModalBayarOpen(false);
    }
    setSubmitting(false);
  };

  const openHistoryModal = async (u: UtangItem) => {
    setSelectedUtang(u);
    if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        const hist = await invoke<PembayaranUtangItem[]>("get_pembayaran_utang_history", {
          utangId: u.utang_id,
        });
        setHistoryList(hist);
      } catch (err) {
        console.error("Gagal load riwayat bayar:", err);
      }
    } else {
      setHistoryList([
        {
          pembayaran_utang_id: 1,
          nomor_bukti_bayar: "BYR-173000000",
          utang_id: u.utang_id,
          tanggal_bayar: "2026-10-28",
          nominal_bayar: 500000,
          metode_bayar: "TUNAI",
          keterangan: "Uang muka pembelian faktur",
          created_at: new Date().toISOString(),
        },
      ]);
    }
    setModalHistoryOpen(true);
  };

  // Calculations for KPI Cards
  const totalUtangAwal = utangList.reduce((acc, u) => acc + u.total_utang_awal, 0);
  const totalDibayar = utangList.reduce((acc, u) => acc + u.total_dibayar, 0);
  const totalSisaUtang = utangList.reduce((acc, u) => acc + u.sisa_utang, 0);
  const utangAktifCount = utangList.filter((u) => u.status_utang !== "LUNAS").length;

  const filtered = utangList.filter((u) => {
    const matchQuery =
      u.nomor_referensi.toLowerCase().includes(searchQuery.toLowerCase()) ||
      u.nomor_faktur.toLowerCase().includes(searchQuery.toLowerCase()) ||
      u.nama_supplier.toLowerCase().includes(searchQuery.toLowerCase());
    return matchQuery;
  });

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h2 className="text-2xl font-bold tracking-tight text-slate-900 dark:text-white flex items-center gap-2">
            <CreditCard className="h-6 w-6 text-indigo-600" />
            Kartu Utang Pembelian
          </h2>
          <p className="text-xs text-slate-500 mt-1">
            Pantau saldo utang kepada pemasok / supplier. Pembayaran cicilan mencatat pengeluaran kas
            tanpa mempengaruhi stok barang.
          </p>
        </div>

        <button
          onClick={loadData}
          disabled={loading}
          className="flex items-center gap-1.5 rounded-xl border border-slate-200 bg-white px-3 py-2 text-xs font-semibold text-slate-700 shadow-sm hover:bg-slate-50 dark:border-slate-800 dark:bg-slate-900 dark:text-slate-300"
        >
          <RefreshCw className={`h-3.5 w-3.5 ${loading ? "animate-spin" : ""}`} />
          Segarkan
        </button>
      </div>

      {/* KPI Cards */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
        <div className="rounded-2xl border border-slate-200/80 bg-white p-4 shadow-sm dark:border-slate-800 dark:bg-slate-900">
          <div className="flex items-center justify-between text-slate-400">
            <span className="text-[11px] font-semibold uppercase tracking-wider">
              Total Utang Awal
            </span>
            <Coins className="h-4 w-4 text-slate-500" />
          </div>
          <div className="mt-2 text-xl font-bold text-slate-900 dark:text-white truncate">
            {formatRupiah(totalUtangAwal)}
          </div>
          <div className="text-[11px] text-slate-400 mt-0.5">Seluruh komitmen tempo</div>
        </div>

        <div className="rounded-2xl border border-slate-200/80 bg-white p-4 shadow-sm dark:border-slate-800 dark:bg-slate-900">
          <div className="flex items-center justify-between text-slate-400">
            <span className="text-[11px] font-semibold uppercase tracking-wider">
              Sudah Terbayar
            </span>
            <CheckCircle2 className="h-4 w-4 text-emerald-500" />
          </div>
          <div className="mt-2 text-xl font-bold text-emerald-600 dark:text-emerald-400 truncate">
            {formatRupiah(totalDibayar)}
          </div>
          <div className="text-[11px] text-slate-400 mt-0.5">Arus kas keluar tercatat</div>
        </div>

        <div className="rounded-2xl border border-slate-200/80 bg-white p-4 shadow-sm dark:border-slate-800 dark:bg-slate-900">
          <div className="flex items-center justify-between text-slate-400">
            <span className="text-[11px] font-semibold uppercase tracking-wider">
              Sisa Utang Aktif
            </span>
            <ArrowDownLeft className="h-4 w-4 text-rose-500" />
          </div>
          <div className="mt-2 text-xl font-bold text-rose-600 dark:text-rose-400 truncate">
            {formatRupiah(totalSisaUtang)}
          </div>
          <div className="text-[11px] text-rose-600/80 dark:text-rose-400/80 mt-0.5">
            Wajib dilunasi
          </div>
        </div>

        <div className="rounded-2xl border border-slate-200/80 bg-white p-4 shadow-sm dark:border-slate-800 dark:bg-slate-900">
          <div className="flex items-center justify-between text-slate-400">
            <span className="text-[11px] font-semibold uppercase tracking-wider">
              Faktur Belum Lunas
            </span>
            <Clock className="h-4 w-4 text-amber-500" />
          </div>
          <div className="mt-2 text-xl font-bold text-amber-600 dark:text-amber-400">
            {utangAktifCount} Faktur
          </div>
          <div className="text-[11px] text-slate-400 mt-0.5">Membutuhkan tindak lanjut</div>
        </div>
      </div>

      {/* Filter Bar */}
      <div className="flex flex-col sm:flex-row items-center gap-3 bg-white p-3 rounded-2xl border border-slate-200 shadow-sm dark:bg-slate-900 dark:border-slate-800">
        <div className="relative flex-1 w-full">
          <Search className="absolute left-3 top-2.5 h-4 w-4 text-slate-400" />
          <input
            type="text"
            placeholder="Cari referensi utang, faktur, atau nama supplier..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full rounded-xl bg-slate-50 pl-9 pr-4 py-2 text-xs outline-none focus:ring-2 focus:ring-indigo-500/20 border border-slate-200/80 dark:bg-slate-800/60 dark:border-slate-700 dark:text-white"
          />
        </div>

        <div className="flex items-center gap-2 w-full sm:w-auto">
          <Filter className="h-4 w-4 text-slate-400 shrink-0" />
          <select
            value={statusFilter}
            onChange={(e) => setStatusFilter(e.target.value)}
            className="rounded-xl border border-slate-200/80 bg-slate-50 px-3 py-2 text-xs outline-none focus:ring-2 focus:ring-indigo-500/20 dark:bg-slate-800/60 dark:border-slate-700 dark:text-white w-full sm:w-auto"
          >
            <option value="ALL">Semua Status Utang</option>
            <option value="BELUM_DIBAYAR">Belum Dibayar</option>
            <option value="DIBAYAR_SEBAGIAN">Dibayar Sebagian</option>
            <option value="LUNAS">Lunas</option>
          </select>
        </div>
      </div>

      {/* Table */}
      <div className="overflow-hidden rounded-2xl border border-slate-200/80 bg-white shadow-sm dark:border-slate-800 dark:bg-slate-900">
        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs">
            <thead className="bg-slate-50/80 text-[11px] font-semibold uppercase tracking-wider text-slate-400 dark:bg-slate-800/50">
              <tr>
                <th className="px-4 py-3">Ref Utang / Faktur</th>
                <th className="px-4 py-3">Supplier</th>
                <th className="px-4 py-3 text-right">Total Utang Awal</th>
                <th className="px-4 py-3 text-right">Total Dibayar</th>
                <th className="px-4 py-3 text-right">Sisa Utang</th>
                <th className="px-4 py-3 text-center">Jatuh Tempo</th>
                <th className="px-4 py-3 text-center">Status</th>
                <th className="px-4 py-3 text-center">Aksi</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
              {filtered.length === 0 ? (
                <tr>
                  <td colSpan={8} className="py-8 text-center text-slate-400">
                    Tidak ada catatan kartu utang yang sesuai.
                  </td>
                </tr>
              ) : (
                filtered.map((u) => (
                  <tr
                    key={u.utang_id}
                    className="hover:bg-slate-50/50 dark:hover:bg-slate-800/30 transition-colors"
                  >
                    <td className="px-4 py-3 font-mono">
                      <div className="font-semibold text-slate-900 dark:text-white">
                        {u.nomor_referensi}
                      </div>
                      <div className="text-[10px] text-slate-400">{u.nomor_faktur}</div>
                    </td>
                    <td className="px-4 py-3 font-medium text-slate-800 dark:text-slate-200">
                      {u.nama_supplier}
                    </td>
                    <td className="px-4 py-3 text-right font-mono text-slate-600 dark:text-slate-300">
                      {formatRupiah(u.total_utang_awal)}
                    </td>
                    <td className="px-4 py-3 text-right font-mono text-emerald-600 dark:text-emerald-400">
                      {formatRupiah(u.total_dibayar)}
                    </td>
                    <td className="px-4 py-3 text-right font-mono font-bold">
                      {u.sisa_utang > 0 ? (
                        <span className="text-rose-600 dark:text-rose-400">
                          {formatRupiah(u.sisa_utang)}
                        </span>
                      ) : (
                        <span className="text-slate-400">Rp0</span>
                      )}
                    </td>
                    <td className="px-4 py-3 text-center font-mono text-slate-500">
                      {u.tanggal_jatuh_tempo || "-"}
                    </td>
                    <td className="px-4 py-3 text-center">
                      {u.status_utang === "LUNAS" && (
                        <span className="inline-flex items-center gap-1 rounded-full bg-emerald-50 px-2.5 py-0.5 text-[10px] font-bold text-emerald-600 dark:bg-emerald-950/60 dark:text-emerald-400">
                          <CheckCircle2 className="h-3 w-3" /> Lunas
                        </span>
                      )}
                      {u.status_utang === "DIBAYAR_SEBAGIAN" && (
                        <span className="inline-flex items-center gap-1 rounded-full bg-amber-50 px-2.5 py-0.5 text-[10px] font-bold text-amber-600 dark:bg-amber-950/60 dark:text-amber-400">
                          <Clock className="h-3 w-3" /> Dicicil
                        </span>
                      )}
                      {u.status_utang === "BELUM_DIBAYAR" && (
                        <span className="inline-flex items-center gap-1 rounded-full bg-rose-50 px-2.5 py-0.5 text-[10px] font-bold text-rose-600 dark:bg-rose-950/60 dark:text-rose-400">
                          <AlertCircle className="h-3 w-3" /> Belum Dibayar
                        </span>
                      )}
                    </td>
                    <td className="px-4 py-3 text-center">
                      <div className="flex items-center justify-center gap-1.5">
                        <button
                          onClick={() => openHistoryModal(u)}
                          className="rounded-lg p-1.5 text-slate-500 hover:bg-slate-100 dark:hover:bg-slate-800"
                          title="Riwayat Pembayaran Cicilan"
                        >
                          <History className="h-4 w-4" />
                        </button>
                        {u.sisa_utang > 0 && (
                          <button
                            onClick={() => openBayarModal(u)}
                            className="inline-flex items-center gap-1 rounded-xl bg-indigo-600 px-3 py-1 text-xs font-semibold text-white shadow hover:bg-indigo-700"
                          >
                            <CreditCard className="h-3.5 w-3.5" />
                            Bayar
                          </button>
                        )}
                      </div>
                    </td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>
      </div>

      {/* Modal Bayar Cicilan */}
      {modalBayarOpen && selectedUtang && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/60 backdrop-blur-sm p-4">
          <div className="w-full max-w-md rounded-2xl bg-white p-6 shadow-2xl dark:bg-slate-900 border border-slate-100 dark:border-slate-800">
            <div className="flex items-center justify-between border-b border-slate-100 pb-3 dark:border-slate-800">
              <div className="flex items-center gap-2">
                <CreditCard className="h-5 w-5 text-indigo-600" />
                <h3 className="font-bold text-slate-900 dark:text-white text-base">
                  Bayar Cicilan Utang
                </h3>
              </div>
              <button
                onClick={() => setModalBayarOpen(false)}
                className="text-slate-400 hover:text-slate-600"
              >
                <X className="h-5 w-5" />
              </button>
            </div>

            <div className="mt-4 space-y-4 text-xs">
              <div className="rounded-xl bg-slate-50 p-3 dark:bg-slate-800/60 space-y-1">
                <div className="flex justify-between">
                  <span className="text-slate-400">Supplier:</span>
                  <span className="font-semibold text-slate-800 dark:text-slate-200">
                    {selectedUtang.nama_supplier}
                  </span>
                </div>
                <div className="flex justify-between">
                  <span className="text-slate-400">Nomor Faktur:</span>
                  <span className="font-mono text-slate-700 dark:text-slate-300">
                    {selectedUtang.nomor_faktur}
                  </span>
                </div>
                <div className="flex justify-between pt-1 border-t border-slate-200 dark:border-slate-700">
                  <span className="text-slate-500 font-medium">Sisa Utang Saat Ini:</span>
                  <span className="font-mono font-bold text-rose-600 text-sm">
                    {formatRupiah(selectedUtang.sisa_utang)}
                  </span>
                </div>
              </div>

              <div>
                <label className="font-semibold text-slate-700 dark:text-slate-300">
                  Nominal Pembayaran (Rp)
                </label>
                <input
                  type="number"
                  min="1"
                  max={selectedUtang.sisa_utang}
                  value={nominalBayar}
                  onChange={(e) => setNominalBayar(Number(e.target.value))}
                  className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-mono font-bold outline-none focus:ring-2 focus:ring-indigo-500/20 dark:border-slate-700 dark:bg-slate-800 dark:text-white"
                  autoFocus
                />
              </div>

              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="font-semibold text-slate-700 dark:text-slate-300">
                    Metode Bayar
                  </label>
                  <select
                    value={metodeBayar}
                    onChange={(e) => setMetodeBayar(e.target.value as any)}
                    className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 outline-none dark:border-slate-700 dark:bg-slate-800 dark:text-white"
                  >
                    <option value="TUNAI">TUNAI (Kas)</option>
                    <option value="TRANSFER">TRANSFER BANK</option>
                  </select>
                </div>

                <div>
                  <label className="font-semibold text-slate-700 dark:text-slate-300">
                    Tanggal Bayar
                  </label>
                  <input
                    type="date"
                    value={tanggalBayar}
                    onChange={(e) => setTanggalBayar(e.target.value)}
                    className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 outline-none dark:border-slate-700 dark:bg-slate-800 dark:text-white"
                  />
                </div>
              </div>

              <div>
                <label className="font-semibold text-slate-700 dark:text-slate-300">
                  Catatan / Keterangan
                </label>
                <input
                  type="text"
                  value={keteranganBayar}
                  onChange={(e) => setKeteranganBayar(e.target.value)}
                  placeholder="Contoh: Transfer via BCA / Cicilan tahap 2..."
                  className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 outline-none dark:border-slate-700 dark:bg-slate-800 dark:text-white"
                />
              </div>

              <div className="rounded-xl border border-dashed border-slate-200 p-3 text-slate-500 dark:border-slate-800">
                <div className="flex justify-between items-center">
                  <span>Sisa Utang Setelah Pembayaran:</span>
                  <span className="font-mono font-bold text-slate-900 dark:text-white text-sm">
                    {formatRupiah(Math.max(0, selectedUtang.sisa_utang - nominalBayar))}
                  </span>
                </div>
              </div>

              <div className="flex justify-end gap-2 pt-2">
                <button
                  type="button"
                  onClick={() => setModalBayarOpen(false)}
                  className="rounded-xl border border-slate-200 px-4 py-2 font-semibold text-slate-600 hover:bg-slate-50 dark:border-slate-700 dark:text-slate-300"
                >
                  Batal
                </button>
                <button
                  type="button"
                  onClick={handleProcessBayar}
                  disabled={submitting || nominalBayar <= 0}
                  className="rounded-xl bg-indigo-600 px-5 py-2 font-bold text-white shadow-md hover:bg-indigo-700 disabled:opacity-50"
                >
                  {submitting ? "Memproses..." : "Konfirmasi Pembayaran"}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Modal Riwayat Pembayaran Utang */}
      {modalHistoryOpen && selectedUtang && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/60 backdrop-blur-sm p-4">
          <div className="w-full max-w-lg rounded-2xl bg-white p-6 shadow-2xl dark:bg-slate-900 border border-slate-100 dark:border-slate-800">
            <div className="flex items-center justify-between border-b border-slate-100 pb-3 dark:border-slate-800">
              <div className="flex items-center gap-2">
                <Receipt className="h-5 w-5 text-indigo-600" />
                <h3 className="font-bold text-slate-900 dark:text-white text-base">
                  Riwayat Cicilan {selectedUtang.nomor_referensi}
                </h3>
              </div>
              <button
                onClick={() => setModalHistoryOpen(false)}
                className="text-slate-400 hover:text-slate-600"
              >
                <X className="h-5 w-5" />
              </button>
            </div>

            <div className="mt-4 space-y-3">
              <div className="overflow-hidden rounded-xl border border-slate-200 text-xs dark:border-slate-800">
                <table className="w-full text-left">
                  <thead className="bg-slate-50 dark:bg-slate-800/60 text-slate-400 uppercase text-[10px]">
                    <tr>
                      <th className="px-3 py-2">No Bukti & Tanggal</th>
                      <th className="px-3 py-2 text-center">Metode</th>
                      <th className="px-3 py-2 text-right">Nominal</th>
                      <th className="px-3 py-2">Keterangan</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
                    {historyList.length === 0 ? (
                      <tr>
                        <td colSpan={4} className="py-6 text-center text-slate-400">
                          Belum ada riwayat pembayaran cicilan.
                        </td>
                      </tr>
                    ) : (
                      historyList.map((h) => (
                        <tr key={h.pembayaran_utang_id}>
                          <td className="px-3 py-2 font-mono">
                            <div className="font-bold text-slate-800 dark:text-slate-200">
                              {h.nomor_bukti_bayar}
                            </div>
                            <div className="text-[10px] text-slate-400">{h.tanggal_bayar}</div>
                          </td>
                          <td className="px-3 py-2 text-center">
                            <span className="rounded bg-slate-100 px-2 py-0.5 text-[10px] font-semibold dark:bg-slate-800">
                              {h.metode_bayar}
                            </span>
                          </td>
                          <td className="px-3 py-2 text-right font-mono font-bold text-emerald-600">
                            {formatRupiah(h.nominal_bayar)}
                          </td>
                          <td className="px-3 py-2 text-slate-500 text-[11px]">
                            {h.keterangan || "-"}
                          </td>
                        </tr>
                      ))
                    )}
                  </tbody>
                </table>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
