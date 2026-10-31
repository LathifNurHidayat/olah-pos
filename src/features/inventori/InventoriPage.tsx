import React, { useState, useEffect } from "react";
import {
  Layers,
  Search,
  Filter,
  RefreshCw,
  AlertTriangle,
  CheckCircle2,
  XCircle,
  TrendingDown,
  TrendingUp,
  PackageCheck,
  ClipboardList,
  Coins,
  X,
  History,
} from "lucide-react";
import { formatRupiah } from "@/lib/utils";

export interface InventoryItem {
  barang_id: number;
  kode_barang: string;
  barcode: string | null;
  nama_barang: string;
  kategori_id: number;
  nama_kategori: string;
  satuan_id: number;
  nama_satuan: string;
  harga_beli: number;
  harga_jual: number;
  stok_saat_ini: number;
  stok_minimum: number;
  status_stok: "AMAN" | "MENIPIS" | "HABIS" | string;
}

export interface StockMutationItem {
  mutasi_id: number;
  barang_id: number;
  kode_barang: string;
  nama_barang: string;
  jenis_mutasi_id: number;
  kode_mutasi: string;
  nama_mutasi: string;
  tanggal_waktu: string;
  stok_masuk: number;
  stok_keluar: number;
  stok_sebelum: number;
  stok_sesudah: number;
  referensi_tabel: string | null;
  referensi_id: number | null;
  keterangan: string | null;
  created_at: string;
  created_by: number | null;
}

export interface InventoryStats {
  total_item: number;
  total_stok_fisik: number;
  stok_menipis_count: number;
  stok_habis_count: number;
  estimasi_nilai_aset: number;
}

const DEMO_ITEMS: InventoryItem[] = [
  {
    barang_id: 1,
    kode_barang: "BRG-001",
    barcode: "899123450001",
    nama_barang: "Beras Premium Ramos 5kg",
    kategori_id: 1,
    nama_kategori: "Sembako",
    satuan_id: 1,
    nama_satuan: "PCS",
    harga_beli: 60000,
    harga_jual: 72000,
    stok_saat_ini: 24,
    stok_minimum: 5,
    status_stok: "AMAN",
  },
  {
    barang_id: 2,
    kode_barang: "BRG-002",
    barcode: "899123450002",
    nama_barang: "Minyak Goreng Sawit 2L",
    kategori_id: 1,
    nama_kategori: "Sembako",
    satuan_id: 1,
    nama_satuan: "PCS",
    harga_beli: 28000,
    harga_jual: 34000,
    stok_saat_ini: 4,
    stok_minimum: 10,
    status_stok: "MENIPIS",
  },
  {
    barang_id: 3,
    kode_barang: "BRG-003",
    barcode: "899123450003",
    nama_barang: "Kopi Arabika Gayo 250g",
    kategori_id: 3,
    nama_kategori: "Minuman",
    satuan_id: 1,
    nama_satuan: "PCS",
    harga_beli: 32000,
    harga_jual: 45000,
    stok_saat_ini: 0,
    stok_minimum: 5,
    status_stok: "HABIS",
  },
];

const DEMO_STATS: InventoryStats = {
  total_item: 3,
  total_stok_fisik: 28,
  stok_menipis_count: 1,
  stok_habis_count: 1,
  estimasi_nilai_aset: 1552000,
};

export const InventoriPage: React.FC = () => {
  const [activeSubTab, setActiveSubTab] = useState<"overview" | "mutasi">("overview");
  const [items, setItems] = useState<InventoryItem[]>(DEMO_ITEMS);
  const [mutations, setMutations] = useState<StockMutationItem[]>([]);
  const [stats, setStats] = useState<InventoryStats>(DEMO_STATS);
  const [searchQuery, setSearchQuery] = useState("");
  const [filterStatus, setFilterStatus] = useState<string>("ALL");
  const [loading, setLoading] = useState(false);

  // Opname Modal state
  const [opnameModalOpen, setOpnameModalOpen] = useState(false);
  const [selectedBarang, setSelectedBarang] = useState<InventoryItem | null>(null);
  const [stokFisikInput, setStokFisikInput] = useState<number>(0);
  const [catatanOpname, setCatatanOpname] = useState("");
  const [opnameSubmitting, setOpnameSubmitting] = useState(false);

  const loadData = async () => {
    setLoading(true);
    if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        const [invData, statData, mutData] = await Promise.all([
          invoke<InventoryItem[]>("get_inventory_overview"),
          invoke<InventoryStats>("get_inventory_stats"),
          invoke<StockMutationItem[]>("get_stock_mutations", {
            barangId: null,
            limit: 50,
            offset: 0,
          }),
        ]);
        if (invData.length > 0) setItems(invData);
        setStats(statData);
        setMutations(mutData);
      } catch (err) {
        console.error("Gagal memuat inventori:", err);
      }
    }
    setLoading(false);
  };

  useEffect(() => {
    loadData();
  }, []);

  const openOpnameModal = (item: InventoryItem) => {
    setSelectedBarang(item);
    setStokFisikInput(item.stok_saat_ini);
    setCatatanOpname("");
    setOpnameModalOpen(true);
  };

  const handleSaveOpname = async () => {
    if (!selectedBarang) return;
    if (stokFisikInput < 0) {
      alert("Stok fisik tidak boleh bernilai negatif!");
      return;
    }

    setOpnameSubmitting(true);
    if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        await invoke("process_stock_opname", {
          input: {
            barang_id: selectedBarang.barang_id,
            stok_fisik: Number(stokFisikInput),
            keterangan: catatanOpname || null,
            user_id: 1,
          },
        });
        await loadData();
        setOpnameModalOpen(false);
      } catch (err) {
        alert("Gagal memproses Stock Opname: " + err);
      }
    } else {
      // Local fallback
      const selisih = stokFisikInput - selectedBarang.stok_saat_ini;
      setItems((prev) =>
        prev.map((it) =>
          it.barang_id === selectedBarang.barang_id
            ? {
                ...it,
                stok_saat_ini: stokFisikInput,
                status_stok:
                  stokFisikInput <= 0
                    ? "HABIS"
                    : stokFisikInput <= it.stok_minimum
                    ? "MENIPIS"
                    : "AMAN",
              }
            : it
        )
      );

      const fakeMutation: StockMutationItem = {
        mutasi_id: Date.now(),
        barang_id: selectedBarang.barang_id,
        kode_barang: selectedBarang.kode_barang,
        nama_barang: selectedBarang.nama_barang,
        jenis_mutasi_id: 4,
        kode_mutasi: "PENYESUAIAN",
        nama_mutasi: "Stock Opname",
        tanggal_waktu: new Date().toISOString(),
        stok_masuk: selisih > 0 ? selisih : 0,
        stok_keluar: selisih < 0 ? Math.abs(selisih) : 0,
        stok_sebelum: selectedBarang.stok_saat_ini,
        stok_sesudah: stokFisikInput,
        referensi_tabel: "STOCK_OPNAME",
        referensi_id: null,
        keterangan: catatanOpname,
        created_at: new Date().toISOString(),
        created_by: 1,
      };
      setMutations((prev) => [fakeMutation, ...prev]);
      setOpnameModalOpen(false);
    }
    setOpnameSubmitting(false);
  };

  const filteredItems = items.filter((item) => {
    const matchQuery =
      item.nama_barang.toLowerCase().includes(searchQuery.toLowerCase()) ||
      item.kode_barang.toLowerCase().includes(searchQuery.toLowerCase()) ||
      (item.barcode && item.barcode.includes(searchQuery));

    if (filterStatus === "ALL") return matchQuery;
    return matchQuery && item.status_stok === filterStatus;
  });

  const filteredMutations = mutations.filter((m) => {
    return (
      m.nama_barang.toLowerCase().includes(searchQuery.toLowerCase()) ||
      m.kode_barang.toLowerCase().includes(searchQuery.toLowerCase()) ||
      m.nama_mutasi.toLowerCase().includes(searchQuery.toLowerCase()) ||
      (m.keterangan && m.keterangan.toLowerCase().includes(searchQuery.toLowerCase()))
    );
  });

  const opnameDiff = selectedBarang ? stokFisikInput - selectedBarang.stok_saat_ini : 0;

  return (
    <div className="space-y-6">
      {/* Top Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h2 className="text-2xl font-bold tracking-tight text-slate-900 dark:text-white flex items-center gap-2">
            <Layers className="h-6 w-6 text-indigo-600" />
            Manajemen Inventori & Stok
          </h2>
          <p className="text-xs text-slate-500 mt-1">
            Pantau ketersediaan fisik, batasi stok minus secara ketat, dan catat riwayat mutasi terpadu.
          </p>
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={loadData}
            disabled={loading}
            className="flex items-center gap-1.5 rounded-xl border border-slate-200 bg-white px-3 py-2 text-xs font-semibold text-slate-700 shadow-sm hover:bg-slate-50 dark:border-slate-800 dark:bg-slate-900 dark:text-slate-300"
          >
            <RefreshCw className={`h-3.5 w-3.5 ${loading ? "animate-spin" : ""}`} />
            Segarkan
          </button>
        </div>
      </div>

      {/* KPI Stats Cards */}
      <div className="grid grid-cols-2 lg:grid-cols-5 gap-3.5">
        <div className="rounded-2xl border border-slate-200/80 bg-white p-4 shadow-sm dark:border-slate-800 dark:bg-slate-900">
          <div className="flex items-center justify-between text-slate-400">
            <span className="text-[11px] font-semibold uppercase tracking-wider">Total Item</span>
            <PackageCheck className="h-4 w-4 text-indigo-500" />
          </div>
          <div className="mt-2 text-xl font-bold text-slate-900 dark:text-white">
            {stats.total_item}
          </div>
          <div className="text-[11px] text-slate-400 mt-0.5">Katalog terdaftar</div>
        </div>

        <div className="rounded-2xl border border-slate-200/80 bg-white p-4 shadow-sm dark:border-slate-800 dark:bg-slate-900">
          <div className="flex items-center justify-between text-slate-400">
            <span className="text-[11px] font-semibold uppercase tracking-wider">Stok Fisik</span>
            <Layers className="h-4 w-4 text-blue-500" />
          </div>
          <div className="mt-2 text-xl font-bold text-slate-900 dark:text-white">
            {stats.total_stok_fisik.toLocaleString()}
          </div>
          <div className="text-[11px] text-slate-400 mt-0.5">Unit di gudang/toko</div>
        </div>

        <div className="rounded-2xl border border-slate-200/80 bg-white p-4 shadow-sm dark:border-slate-800 dark:bg-slate-900">
          <div className="flex items-center justify-between text-slate-400">
            <span className="text-[11px] font-semibold uppercase tracking-wider">Stok Menipis</span>
            <AlertTriangle className="h-4 w-4 text-amber-500" />
          </div>
          <div className="mt-2 text-xl font-bold text-amber-600 dark:text-amber-400">
            {stats.stok_menipis_count}
          </div>
          <div className="text-[11px] text-amber-600/80 dark:text-amber-400/80 mt-0.5">
            Di bawah batas minimum
          </div>
        </div>

        <div className="rounded-2xl border border-slate-200/80 bg-white p-4 shadow-sm dark:border-slate-800 dark:bg-slate-900">
          <div className="flex items-center justify-between text-slate-400">
            <span className="text-[11px] font-semibold uppercase tracking-wider">Stok Habis</span>
            <XCircle className="h-4 w-4 text-rose-500" />
          </div>
          <div className="mt-2 text-xl font-bold text-rose-600 dark:text-rose-400">
            {stats.stok_habis_count}
          </div>
          <div className="text-[11px] text-rose-600/80 dark:text-rose-400/80 mt-0.5">
            Kekosongan stok (0)
          </div>
        </div>

        <div className="rounded-2xl border border-slate-200/80 bg-white p-4 shadow-sm dark:border-slate-800 dark:bg-slate-900 col-span-2 lg:col-span-1">
          <div className="flex items-center justify-between text-slate-400">
            <span className="text-[11px] font-semibold uppercase tracking-wider">Nilai Aset</span>
            <Coins className="h-4 w-4 text-emerald-500" />
          </div>
          <div className="mt-2 text-lg font-bold text-emerald-600 dark:text-emerald-400 truncate">
            {formatRupiah(stats.estimasi_nilai_aset)}
          </div>
          <div className="text-[11px] text-slate-400 mt-0.5">Estimasi harga beli</div>
        </div>
      </div>

      {/* Navigation Sub-tabs */}
      <div className="flex border-b border-slate-200 dark:border-slate-800">
        <button
          onClick={() => setActiveSubTab("overview")}
          className={`flex items-center gap-2 border-b-2 px-4 py-2.5 text-xs font-semibold transition-colors ${
            activeSubTab === "overview"
              ? "border-indigo-600 text-indigo-600 dark:border-indigo-400 dark:text-indigo-400"
              : "border-transparent text-slate-500 hover:text-slate-800 dark:hover:text-slate-200"
          }`}
        >
          <ClipboardList className="h-4 w-4" />
          Ikhtisar Stok & Stock Opname
        </button>
        <button
          onClick={() => setActiveSubTab("mutasi")}
          className={`flex items-center gap-2 border-b-2 px-4 py-2.5 text-xs font-semibold transition-colors ${
            activeSubTab === "mutasi"
              ? "border-indigo-600 text-indigo-600 dark:border-indigo-400 dark:text-indigo-400"
              : "border-transparent text-slate-500 hover:text-slate-800 dark:hover:text-slate-200"
          }`}
        >
          <History className="h-4 w-4" />
          Riwayat Mutasi Stok
        </button>
      </div>

      {/* Subtab 1: Ikhtisar Stok */}
      {activeSubTab === "overview" && (
        <div className="space-y-4">
          {/* Filter Bar */}
          <div className="flex flex-col sm:flex-row items-center gap-3 bg-white p-3 rounded-2xl border border-slate-200 shadow-sm dark:bg-slate-900 dark:border-slate-800">
            <div className="relative flex-1 w-full">
              <Search className="absolute left-3 top-2.5 h-4 w-4 text-slate-400" />
              <input
                type="text"
                placeholder="Cari nama barang, kode, atau barcode..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="w-full rounded-xl bg-slate-50 pl-9 pr-4 py-2 text-xs outline-none focus:ring-2 focus:ring-indigo-500/20 border border-slate-200/80 dark:bg-slate-800/60 dark:border-slate-700 dark:text-white"
              />
            </div>

            <div className="flex items-center gap-2 w-full sm:w-auto">
              <Filter className="h-4 w-4 text-slate-400 shrink-0" />
              <select
                value={filterStatus}
                onChange={(e) => setFilterStatus(e.target.value)}
                className="rounded-xl border border-slate-200/80 bg-slate-50 px-3 py-2 text-xs outline-none focus:ring-2 focus:ring-indigo-500/20 dark:bg-slate-800/60 dark:border-slate-700 dark:text-white w-full sm:w-auto"
              >
                <option value="ALL">Semua Status Stok</option>
                <option value="AMAN">Stok Aman</option>
                <option value="MENIPIS">Stok Menipis</option>
                <option value="HABIS">Stok Habis</option>
              </select>
            </div>
          </div>

          {/* Table View */}
          <div className="overflow-hidden rounded-2xl border border-slate-200/80 bg-white shadow-sm dark:border-slate-800 dark:bg-slate-900">
            <div className="overflow-x-auto">
              <table className="w-full text-left text-xs">
                <thead className="bg-slate-50/80 text-[11px] font-semibold uppercase tracking-wider text-slate-400 dark:bg-slate-800/50">
                  <tr>
                    <th className="px-4 py-3">Kode / Barcode</th>
                    <th className="px-4 py-3">Nama Barang</th>
                    <th className="px-4 py-3">Kategori</th>
                    <th className="px-4 py-3 text-right">Harga Beli</th>
                    <th className="px-4 py-3 text-right">Stok Fisik</th>
                    <th className="px-4 py-3 text-center">Batas Min</th>
                    <th className="px-4 py-3 text-center">Status</th>
                    <th className="px-4 py-3 text-center">Aksi</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
                  {filteredItems.length === 0 ? (
                    <tr>
                      <td colSpan={8} className="py-8 text-center text-slate-400">
                        Tidak ada barang yang cocok dengan filter.
                      </td>
                    </tr>
                  ) : (
                    filteredItems.map((item) => (
                      <tr
                        key={item.barang_id}
                        className="hover:bg-slate-50/50 dark:hover:bg-slate-800/30 transition-colors"
                      >
                        <td className="px-4 py-3 font-mono">
                          <div className="font-semibold text-slate-900 dark:text-white">
                            {item.kode_barang}
                          </div>
                          {item.barcode && (
                            <div className="text-[10px] text-slate-400">{item.barcode}</div>
                          )}
                        </td>
                        <td className="px-4 py-3 font-medium text-slate-800 dark:text-slate-200">
                          {item.nama_barang}
                        </td>
                        <td className="px-4 py-3 text-slate-500">
                          <span className="rounded-lg bg-slate-100 px-2 py-0.5 text-[10px] font-semibold dark:bg-slate-800 dark:text-slate-300">
                            {item.nama_kategori}
                          </span>
                        </td>
                        <td className="px-4 py-3 text-right font-mono text-slate-600 dark:text-slate-300">
                          {formatRupiah(item.harga_beli)}
                        </td>
                        <td className="px-4 py-3 text-right">
                          <span
                            className={`font-mono text-sm font-bold ${
                              item.stok_saat_ini <= 0
                                ? "text-rose-600 dark:text-rose-400"
                                : item.stok_saat_ini <= item.stok_minimum
                                ? "text-amber-600 dark:text-amber-400"
                                : "text-emerald-600 dark:text-emerald-400"
                            }`}
                          >
                            {item.stok_saat_ini} {item.nama_satuan}
                          </span>
                        </td>
                        <td className="px-4 py-3 text-center font-mono text-slate-400">
                          {item.stok_minimum}
                        </td>
                        <td className="px-4 py-3 text-center">
                          {item.status_stok === "HABIS" && (
                            <span className="inline-flex items-center gap-1 rounded-full bg-rose-50 px-2 py-0.5 text-[10px] font-bold text-rose-600 dark:bg-rose-950/60 dark:text-rose-400">
                              <XCircle className="h-3 w-3" /> Habis
                            </span>
                          )}
                          {item.status_stok === "MENIPIS" && (
                            <span className="inline-flex items-center gap-1 rounded-full bg-amber-50 px-2 py-0.5 text-[10px] font-bold text-amber-600 dark:bg-amber-950/60 dark:text-amber-400">
                              <AlertTriangle className="h-3 w-3" /> Menipis
                            </span>
                          )}
                          {item.status_stok === "AMAN" && (
                            <span className="inline-flex items-center gap-1 rounded-full bg-emerald-50 px-2 py-0.5 text-[10px] font-bold text-emerald-600 dark:bg-emerald-950/60 dark:text-emerald-400">
                              <CheckCircle2 className="h-3 w-3" /> Aman
                            </span>
                          )}
                        </td>
                        <td className="px-4 py-3 text-center">
                          <button
                            onClick={() => openOpnameModal(item)}
                            className="inline-flex items-center gap-1 rounded-xl bg-indigo-50 px-2.5 py-1 text-xs font-semibold text-indigo-600 hover:bg-indigo-100 dark:bg-indigo-950/60 dark:text-indigo-400 dark:hover:bg-indigo-900"
                          >
                            <ClipboardList className="h-3.5 w-3.5" />
                            Stock Opname
                          </button>
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

      {/* Subtab 2: Riwayat Mutasi Stok */}
      {activeSubTab === "mutasi" && (
        <div className="space-y-4">
          {/* Filter Bar */}
          <div className="flex items-center gap-3 bg-white p-3 rounded-2xl border border-slate-200 shadow-sm dark:bg-slate-900 dark:border-slate-800">
            <div className="relative flex-1">
              <Search className="absolute left-3 top-2.5 h-4 w-4 text-slate-400" />
              <input
                type="text"
                placeholder="Cari transaksi mutasi, jenis, atau catatan..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="w-full rounded-xl bg-slate-50 pl-9 pr-4 py-2 text-xs outline-none focus:ring-2 focus:ring-indigo-500/20 border border-slate-200/80 dark:bg-slate-800/60 dark:border-slate-700 dark:text-white"
              />
            </div>
          </div>

          <div className="overflow-hidden rounded-2xl border border-slate-200/80 bg-white shadow-sm dark:border-slate-800 dark:bg-slate-900">
            <div className="overflow-x-auto">
              <table className="w-full text-left text-xs">
                <thead className="bg-slate-50/80 text-[11px] font-semibold uppercase tracking-wider text-slate-400 dark:bg-slate-800/50">
                  <tr>
                    <th className="px-4 py-3">Waktu</th>
                    <th className="px-4 py-3">Barang</th>
                    <th className="px-4 py-3">Jenis Mutasi</th>
                    <th className="px-4 py-3 text-center">Pergerakan</th>
                    <th className="px-4 py-3 text-center">Stok Sebelum</th>
                    <th className="px-4 py-3 text-center">Stok Sesudah</th>
                    <th className="px-4 py-3">Keterangan</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
                  {filteredMutations.length === 0 ? (
                    <tr>
                      <td colSpan={7} className="py-8 text-center text-slate-400">
                        Belum ada catatan riwayat mutasi stok.
                      </td>
                    </tr>
                  ) : (
                    filteredMutations.map((m) => (
                      <tr
                        key={m.mutasi_id}
                        className="hover:bg-slate-50/50 dark:hover:bg-slate-800/30 transition-colors"
                      >
                        <td className="px-4 py-3 text-slate-500 font-mono text-[11px] whitespace-nowrap">
                          {new Date(m.tanggal_waktu).toLocaleString("id-ID")}
                        </td>
                        <td className="px-4 py-3">
                          <div className="font-semibold text-slate-800 dark:text-slate-200">
                            {m.nama_barang}
                          </div>
                          <div className="text-[10px] font-mono text-slate-400">
                            {m.kode_barang}
                          </div>
                        </td>
                        <td className="px-4 py-3">
                          <span className="rounded-lg bg-indigo-50 px-2 py-0.5 text-[10px] font-semibold text-indigo-700 dark:bg-indigo-950/60 dark:text-indigo-300">
                            {m.nama_mutasi}
                          </span>
                        </td>
                        <td className="px-4 py-3 text-center font-mono font-bold">
                          {m.stok_masuk > 0 && (
                            <span className="inline-flex items-center gap-0.5 text-emerald-600 dark:text-emerald-400">
                              <TrendingUp className="h-3 w-3" /> +{m.stok_masuk}
                            </span>
                          )}
                          {m.stok_keluar > 0 && (
                            <span className="inline-flex items-center gap-0.5 text-rose-600 dark:text-rose-400">
                              <TrendingDown className="h-3 w-3" /> -{m.stok_keluar}
                            </span>
                          )}
                          {m.stok_masuk === 0 && m.stok_keluar === 0 && (
                            <span className="text-slate-400">0</span>
                          )}
                        </td>
                        <td className="px-4 py-3 text-center font-mono text-slate-500">
                          {m.stok_sebelum}
                        </td>
                        <td className="px-4 py-3 text-center font-mono font-bold text-slate-800 dark:text-slate-200">
                          {m.stok_sesudah}
                        </td>
                        <td className="px-4 py-3 text-slate-500 text-[11px] max-w-xs truncate">
                          {m.keterangan || "-"}
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

      {/* Modal Stock Opname */}
      {opnameModalOpen && selectedBarang && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/60 backdrop-blur-sm p-4">
          <div className="w-full max-w-md rounded-2xl bg-white p-6 shadow-2xl dark:bg-slate-900 border border-slate-100 dark:border-slate-800">
            <div className="flex items-center justify-between border-b border-slate-100 pb-3 dark:border-slate-800">
              <div className="flex items-center gap-2">
                <ClipboardList className="h-5 w-5 text-indigo-600" />
                <h3 className="font-bold text-slate-900 dark:text-white text-base">
                  Stock Opname Fisik
                </h3>
              </div>
              <button
                onClick={() => setOpnameModalOpen(false)}
                className="text-slate-400 hover:text-slate-600"
              >
                <X className="h-5 w-5" />
              </button>
            </div>

            <div className="mt-4 space-y-4">
              <div className="rounded-xl bg-slate-50 p-3 dark:bg-slate-800/60 text-xs">
                <div className="text-[11px] text-slate-400 uppercase font-semibold">Barang</div>
                <div className="font-bold text-slate-900 dark:text-white text-sm">
                  {selectedBarang.nama_barang}
                </div>
                <div className="flex justify-between items-center mt-2 text-slate-600 dark:text-slate-300">
                  <span>Stok Sistem Saat Ini:</span>
                  <span className="font-mono font-bold text-indigo-600 text-sm">
                    {selectedBarang.stok_saat_ini} {selectedBarang.nama_satuan}
                  </span>
                </div>
              </div>

              <div>
                <label className="text-xs font-semibold text-slate-700 dark:text-slate-300">
                  Stok Fisik Aktual (Hasil Hitung Fisik)
                </label>
                <input
                  type="number"
                  min="0"
                  value={stokFisikInput}
                  onChange={(e) => setStokFisikInput(Number(e.target.value))}
                  className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-mono font-bold outline-none focus:ring-2 focus:ring-indigo-500/20 dark:border-slate-700 dark:bg-slate-800 dark:text-white"
                  autoFocus
                />
              </div>

              {/* Selisih Badge */}
              <div className="rounded-xl border border-dashed border-slate-200 p-3 text-xs dark:border-slate-800">
                <div className="flex justify-between items-center">
                  <span className="text-slate-500">Selisih Penyesuaian:</span>
                  <span
                    className={`font-mono font-bold text-sm ${
                      opnameDiff > 0
                        ? "text-emerald-600 dark:text-emerald-400"
                        : opnameDiff < 0
                        ? "text-rose-600 dark:text-rose-400"
                        : "text-slate-400"
                    }`}
                  >
                    {opnameDiff > 0 ? `+${opnameDiff}` : opnameDiff} {selectedBarang.nama_satuan}
                  </span>
                </div>
                <div className="text-[11px] text-slate-400 mt-1">
                  {opnameDiff > 0 && "Sistem akan mencatat mutasi MASUK untuk penyesuaian opname."}
                  {opnameDiff < 0 && "Sistem akan mencatat mutasi KELUAR untuk penyesuaian opname."}
                  {opnameDiff === 0 && "Jumlah fisik sama dengan data sistem, tidak ada selisih."}
                </div>
              </div>

              <div>
                <label className="text-xs font-semibold text-slate-700 dark:text-slate-300">
                  Alasan / Keterangan Penyesuaian
                </label>
                <textarea
                  value={catatanOpname}
                  onChange={(e) => setCatatanOpname(e.target.value)}
                  placeholder="Contoh: Selisih setelah penghitungan fisik bulanan / barang rusak..."
                  rows={2}
                  className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-xs outline-none focus:ring-2 focus:ring-indigo-500/20 dark:border-slate-700 dark:bg-slate-800 dark:text-white"
                />
              </div>

              <div className="flex justify-end gap-2 pt-2">
                <button
                  type="button"
                  onClick={() => setOpnameModalOpen(false)}
                  className="rounded-xl border border-slate-200 px-4 py-2 text-xs font-semibold text-slate-600 hover:bg-slate-50 dark:border-slate-700 dark:text-slate-300"
                >
                  Batal
                </button>
                <button
                  type="button"
                  onClick={handleSaveOpname}
                  disabled={opnameSubmitting}
                  className="rounded-xl bg-indigo-600 px-4 py-2 text-xs font-semibold text-white shadow-md hover:bg-indigo-700"
                >
                  {opnameSubmitting ? "Menyimpan..." : "Simpan Penyesuaian Opname"}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
