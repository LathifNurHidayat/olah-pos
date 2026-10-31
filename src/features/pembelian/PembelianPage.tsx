import React, { useState, useEffect } from "react";
import {
  Truck,
  Plus,
  Search,
  RefreshCw,
  CreditCard,
  Banknote,
  X,
  Trash2,
  Receipt,
  Eye,
} from "lucide-react";
import { formatRupiah } from "@/lib/utils";

export interface PembelianItem {
  pembelian_id: number;
  nomor_faktur: string;
  tanggal_pembelian: string;
  supplier_id: number;
  nama_supplier: string;
  jenis_pembayaran: "TUNAI" | "KREDIT" | string;
  total_tagihan: number;
  nominal_dibayar: number;
  sisa_utang: number;
  jatuh_tempo: string | null;
  catatan: string | null;
  total_item: number;
}

export interface DetailPembelianItem {
  detail_pembelian_id: number;
  pembelian_id: number;
  barang_id: number;
  kode_barang: string;
  nama_barang: string;
  nama_satuan: string;
  kuantitas: number;
  harga_beli_satuan: number;
  subtotal: number;
}

export interface SupplierOption {
  supplier_id: number;
  nama_pihak: string;
  kode_pihak: string;
}

export interface BarangOption {
  barang_id: number;
  nama_barang: string;
  kode_barang: string;
  harga_beli: number;
  nama_satuan: string;
}

interface NewItemRow {
  barang_id: number;
  nama_barang: string;
  kuantitas: number;
  harga_beli_satuan: number;
}

const DEMO_PEMBELIAN: PembelianItem[] = [
  {
    pembelian_id: 1,
    nomor_faktur: "FAK-20261101-001",
    tanggal_pembelian: "2026-11-01",
    supplier_id: 1,
    nama_supplier: "PT Sumber Pangan Sejahtera",
    jenis_pembayaran: "TUNAI",
    total_tagihan: 1200000,
    nominal_dibayar: 1200000,
    sisa_utang: 0,
    jatuh_tempo: null,
    catatan: "Pengadaan beras premium 20 sak",
    total_item: 1,
  },
  {
    pembelian_id: 2,
    nomor_faktur: "FAK-20261028-004",
    tanggal_pembelian: "2026-10-28",
    supplier_id: 2,
    nama_supplier: "CV Distribusi Utama Nusantara",
    jenis_pembayaran: "KREDIT",
    total_tagihan: 2500000,
    nominal_dibayar: 500000,
    sisa_utang: 2000000,
    jatuh_tempo: "2026-11-28",
    catatan: "Tempo 30 hari",
    total_item: 3,
  },
];

export const PembelianPage: React.FC = () => {
  const [list, setList] = useState<PembelianItem[]>(DEMO_PEMBELIAN);
  const [suppliers, setSuppliers] = useState<SupplierOption[]>([]);
  const [barangs, setBarangs] = useState<BarangOption[]>([]);
  const [searchQuery, setSearchQuery] = useState("");
  const [loading, setLoading] = useState(false);

  // Modal Input Baru
  const [modalNewOpen, setModalNewOpen] = useState(false);
  const [nomorFaktur, setNomorFaktur] = useState("");
  const [tanggal, setTanggal] = useState(new Date().toISOString().split("T")[0]);
  const [selectedSupplierId, setSelectedSupplierId] = useState<number>(1);
  const [jenisPembayaran, setJenisPembayaran] = useState<"TUNAI" | "KREDIT">("TUNAI");
  const [dpNominal, setDpNominal] = useState<number>(0);
  const [jatuhTempo, setJatuhTempo] = useState("");
  const [catatan, setCatatan] = useState("");
  const [cartItems, setCartItems] = useState<NewItemRow[]>([]);
  const [submitting, setSubmitting] = useState(false);

  // Detail Modal
  const [detailModalOpen, setDetailModalOpen] = useState(false);
  const [selectedFaktur, setSelectedFaktur] = useState<PembelianItem | null>(null);
  const [detailItems, setDetailItems] = useState<DetailPembelianItem[]>([]);

  const loadData = async () => {
    setLoading(true);
    if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        const [pembelianData, supData, brgData] = await Promise.all([
          invoke<PembelianItem[]>("get_pembelian_list"),
          invoke<any[]>("get_supplier_list"),
          invoke<any[]>("get_barang_list"),
        ]);
        if (pembelianData.length > 0) setList(pembelianData);
        setSuppliers(supData);
        setBarangs(brgData);
      } catch (err) {
        console.error("Gagal memuat daftar pembelian:", err);
      }
    }
    setLoading(false);
  };

  useEffect(() => {
    loadData();
  }, []);

  const openNewModal = () => {
    setNomorFaktur(`INV-${Date.now().toString().slice(-6)}`);
    setTanggal(new Date().toISOString().split("T")[0]);
    setJenisPembayaran("TUNAI");
    setDpNominal(0);
    setJatuhTempo("");
    setCatatan("");
    setCartItems([]);
    setModalNewOpen(true);
  };

  const addItemToCart = (b: BarangOption) => {
    const existing = cartItems.find((it) => it.barang_id === b.barang_id);
    if (existing) {
      setCartItems((prev) =>
        prev.map((it) =>
          it.barang_id === b.barang_id ? { ...it, kuantitas: it.kuantitas + 1 } : it
        )
      );
    } else {
      setCartItems((prev) => [
        ...prev,
        {
          barang_id: b.barang_id,
          nama_barang: b.nama_barang,
          kuantitas: 1,
          harga_beli_satuan: b.harga_beli,
        },
      ]);
    }
  };

  const updateCartItem = (
    index: number,
    field: "kuantitas" | "harga_beli_satuan",
    val: number
  ) => {
    setCartItems((prev) => {
      const copy = [...prev];
      copy[index] = { ...copy[index], [field]: val };
      return copy;
    });
  };

  const removeCartItem = (index: number) => {
    setCartItems((prev) => prev.filter((_, i) => i !== index));
  };

  const totalCalculated = cartItems.reduce(
    (sum, it) => sum + it.kuantitas * it.harga_beli_satuan,
    0
  );

  const handleSavePembelian = async () => {
    if (!nomorFaktur.trim()) {
      alert("Nomor faktur tidak boleh kosong!");
      return;
    }
    if (cartItems.length === 0) {
      alert("Masukkan minimal 1 barang pembelian!");
      return;
    }

    setSubmitting(true);
    const payload = {
      nomor_faktur: nomorFaktur,
      tanggal_pembelian: tanggal,
      supplier_id: Number(selectedSupplierId) || 1,
      jenis_pembayaran: jenisPembayaran,
      items: cartItems.map((it) => ({
        barang_id: it.barang_id,
        kuantitas: Number(it.kuantitas),
        harga_beli_satuan: Number(it.harga_beli_satuan),
      })),
      nominal_dibayar: jenisPembayaran === "TUNAI" ? totalCalculated : Number(dpNominal),
      jatuh_tempo: jenisPembayaran === "KREDIT" && jatuhTempo ? jatuhTempo : null,
      catatan: catatan || null,
      user_id: 1,
    };

    if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        await invoke("create_pembelian", { input: payload });
        await loadData();
        setModalNewOpen(false);
      } catch (err) {
        alert("Gagal menyimpan pembelian: " + err);
      }
    } else {
      // Local fallback
      const newItem: PembelianItem = {
        pembelian_id: Date.now(),
        nomor_faktur: payload.nomor_faktur,
        tanggal_pembelian: payload.tanggal_pembelian,
        supplier_id: payload.supplier_id,
        nama_supplier: "Supplier Pilihan",
        jenis_pembayaran: payload.jenis_pembayaran,
        total_tagihan: totalCalculated,
        nominal_dibayar: payload.nominal_dibayar,
        sisa_utang: totalCalculated - payload.nominal_dibayar,
        jatuh_tempo: payload.jatuh_tempo,
        catatan: payload.catatan,
        total_item: cartItems.length,
      };
      setList((prev) => [newItem, ...prev]);
      setModalNewOpen(false);
    }
    setSubmitting(false);
  };

  const openDetail = async (p: PembelianItem) => {
    setSelectedFaktur(p);
    if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        const details = await invoke<DetailPembelianItem[]>("get_detail_pembelian", {
          pembelianId: p.pembelian_id,
        });
        setDetailItems(details);
      } catch (e) {
        console.error("Gagal load detail pembelian:", e);
      }
    } else {
      setDetailItems([
        {
          detail_pembelian_id: 1,
          pembelian_id: p.pembelian_id,
          barang_id: 1,
          kode_barang: "BRG-001",
          nama_barang: "Beras Premium Ramos 5kg",
          nama_satuan: "PCS",
          kuantitas: 20,
          harga_beli_satuan: 60000,
          subtotal: 1200000,
        },
      ]);
    }
    setDetailModalOpen(true);
  };

  const filtered = list.filter((p) => {
    return (
      p.nomor_faktur.toLowerCase().includes(searchQuery.toLowerCase()) ||
      p.nama_supplier.toLowerCase().includes(searchQuery.toLowerCase()) ||
      (p.catatan && p.catatan.toLowerCase().includes(searchQuery.toLowerCase()))
    );
  });

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h2 className="text-2xl font-bold tracking-tight text-slate-900 dark:text-white flex items-center gap-2">
            <Truck className="h-6 w-6 text-indigo-600" />
            Pengadaan & Pembelian
          </h2>
          <p className="text-xs text-slate-500 mt-1">
            Catat faktur masuk supplier. Pembelian tunai langsung memotong kas; pembelian kredit
            menambah stok dan membentuk kartu utang.
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
          <button
            onClick={openNewModal}
            className="flex items-center gap-1.5 rounded-xl bg-indigo-600 px-4 py-2 text-xs font-semibold text-white shadow-md shadow-indigo-600/30 hover:bg-indigo-700 transition-all"
          >
            <Plus className="h-4 w-4" /> Faktur Pembelian Baru
          </button>
        </div>
      </div>

      {/* Filter Bar */}
      <div className="flex items-center gap-3 bg-white p-3 rounded-2xl border border-slate-200 shadow-sm dark:bg-slate-900 dark:border-slate-800">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-2.5 h-4 w-4 text-slate-400" />
          <input
            type="text"
            placeholder="Cari nomor faktur atau supplier..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full rounded-xl bg-slate-50 pl-9 pr-4 py-2 text-xs outline-none focus:ring-2 focus:ring-indigo-500/20 border border-slate-200/80 dark:bg-slate-800/60 dark:border-slate-700 dark:text-white"
          />
        </div>
      </div>

      {/* Table List */}
      <div className="overflow-hidden rounded-2xl border border-slate-200/80 bg-white shadow-sm dark:border-slate-800 dark:bg-slate-900">
        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs">
            <thead className="bg-slate-50/80 text-[11px] font-semibold uppercase tracking-wider text-slate-400 dark:bg-slate-800/50">
              <tr>
                <th className="px-4 py-3">Faktur & Tanggal</th>
                <th className="px-4 py-3">Supplier</th>
                <th className="px-4 py-3 text-center">Metode</th>
                <th className="px-4 py-3 text-right">Total Tagihan</th>
                <th className="px-4 py-3 text-right">Dibayar</th>
                <th className="px-4 py-3 text-right">Sisa Utang</th>
                <th className="px-4 py-3">Jatuh Tempo</th>
                <th className="px-4 py-3 text-center">Aksi</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
              {filtered.length === 0 ? (
                <tr>
                  <td colSpan={8} className="py-8 text-center text-slate-400">
                    Belum ada riwayat faktur pembelian.
                  </td>
                </tr>
              ) : (
                filtered.map((p) => (
                  <tr
                    key={p.pembelian_id}
                    className="hover:bg-slate-50/50 dark:hover:bg-slate-800/30 transition-colors"
                  >
                    <td className="px-4 py-3 font-mono">
                      <div className="font-semibold text-slate-900 dark:text-white">
                        {p.nomor_faktur}
                      </div>
                      <div className="text-[10px] text-slate-400">{p.tanggal_pembelian}</div>
                    </td>
                    <td className="px-4 py-3 font-medium text-slate-800 dark:text-slate-200">
                      {p.nama_supplier}
                    </td>
                    <td className="px-4 py-3 text-center">
                      <span
                        className={`inline-flex items-center gap-1 rounded-full px-2.5 py-0.5 text-[10px] font-bold ${
                          p.jenis_pembayaran === "TUNAI"
                            ? "bg-emerald-50 text-emerald-600 dark:bg-emerald-950/60 dark:text-emerald-400"
                            : "bg-amber-50 text-amber-600 dark:bg-amber-950/60 dark:text-amber-400"
                        }`}
                      >
                        {p.jenis_pembayaran === "TUNAI" ? (
                          <Banknote className="h-3 w-3" />
                        ) : (
                          <CreditCard className="h-3 w-3" />
                        )}
                        {p.jenis_pembayaran}
                      </span>
                    </td>
                    <td className="px-4 py-3 text-right font-mono font-bold text-slate-900 dark:text-white">
                      {formatRupiah(p.total_tagihan)}
                    </td>
                    <td className="px-4 py-3 text-right font-mono text-emerald-600 dark:text-emerald-400">
                      {formatRupiah(p.nominal_dibayar)}
                    </td>
                    <td className="px-4 py-3 text-right font-mono font-bold">
                      {p.sisa_utang > 0 ? (
                        <span className="text-rose-600 dark:text-rose-400">
                          {formatRupiah(p.sisa_utang)}
                        </span>
                      ) : (
                        <span className="text-slate-400">Rp0 (Lunas)</span>
                      )}
                    </td>
                    <td className="px-4 py-3 text-slate-500 text-[11px] font-mono">
                      {p.jatuh_tempo || "-"}
                    </td>
                    <td className="px-4 py-3 text-center">
                      <button
                        onClick={() => openDetail(p)}
                        className="inline-flex items-center gap-1 rounded-xl bg-slate-100 px-2.5 py-1 text-xs font-semibold text-slate-700 hover:bg-slate-200 dark:bg-slate-800 dark:text-slate-300"
                      >
                        <Eye className="h-3.5 w-3.5" /> Rincian
                      </button>
                    </td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>
      </div>

      {/* Modal Input Pembelian Baru */}
      {modalNewOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/60 backdrop-blur-sm p-4">
          <div className="w-full max-w-3xl max-h-[90vh] overflow-y-auto rounded-2xl bg-white p-6 shadow-2xl dark:bg-slate-900 border border-slate-100 dark:border-slate-800">
            <div className="flex items-center justify-between border-b border-slate-100 pb-3 dark:border-slate-800">
              <div className="flex items-center gap-2">
                <Truck className="h-5 w-5 text-indigo-600" />
                <h3 className="font-bold text-slate-900 dark:text-white text-base">
                  Faktur Pembelian Baru
                </h3>
              </div>
              <button
                onClick={() => setModalNewOpen(false)}
                className="text-slate-400 hover:text-slate-600"
              >
                <X className="h-5 w-5" />
              </button>
            </div>

            <div className="mt-4 grid grid-cols-1 md:grid-cols-3 gap-4">
              <div>
                <label className="text-xs font-semibold text-slate-700 dark:text-slate-300">
                  Nomor Faktur / Invoice
                </label>
                <input
                  type="text"
                  value={nomorFaktur}
                  onChange={(e) => setNomorFaktur(e.target.value)}
                  className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-xs font-mono outline-none focus:ring-2 focus:ring-indigo-500/20 dark:border-slate-700 dark:bg-slate-800 dark:text-white"
                />
              </div>

              <div>
                <label className="text-xs font-semibold text-slate-700 dark:text-slate-300">
                  Tanggal Pembelian
                </label>
                <input
                  type="date"
                  value={tanggal}
                  onChange={(e) => setTanggal(e.target.value)}
                  className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-xs outline-none focus:ring-2 focus:ring-indigo-500/20 dark:border-slate-700 dark:bg-slate-800 dark:text-white"
                />
              </div>

              <div>
                <label className="text-xs font-semibold text-slate-700 dark:text-slate-300">
                  Supplier
                </label>
                <select
                  value={selectedSupplierId}
                  onChange={(e) => setSelectedSupplierId(Number(e.target.value))}
                  className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-xs outline-none focus:ring-2 focus:ring-indigo-500/20 dark:border-slate-700 dark:bg-slate-800 dark:text-white"
                >
                  {suppliers.length > 0 ? (
                    suppliers.map((s) => (
                      <option key={s.supplier_id} value={s.supplier_id}>
                        {s.nama_pihak}
                      </option>
                    ))
                  ) : (
                    <option value={1}>PT Sumber Pangan Sejahtera</option>
                  )}
                </select>
              </div>
            </div>

            {/* Quick Barang Add */}
            <div className="mt-4 rounded-xl border border-slate-100 bg-slate-50 p-3 dark:border-slate-800 dark:bg-slate-800/40">
              <span className="text-xs font-bold text-slate-700 dark:text-slate-300 block mb-2">
                Pilih Barang Masuk:
              </span>
              <div className="flex flex-wrap gap-2 max-h-32 overflow-y-auto">
                {barangs.map((b) => (
                  <button
                    key={b.barang_id}
                    type="button"
                    onClick={() => addItemToCart(b)}
                    className="inline-flex items-center gap-1.5 rounded-lg border border-slate-200 bg-white px-2.5 py-1 text-xs font-medium text-slate-700 hover:border-indigo-400 hover:text-indigo-600 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-200"
                  >
                    <Plus className="h-3 w-3" />
                    {b.nama_barang}
                  </button>
                ))}
              </div>
            </div>

            {/* Table Cart Items */}
            <div className="mt-4 overflow-hidden rounded-xl border border-slate-200 text-xs dark:border-slate-800">
              <table className="w-full text-left">
                <thead className="bg-slate-50 dark:bg-slate-800/60 text-slate-500 font-semibold">
                  <tr>
                    <th className="px-3 py-2">Barang</th>
                    <th className="px-3 py-2 text-center w-24">Jumlah</th>
                    <th className="px-3 py-2 text-right w-36">Harga Beli Satuan</th>
                    <th className="px-3 py-2 text-right w-32">Subtotal</th>
                    <th className="px-3 py-2 text-center w-12">Hapus</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
                  {cartItems.length === 0 ? (
                    <tr>
                      <td colSpan={5} className="py-6 text-center text-slate-400">
                        Belum ada barang dipilih. Klik tombol barang di atas.
                      </td>
                    </tr>
                  ) : (
                    cartItems.map((it, idx) => (
                      <tr key={it.barang_id}>
                        <td className="px-3 py-2 font-medium">{it.nama_barang}</td>
                        <td className="px-3 py-2">
                          <input
                            type="number"
                            min="1"
                            value={it.kuantitas}
                            onChange={(e) =>
                              updateCartItem(idx, "kuantitas", Math.max(1, Number(e.target.value)))
                            }
                            className="w-full rounded-lg border border-slate-200 px-2 py-1 text-center font-mono outline-none dark:border-slate-700 dark:bg-slate-800"
                          />
                        </td>
                        <td className="px-3 py-2">
                          <input
                            type="number"
                            min="0"
                            step="500"
                            value={it.harga_beli_satuan}
                            onChange={(e) =>
                              updateCartItem(
                                idx,
                                "harga_beli_satuan",
                                Math.max(0, Number(e.target.value))
                              )
                            }
                            className="w-full rounded-lg border border-slate-200 px-2 py-1 text-right font-mono outline-none dark:border-slate-700 dark:bg-slate-800"
                          />
                        </td>
                        <td className="px-3 py-2 text-right font-mono font-bold text-slate-900 dark:text-white">
                          {formatRupiah(it.kuantitas * it.harga_beli_satuan)}
                        </td>
                        <td className="px-3 py-2 text-center">
                          <button
                            type="button"
                            onClick={() => removeCartItem(idx)}
                            className="text-rose-500 hover:text-rose-700"
                          >
                            <Trash2 className="h-4 w-4" />
                          </button>
                        </td>
                      </tr>
                    ))
                  )}
                </tbody>
              </table>
            </div>

            {/* Total & Payment details */}
            <div className="mt-4 grid grid-cols-1 md:grid-cols-2 gap-4 border-t border-slate-100 pt-4 dark:border-slate-800">
              <div className="space-y-3">
                <div>
                  <label className="text-xs font-semibold text-slate-700 dark:text-slate-300">
                    Jenis Pembayaran
                  </label>
                  <div className="mt-1 flex gap-2">
                    <button
                      type="button"
                      onClick={() => setJenisPembayaran("TUNAI")}
                      className={`flex-1 rounded-xl py-2 text-xs font-bold border transition-colors ${
                        jenisPembayaran === "TUNAI"
                          ? "bg-indigo-600 text-white border-indigo-600 shadow"
                          : "border-slate-200 text-slate-600 hover:bg-slate-50 dark:border-slate-700 dark:text-slate-300"
                      }`}
                    >
                      TUNAI (Lunas)
                    </button>
                    <button
                      type="button"
                      onClick={() => setJenisPembayaran("KREDIT")}
                      className={`flex-1 rounded-xl py-2 text-xs font-bold border transition-colors ${
                        jenisPembayaran === "KREDIT"
                          ? "bg-indigo-600 text-white border-indigo-600 shadow"
                          : "border-slate-200 text-slate-600 hover:bg-slate-50 dark:border-slate-700 dark:text-slate-300"
                      }`}
                    >
                      KREDIT (Tempo)
                    </button>
                  </div>
                </div>

                {jenisPembayaran === "KREDIT" && (
                  <div className="grid grid-cols-2 gap-2">
                    <div>
                      <label className="text-xs font-semibold text-slate-700 dark:text-slate-300">
                        Uang Muka / DP (Rp)
                      </label>
                      <input
                        type="number"
                        min="0"
                        value={dpNominal}
                        onChange={(e) => setDpNominal(Number(e.target.value))}
                        className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-1.5 text-xs font-mono outline-none dark:border-slate-700 dark:bg-slate-800"
                      />
                    </div>
                    <div>
                      <label className="text-xs font-semibold text-slate-700 dark:text-slate-300">
                        Tanggal Jatuh Tempo
                      </label>
                      <input
                        type="date"
                        value={jatuhTempo}
                        onChange={(e) => setJatuhTempo(e.target.value)}
                        className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-1.5 text-xs outline-none dark:border-slate-700 dark:bg-slate-800"
                      />
                    </div>
                  </div>
                )}

                <div>
                  <label className="text-xs font-semibold text-slate-700 dark:text-slate-300">
                    Catatan Faktur
                  </label>
                  <input
                    type="text"
                    value={catatan}
                    onChange={(e) => setCatatan(e.target.value)}
                    placeholder="Keterangan pengiriman, nomor PO, dll..."
                    className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-1.5 text-xs outline-none dark:border-slate-700 dark:bg-slate-800"
                  />
                </div>
              </div>

              <div className="rounded-xl bg-slate-50 p-4 dark:bg-slate-800/60 flex flex-col justify-between">
                <div className="space-y-2 text-xs">
                  <div className="flex justify-between text-slate-500">
                    <span>Total Tagihan:</span>
                    <span className="font-mono font-bold text-slate-900 dark:text-white text-base">
                      {formatRupiah(totalCalculated)}
                    </span>
                  </div>
                  <div className="flex justify-between text-slate-500">
                    <span>Nominal Dibayar (Keluar Kas):</span>
                    <span className="font-mono font-bold text-emerald-600">
                      {formatRupiah(
                        jenisPembayaran === "TUNAI" ? totalCalculated : dpNominal
                      )}
                    </span>
                  </div>
                  <div className="flex justify-between text-slate-500 border-t border-dashed border-slate-200 pt-2 dark:border-slate-700">
                    <span>Sisa Utang Pembelian:</span>
                    <span className="font-mono font-bold text-rose-600 text-sm">
                      {formatRupiah(
                        jenisPembayaran === "TUNAI"
                          ? 0
                          : Math.max(0, totalCalculated - dpNominal)
                      )}
                    </span>
                  </div>
                </div>

                <div className="flex justify-end gap-2 pt-4">
                  <button
                    type="button"
                    onClick={() => setModalNewOpen(false)}
                    className="rounded-xl border border-slate-200 px-4 py-2 text-xs font-semibold text-slate-600 hover:bg-slate-100 dark:border-slate-700 dark:text-slate-300"
                  >
                    Batal
                  </button>
                  <button
                    type="button"
                    onClick={handleSavePembelian}
                    disabled={submitting || totalCalculated <= 0}
                    className="rounded-xl bg-indigo-600 px-5 py-2 text-xs font-bold text-white shadow-md hover:bg-indigo-700 disabled:opacity-50"
                  >
                    {submitting ? "Menyimpan..." : "Simpan & Tambah Stok"}
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Modal Detail Pembelian */}
      {detailModalOpen && selectedFaktur && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/60 backdrop-blur-sm p-4">
          <div className="w-full max-w-xl rounded-2xl bg-white p-6 shadow-2xl dark:bg-slate-900 border border-slate-100 dark:border-slate-800">
            <div className="flex items-center justify-between border-b border-slate-100 pb-3 dark:border-slate-800">
              <div className="flex items-center gap-2">
                <Receipt className="h-5 w-5 text-indigo-600" />
                <h3 className="font-bold text-slate-900 dark:text-white text-base">
                  Rincian Faktur {selectedFaktur.nomor_faktur}
                </h3>
              </div>
              <button
                onClick={() => setDetailModalOpen(false)}
                className="text-slate-400 hover:text-slate-600"
              >
                <X className="h-5 w-5" />
              </button>
            </div>

            <div className="mt-4 space-y-3 text-xs">
              <div className="grid grid-cols-2 gap-2 rounded-xl bg-slate-50 p-3 dark:bg-slate-800/50">
                <div>
                  <span className="text-slate-400">Supplier:</span>
                  <div className="font-bold text-slate-800 dark:text-slate-200">
                    {selectedFaktur.nama_supplier}
                  </div>
                </div>
                <div>
                  <span className="text-slate-400">Tanggal:</span>
                  <div className="font-bold text-slate-800 dark:text-slate-200">
                    {selectedFaktur.tanggal_pembelian}
                  </div>
                </div>
              </div>

              <div className="overflow-hidden rounded-xl border border-slate-200 dark:border-slate-800">
                <table className="w-full text-left text-xs">
                  <thead className="bg-slate-50 dark:bg-slate-800/60 text-slate-400 font-semibold uppercase text-[10px]">
                    <tr>
                      <th className="px-3 py-2">Barang</th>
                      <th className="px-3 py-2 text-center">Jumlah</th>
                      <th className="px-3 py-2 text-right">Harga Satuan</th>
                      <th className="px-3 py-2 text-right">Subtotal</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
                    {detailItems.map((d) => (
                      <tr key={d.detail_pembelian_id}>
                        <td className="px-3 py-2">
                          <div className="font-medium">{d.nama_barang}</div>
                          <div className="font-mono text-[10px] text-slate-400">
                            {d.kode_barang}
                          </div>
                        </td>
                        <td className="px-3 py-2 text-center font-mono">
                          {d.kuantitas} {d.nama_satuan}
                        </td>
                        <td className="px-3 py-2 text-right font-mono">
                          {formatRupiah(d.harga_beli_satuan)}
                        </td>
                        <td className="px-3 py-2 text-right font-mono font-bold">
                          {formatRupiah(d.subtotal)}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>

              <div className="flex justify-between items-center pt-2 font-bold text-sm">
                <span>Total Faktur:</span>
                <span className="text-indigo-600 font-mono text-base">
                  {formatRupiah(selectedFaktur.total_tagihan)}
                </span>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
