import React, { useState, useEffect } from "react";
import {
  Package,
  Plus,
  Search,
  Filter,
  Edit2,
  Trash2,
  Barcode,
  AlertTriangle,
  CheckCircle2,
  X,
} from "lucide-react";
import { formatRupiah } from "@/lib/utils";

export interface Barang {
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
  catatan: string | null;
  active_st: number;
}

export interface Kategori {
  kategori_id: number;
  kode_kategori: string;
  nama_kategori: string;
}

export interface Satuan {
  satuan_id: number;
  kode_satuan: string;
  nama_satuan: string;
}

const DEMO_BARANG: Barang[] = [
  {
    barang_id: 1,
    kode_barang: "BRG-001",
    barcode: "899123450001",
    nama_barang: "Beras Premium Ramos 5kg",
    kategori_id: 1,
    nama_kategori: "Sembako",
    satuan_id: 1,
    nama_satuan: "PCS",
    harga_beli: 62000,
    harga_jual: 72000,
    stok_saat_ini: 24,
    stok_minimum: 5,
    catatan: "Beras pulen kemasan 5kg",
    active_st: 1,
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
    stok_saat_ini: 48,
    stok_minimum: 10,
    catatan: "Kemasan pouch 2 liter",
    active_st: 1,
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
    stok_saat_ini: 3,
    stok_minimum: 5,
    catatan: "Kopi bubuk roasted medium",
    active_st: 1,
  },
];

export const BarangPage: React.FC = () => {
  const [barangList, setBarangList] = useState<Barang[]>(DEMO_BARANG);
  const [kategoriList, setKategoriList] = useState<Kategori[]>([]);
  const [satuanList, setSatuanList] = useState<Satuan[]>([]);
  const [searchQuery, setSearchQuery] = useState("");
  const [selectedKategori, setSelectedKategori] = useState<string>("ALL");
  const [modalOpen, setModalOpen] = useState(false);
  const [editingBarang, setEditingBarang] = useState<Barang | null>(null);

  // Form State
  const [formData, setFormData] = useState({
    kode_barang: "",
    barcode: "",
    nama_barang: "",
    kategori_id: 1,
    satuan_id: 1,
    harga_beli: 0,
    harga_jual: 0,
    stok_minimum: 5,
    catatan: "",
  });

  const loadData = async () => {
    if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        const [barang, kategori, satuan] = await Promise.all([
          invoke<Barang[]>("get_barang_list"),
          invoke<Kategori[]>("get_kategori_list"),
          invoke<Satuan[]>("get_satuan_list"),
        ]);
        if (barang.length > 0) setBarangList(barang);
        setKategoriList(kategori);
        setSatuanList(satuan);
      } catch (e) {
        console.error("Gagal memuat master barang:", e);
      }
    }
  };

  useEffect(() => {
    loadData();
  }, []);

  const openAddModal = () => {
    setEditingBarang(null);
    setFormData({
      kode_barang: `BRG-${String(barangList.length + 1).padStart(3, "0")}`,
      barcode: "",
      nama_barang: "",
      kategori_id: kategoriList[0]?.kategori_id || 1,
      satuan_id: satuanList[0]?.satuan_id || 1,
      harga_beli: 0,
      harga_jual: 0,
      stok_minimum: 5,
      catatan: "",
    });
    setModalOpen(true);
  };

  const openEditModal = (b: Barang) => {
    setEditingBarang(b);
    setFormData({
      kode_barang: b.kode_barang,
      barcode: b.barcode || "",
      nama_barang: b.nama_barang,
      kategori_id: b.kategori_id,
      satuan_id: b.satuan_id,
      harga_beli: b.harga_beli,
      harga_jual: b.harga_jual,
      stok_minimum: b.stok_minimum,
      catatan: b.catatan || "",
    });
    setModalOpen(true);
  };

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    const payload = {
      barang_id: editingBarang ? editingBarang.barang_id : null,
      kode_barang: formData.kode_barang,
      barcode: formData.barcode || null,
      nama_barang: formData.nama_barang,
      kategori_id: Number(formData.kategori_id),
      satuan_id: Number(formData.satuan_id),
      harga_beli: Number(formData.harga_beli),
      harga_jual: Number(formData.harga_jual),
      stok_minimum: Number(formData.stok_minimum),
      catatan: formData.catatan || null,
    };

    if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        await invoke("save_barang", { input: payload, userId: 1 });
        await loadData();
      } catch (err) {
        alert("Gagal menyimpan barang: " + err);
        return;
      }
    } else {
      // Local fallback
      if (editingBarang) {
        setBarangList((prev) =>
          prev.map((item) =>
            item.barang_id === editingBarang.barang_id
              ? {
                  ...item,
                  kode_barang: payload.kode_barang,
                  barcode: payload.barcode,
                  nama_barang: payload.nama_barang,
                  kategori_id: payload.kategori_id,
                  satuan_id: payload.satuan_id,
                  harga_beli: payload.harga_beli,
                  harga_jual: payload.harga_jual,
                  stok_minimum: payload.stok_minimum,
                  catatan: payload.catatan,
                }
              : item
          )
        );
      } else {
        const newItem: Barang = {
          barang_id: Date.now(),
          kode_barang: payload.kode_barang,
          barcode: payload.barcode,
          nama_barang: payload.nama_barang,
          kategori_id: payload.kategori_id,
          satuan_id: payload.satuan_id,
          harga_beli: payload.harga_beli,
          harga_jual: payload.harga_jual,
          stok_minimum: payload.stok_minimum,
          catatan: payload.catatan,
          nama_kategori: "Kategori",
          nama_satuan: "PCS",
          stok_saat_ini: 0,
          active_st: 1,
        };
        setBarangList((prev) => [...prev, newItem]);
      }
    }
    setModalOpen(false);
  };

  const handleDelete = async (id: number) => {
    if (!confirm("Apakah Anda yakin ingin menghapus barang ini?")) return;
    if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        await invoke("delete_barang", { id, userId: 1 });
        await loadData();
      } catch (err) {
        alert("Gagal menghapus: " + err);
        return;
      }
    } else {
      setBarangList((prev) => prev.filter((b) => b.barang_id !== id));
    }
  };

  const filteredBarang = barangList.filter((b) => {
    const matchesSearch =
      b.nama_barang.toLowerCase().includes(searchQuery.toLowerCase()) ||
      b.kode_barang.toLowerCase().includes(searchQuery.toLowerCase()) ||
      (b.barcode && b.barcode.includes(searchQuery));
    const matchesKategori =
      selectedKategori === "ALL" || b.nama_kategori === selectedKategori;
    return matchesSearch && matchesKategori;
  });

  return (
    <div className="space-y-6">
      {/* Action Header */}
      <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
        <div>
          <h2 className="text-xl font-black text-slate-900 dark:text-white flex items-center gap-2">
            <Package className="h-6 w-6 text-indigo-600" /> Manajemen Data Barang
          </h2>
          <p className="text-xs text-slate-500 mt-0.5">
            Kelola katalog produk, barcode, harga beli & jual, serta batas stok minimum.
          </p>
        </div>
        <button
          onClick={openAddModal}
          className="flex items-center gap-2 rounded-xl bg-indigo-600 px-4 py-2.5 text-sm font-bold text-white shadow-md shadow-indigo-600/30 hover:bg-indigo-700 active:scale-[0.98] transition-all"
        >
          <Plus className="h-4 w-4" /> Tambah Barang Baru
        </button>
      </div>

      {/* Filter and Search Bar */}
      <div className="flex flex-col sm:flex-row items-center gap-3 bg-white p-3 rounded-2xl border border-slate-200 shadow-sm dark:bg-slate-900 dark:border-slate-800">
        <div className="flex flex-1 items-center gap-2 w-full px-2">
          <Search className="h-4 w-4 text-slate-400 shrink-0" />
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Cari berdasarkan nama barang, kode, atau barcode..."
            className="w-full bg-transparent text-sm outline-none placeholder:text-slate-400"
          />
        </div>
        <div className="flex items-center gap-2 w-full sm:w-auto border-t sm:border-t-0 sm:border-l border-slate-100 pt-2 sm:pt-0 sm:pl-3 dark:border-slate-800">
          <Filter className="h-4 w-4 text-slate-400" />
          <select
            value={selectedKategori}
            onChange={(e) => setSelectedKategori(e.target.value)}
            className="bg-transparent text-xs font-semibold text-slate-700 dark:text-slate-300 outline-none"
          >
            <option value="ALL">Semua Kategori</option>
            {kategoriList.map((k) => (
              <option key={k.kategori_id} value={k.nama_kategori}>
                {k.nama_kategori}
              </option>
            ))}
          </select>
        </div>
      </div>

      {/* Modern Data Table */}
      <div className="overflow-hidden rounded-2xl border border-slate-200 bg-white shadow-sm dark:border-slate-800 dark:bg-slate-900">
        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs">
            <thead className="bg-slate-50 text-[11px] font-bold uppercase tracking-wider text-slate-500 border-b border-slate-200 dark:bg-slate-800/50 dark:border-slate-800">
              <tr>
                <th className="px-5 py-3.5">Kode & Barcode</th>
                <th className="px-5 py-3.5">Nama Barang</th>
                <th className="px-5 py-3.5">Kategori / Satuan</th>
                <th className="px-5 py-3.5 text-right">Harga Beli</th>
                <th className="px-5 py-3.5 text-right">Harga Jual</th>
                <th className="px-5 py-3.5 text-center">Stok</th>
                <th className="px-5 py-3.5 text-center">Aksi</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100 dark:divide-slate-800/60 font-medium">
              {filteredBarang.length === 0 ? (
                <tr>
                  <td colSpan={7} className="px-5 py-8 text-center text-slate-400">
                    Tidak ada barang yang cocok dengan kriteria pencarian.
                  </td>
                </tr>
              ) : (
                filteredBarang.map((b) => {
                  const isLowStock = b.stok_saat_ini <= b.stok_minimum;
                  return (
                    <tr
                      key={b.barang_id}
                      className="hover:bg-slate-50/80 dark:hover:bg-slate-800/40 transition-colors"
                    >
                      <td className="px-5 py-3.5">
                        <div className="font-mono font-bold text-slate-900 dark:text-slate-100">
                          {b.kode_barang}
                        </div>
                        {b.barcode && (
                          <div className="flex items-center gap-1 text-[10px] text-slate-400 font-mono mt-0.5">
                            <Barcode className="h-3 w-3" /> {b.barcode}
                          </div>
                        )}
                      </td>
                      <td className="px-5 py-3.5 font-bold text-slate-800 dark:text-slate-200">
                        {b.nama_barang}
                      </td>
                      <td className="px-5 py-3.5">
                        <span className="inline-block rounded-md bg-slate-100 px-2 py-0.5 text-[10px] font-semibold text-slate-600 dark:bg-slate-800 dark:text-slate-300">
                          {b.nama_kategori}
                        </span>
                        <span className="ml-1 text-slate-400 text-[10px]">({b.nama_satuan})</span>
                      </td>
                      <td className="px-5 py-3.5 text-right font-mono text-slate-600 dark:text-slate-400">
                        {formatRupiah(b.harga_beli)}
                      </td>
                      <td className="px-5 py-3.5 text-right font-mono font-bold text-indigo-600 dark:text-indigo-400">
                        {formatRupiah(b.harga_jual)}
                      </td>
                      <td className="px-5 py-3.5 text-center">
                        <span
                          className={`inline-flex items-center gap-1 rounded-full px-2.5 py-0.5 text-[11px] font-bold ${
                            isLowStock
                              ? "bg-amber-100 text-amber-800 dark:bg-amber-950/60 dark:text-amber-300"
                              : "bg-emerald-100 text-emerald-800 dark:bg-emerald-950/60 dark:text-emerald-300"
                          }`}
                        >
                          {isLowStock ? (
                            <AlertTriangle className="h-3 w-3" />
                          ) : (
                            <CheckCircle2 className="h-3 w-3" />
                          )}
                          {b.stok_saat_ini}
                        </span>
                      </td>
                      <td className="px-5 py-3.5 text-center">
                        <div className="flex items-center justify-center gap-1.5">
                          <button
                            onClick={() => openEditModal(b)}
                            className="rounded-lg p-1.5 text-slate-400 hover:bg-slate-100 hover:text-indigo-600 dark:hover:bg-slate-800 transition-colors"
                            title="Edit Barang"
                          >
                            <Edit2 className="h-3.5 w-3.5" />
                          </button>
                          <button
                            onClick={() => handleDelete(b.barang_id)}
                            className="rounded-lg p-1.5 text-slate-400 hover:bg-rose-50 hover:text-rose-600 dark:hover:bg-rose-950/40 transition-colors"
                            title="Hapus Barang"
                          >
                            <Trash2 className="h-3.5 w-3.5" />
                          </button>
                        </div>
                      </td>
                    </tr>
                  );
                })
              )}
            </tbody>
          </table>
        </div>
      </div>

      {/* Modal Tambah / Edit Barang */}
      {modalOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-900/60 backdrop-blur-sm p-4">
          <div className="w-full max-w-lg rounded-3xl border border-slate-200 bg-white shadow-2xl dark:border-slate-800 dark:bg-slate-900 overflow-hidden">
            <div className="flex items-center justify-between border-b border-slate-100 px-6 py-4 dark:border-slate-800">
              <h3 className="font-bold text-base text-slate-900 dark:text-white">
                {editingBarang ? "Edit Data Barang" : "Tambah Barang Baru"}
              </h3>
              <button
                onClick={() => setModalOpen(false)}
                className="rounded-lg p-1 text-slate-400 hover:bg-slate-100 hover:text-slate-600 dark:hover:bg-slate-800"
              >
                <X className="h-5 w-5" />
              </button>
            </div>

            <form onSubmit={handleSave} className="p-6 space-y-4 max-h-[80vh] overflow-y-auto">
              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="text-xs font-semibold text-slate-700 dark:text-slate-300">
                    Kode Barang *
                  </label>
                  <input
                    type="text"
                    value={formData.kode_barang}
                    onChange={(e) => setFormData({ ...formData, kode_barang: e.target.value })}
                    className="w-full mt-1 rounded-xl border border-slate-200 bg-slate-50/50 p-2 text-xs outline-none dark:border-slate-800 dark:bg-slate-800/40 focus:border-indigo-500 font-mono"
                    required
                  />
                </div>
                <div>
                  <label className="text-xs font-semibold text-slate-700 dark:text-slate-300">
                    Barcode (Opsional)
                  </label>
                  <input
                    type="text"
                    value={formData.barcode}
                    onChange={(e) => setFormData({ ...formData, barcode: e.target.value })}
                    className="w-full mt-1 rounded-xl border border-slate-200 bg-slate-50/50 p-2 text-xs outline-none dark:border-slate-800 dark:bg-slate-800/40 focus:border-indigo-500 font-mono"
                    placeholder="Scan barcode..."
                  />
                </div>
              </div>

              <div>
                <label className="text-xs font-semibold text-slate-700 dark:text-slate-300">
                  Nama Barang *
                </label>
                <input
                  type="text"
                  value={formData.nama_barang}
                  onChange={(e) => setFormData({ ...formData, nama_barang: e.target.value })}
                  className="w-full mt-1 rounded-xl border border-slate-200 bg-slate-50/50 p-2 text-xs outline-none dark:border-slate-800 dark:bg-slate-800/40 focus:border-indigo-500 font-medium"
                  required
                />
              </div>

              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="text-xs font-semibold text-slate-700 dark:text-slate-300">
                    Kategori *
                  </label>
                  <select
                    value={formData.kategori_id}
                    onChange={(e) =>
                      setFormData({ ...formData, kategori_id: Number(e.target.value) })
                    }
                    className="w-full mt-1 rounded-xl border border-slate-200 bg-slate-50/50 p-2 text-xs outline-none dark:border-slate-800 dark:bg-slate-800/40 focus:border-indigo-500"
                  >
                    {kategoriList.map((k) => (
                      <option key={k.kategori_id} value={k.kategori_id}>
                        {k.nama_kategori}
                      </option>
                    ))}
                  </select>
                </div>
                <div>
                  <label className="text-xs font-semibold text-slate-700 dark:text-slate-300">
                    Satuan *
                  </label>
                  <select
                    value={formData.satuan_id}
                    onChange={(e) =>
                      setFormData({ ...formData, satuan_id: Number(e.target.value) })
                    }
                    className="w-full mt-1 rounded-xl border border-slate-200 bg-slate-50/50 p-2 text-xs outline-none dark:border-slate-800 dark:bg-slate-800/40 focus:border-indigo-500"
                  >
                    {satuanList.map((s) => (
                      <option key={s.satuan_id} value={s.satuan_id}>
                        {s.nama_satuan} ({s.kode_satuan})
                      </option>
                    ))}
                  </select>
                </div>
              </div>

              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="text-xs font-semibold text-slate-700 dark:text-slate-300">
                    Harga Beli Satuan (Rp) *
                  </label>
                  <input
                    type="number"
                    value={formData.harga_beli}
                    onChange={(e) =>
                      setFormData({ ...formData, harga_beli: Number(e.target.value) })
                    }
                    className="w-full mt-1 rounded-xl border border-slate-200 bg-slate-50/50 p-2 text-xs outline-none dark:border-slate-800 dark:bg-slate-800/40 focus:border-indigo-500 font-mono"
                    required
                    min={0}
                  />
                </div>
                <div>
                  <label className="text-xs font-semibold text-slate-700 dark:text-slate-300">
                    Harga Jual Satuan (Rp) *
                  </label>
                  <input
                    type="number"
                    value={formData.harga_jual}
                    onChange={(e) =>
                      setFormData({ ...formData, harga_jual: Number(e.target.value) })
                    }
                    className="w-full mt-1 rounded-xl border border-slate-200 bg-slate-50/50 p-2 text-xs outline-none dark:border-slate-800 dark:bg-slate-800/40 focus:border-indigo-500 font-mono font-bold text-indigo-600"
                    required
                    min={0}
                  />
                </div>
              </div>

              <div>
                <label className="text-xs font-semibold text-slate-700 dark:text-slate-300">
                  Peringatan Stok Minimum *
                </label>
                <input
                  type="number"
                  value={formData.stok_minimum}
                  onChange={(e) =>
                    setFormData({ ...formData, stok_minimum: Number(e.target.value) })
                  }
                  className="w-full mt-1 rounded-xl border border-slate-200 bg-slate-50/50 p-2 text-xs outline-none dark:border-slate-800 dark:bg-slate-800/40 focus:border-indigo-500 font-mono"
                  required
                  min={0}
                />
              </div>

              <div className="flex justify-end gap-2 pt-2 border-t border-slate-100 dark:border-slate-800">
                <button
                  type="button"
                  onClick={() => setModalOpen(false)}
                  className="rounded-xl px-4 py-2 text-xs font-semibold text-slate-600 hover:bg-slate-100 dark:text-slate-400 dark:hover:bg-slate-800"
                >
                  Batal
                </button>
                <button
                  type="submit"
                  className="rounded-xl bg-indigo-600 px-5 py-2 text-xs font-bold text-white shadow-md shadow-indigo-600/30 hover:bg-indigo-700 active:scale-[0.98]"
                >
                  Simpan Barang
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
};
