import React, { useState, useEffect, useRef } from "react";
import {
  ShoppingCart,
  Search,
  Barcode,
  Plus,
  Minus,
  Trash2,
  Receipt,
  Banknote,
  QrCode,
  CheckCircle2,
  RotateCcw,
  Printer,
  X,
} from "lucide-react";
import { formatRupiah } from "@/lib/utils";
import type { UserSession } from "@/features/auth/LoginPage";

export interface KasirBarangItem {
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
}

export interface CartItem {
  barang_id: number;
  kode_barang: string;
  nama_barang: string;
  nama_satuan: string;
  harga_jual_satuan: number;
  kuantitas: number;
  diskon_item: number;
  stok_saat_ini: number;
}

export interface ReceiptData {
  penjualan_id: number;
  nomor_struk: string;
  tanggal_penjualan: string;
  total_kotor: number;
  total_diskon: number;
  total_bersih: number;
  nominal_diterima: number;
  nominal_kembalian: number;
  metode_pembayaran: string;
  status_pembayaran: string;
  items: {
    barang_id: number;
    nama_barang: string;
    nama_satuan: string;
    kuantitas: number;
    harga_jual_satuan: number;
    subtotal: number;
  }[];
}

const DEMO_PRODUCTS: KasirBarangItem[] = [
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
    stok_saat_ini: 15,
    stok_minimum: 5,
  },
  {
    barang_id: 4,
    kode_barang: "BRG-004",
    barcode: "899123450004",
    nama_barang: "Gula Pasir Kristal Putih 1kg",
    kategori_id: 1,
    nama_kategori: "Sembako",
    satuan_id: 1,
    nama_satuan: "PCS",
    harga_beli: 14500,
    harga_jual: 17500,
    stok_saat_ini: 30,
    stok_minimum: 10,
  },
  {
    barang_id: 5,
    kode_barang: "BRG-005",
    barcode: "899123450005",
    nama_barang: "Teh Celup Melati Kotak",
    kategori_id: 3,
    nama_kategori: "Minuman",
    satuan_id: 1,
    nama_satuan: "PCS",
    harga_beli: 6500,
    harga_jual: 8500,
    stok_saat_ini: 60,
    stok_minimum: 12,
  },
  {
    barang_id: 6,
    kode_barang: "BRG-006",
    barcode: "899123450006",
    nama_barang: "Susu UHT Full Cream 1L",
    kategori_id: 3,
    nama_kategori: "Minuman",
    satuan_id: 1,
    nama_satuan: "PCS",
    harga_beli: 15000,
    harga_jual: 19000,
    stok_saat_ini: 20,
    stok_minimum: 5,
  },
];

export const KasirPage: React.FC<{ currentUser?: UserSession }> = ({ currentUser }) => {
  const [products, setProducts] = useState<KasirBarangItem[]>(DEMO_PRODUCTS);
  const [categories, setCategories] = useState<{ kategori_id: number; nama_kategori: string }[]>(
    []
  );
  const [selectedCategory, setSelectedCategory] = useState<string>("ALL");
  const [searchQuery, setSearchQuery] = useState("");
  const searchInputRef = useRef<HTMLInputElement>(null);

  // Cart State
  const [cart, setCart] = useState<CartItem[]>([]);
  const [diskonTransaksi, setDiskonTransaksi] = useState<number>(0);
  const [metodeBayar, setMetodeBayar] = useState<"TUNAI" | "QRIS">("TUNAI");
  const [nominalDiterima, setNominalDiterima] = useState<number>(0);

  // Struk & Dialog State
  const [checkoutModalOpen, setCheckoutModalOpen] = useState(false);
  const [receiptData, setReceiptData] = useState<ReceiptData | null>(null);
  const [submitting, setSubmitting] = useState(false);

  // Load products & categories from Tauri IPC
  const loadData = async () => {
    if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        const [barangList, catList] = await Promise.all([
          invoke<KasirBarangItem[]>("get_barang_list"),
          invoke<{ kategori_id: number; nama_kategori: string }[]>("get_kategori_list"),
        ]);
        if (barangList.length > 0) setProducts(barangList);
        setCategories(catList);
      } catch (err) {
        console.error("Gagal load katalog kasir:", err);
      }
    }
  };

  useEffect(() => {
    loadData();
    searchInputRef.current?.focus();
  }, []);

  // Keyboard shortcuts (F2: Search focus, F9: Checkout)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "F2") {
        e.preventDefault();
        searchInputRef.current?.focus();
      } else if (e.key === "F9") {
        e.preventDefault();
        if (cart.length > 0) {
          openCheckoutModal();
        }
      } else if (e.key === "Escape") {
        setCheckoutModalOpen(false);
        setReceiptData(null);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [cart]);

  // Handle adding product to cart (or barcode scanning enter)
  const addToCart = (product: KasirBarangItem) => {
    if (product.stok_saat_ini <= 0) {
      alert(`Stok barang "${product.nama_barang}" telah habis!`);
      return;
    }

    setCart((prev) => {
      const existing = prev.find((item) => item.barang_id === product.barang_id);
      if (existing) {
        if (existing.kuantitas + 1 > product.stok_saat_ini) {
          alert(`Jumlah melebihi stok yang tersedia (${product.stok_saat_ini} pcs)!`);
          return prev;
        }
        return prev.map((item) =>
          item.barang_id === product.barang_id
            ? { ...item, kuantitas: item.kuantitas + 1 }
            : item
        );
      } else {
        return [
          ...prev,
          {
            barang_id: product.barang_id,
            kode_barang: product.kode_barang,
            nama_barang: product.nama_barang,
            nama_satuan: product.nama_satuan,
            harga_jual_satuan: product.harga_jual,
            kuantitas: 1,
            diskon_item: 0,
            stok_saat_ini: product.stok_saat_ini,
          },
        ];
      }
    });
  };

  // Barcode search handler (press Enter)
  const handleBarcodeSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!searchQuery.trim()) return;

    // Check barcode exact match first, then kode_barang
    const match = products.find(
      (p) =>
        (p.barcode && p.barcode.toLowerCase() === searchQuery.toLowerCase()) ||
        p.kode_barang.toLowerCase() === searchQuery.toLowerCase()
    );

    if (match) {
      addToCart(match);
      setSearchQuery("");
    }
  };

  const updateQuantity = (barang_id: number, delta: number) => {
    setCart((prev) =>
      prev
        .map((item) => {
          if (item.barang_id === barang_id) {
            const nextQty = item.kuantitas + delta;
            if (nextQty > item.stok_saat_ini) {
              alert(`Jumlah melebihi stok yang tersedia (${item.stok_saat_ini})!`);
              return item;
            }
            return nextQty > 0 ? { ...item, kuantitas: nextQty } : null;
          }
          return item;
        })
        .filter(Boolean) as CartItem[]
    );
  };

  const removeFromCart = (barang_id: number) => {
    setCart((prev) => prev.filter((item) => item.barang_id !== barang_id));
  };

  const clearCart = () => {
    setCart([]);
    setDiskonTransaksi(0);
    setNominalDiterima(0);
  };

  // Calculations
  const totalKotor = cart.reduce(
    (sum, item) => sum + item.kuantitas * item.harga_jual_satuan,
    0
  );
  const totalDiskonItem = cart.reduce((sum, item) => sum + item.diskon_item, 0);
  const totalDiskon = totalDiskonItem + diskonTransaksi;
  const totalBersih = Math.max(0, totalKotor - totalDiskon);
  const kembalian = Math.max(0, nominalDiterima - totalBersih);

  const openCheckoutModal = () => {
    if (cart.length === 0) return;
    setNominalDiterima(totalBersih);
    setCheckoutModalOpen(true);
  };

  const handleProcessPayment = async () => {
    if (metodeBayar === "TUNAI" && nominalDiterima < totalBersih) {
      alert("Nominal pembayaran kurang dari total tagihan!");
      return;
    }

    setSubmitting(true);
    const invoiceNumber = `TRX-${Date.now().toString().slice(-8)}`;
    const today = new Date().toISOString().split("T")[0];

    const payload = {
      nomor_struk: invoiceNumber,
      tanggal_penjualan: today,
      items: cart.map((it) => ({
        barang_id: it.barang_id,
        kuantitas: it.kuantitas,
        harga_jual_satuan: it.harga_jual_satuan,
        diskon_item: it.diskon_item,
      })),
      diskon_transaksi: Number(diskonTransaksi) || 0,
      metode_pembayaran: metodeBayar,
      nominal_diterima: metodeBayar === "TUNAI" ? Number(nominalDiterima) : totalBersih,
      midtrans_transaction_id: metodeBayar === "QRIS" ? `MID-${Date.now()}` : null,
      catatan: null,
      user_id: currentUser?.pengguna_id || 1,
    };

    if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        const receipt = await invoke<ReceiptData>("process_checkout", { input: payload });
        setReceiptData(receipt);
        clearCart();
        setCheckoutModalOpen(false);
        await loadData();
      } catch (err) {
        alert("Gagal memproses transaksi kasir: " + err);
      }
    } else {
      // Local fallback
      const simulatedReceipt: ReceiptData = {
        penjualan_id: Date.now(),
        nomor_struk: invoiceNumber,
        tanggal_penjualan: today,
        total_kotor: totalKotor,
        total_diskon: totalDiskon,
        total_bersih: totalBersih,
        nominal_diterima: payload.nominal_diterima,
        nominal_kembalian: kembalian,
        metode_pembayaran: metodeBayar,
        status_pembayaran: "SUCCESS",
        items: cart.map((it) => ({
          barang_id: it.barang_id,
          nama_barang: it.nama_barang,
          nama_satuan: it.nama_satuan,
          kuantitas: it.kuantitas,
          harga_jual_satuan: it.harga_jual_satuan,
          subtotal: it.kuantitas * it.harga_jual_satuan - it.diskon_item,
        })),
      };
      setReceiptData(simulatedReceipt);
      clearCart();
      setCheckoutModalOpen(false);
    }
    setSubmitting(false);
  };

  const filteredProducts = products.filter((p) => {
    const matchQuery =
      p.nama_barang.toLowerCase().includes(searchQuery.toLowerCase()) ||
      p.kode_barang.toLowerCase().includes(searchQuery.toLowerCase()) ||
      (p.barcode && p.barcode.includes(searchQuery));

    if (selectedCategory === "ALL") return matchQuery;
    return matchQuery && p.kategori_id === Number(selectedCategory);
  });

  return (
    <div className="space-y-4">
      {/* Kasir Grid Container */}
      <div className="grid grid-cols-1 lg:grid-cols-12 gap-5 h-[calc(100vh-120px)]">
        {/* Kolom Kiri: Katalog & Pencarian Barang (7/12) */}
        <div className="lg:col-span-7 flex flex-col gap-4 overflow-hidden">
          {/* Barcode & Search Input Bar */}
          <form
            onSubmit={handleBarcodeSubmit}
            className="flex items-center gap-2 bg-white p-3 rounded-2xl border border-slate-200/90 shadow-sm dark:bg-slate-900 dark:border-slate-800"
          >
            <div className="rounded-xl bg-indigo-50 p-2 text-indigo-600 dark:bg-indigo-950/60 dark:text-indigo-400">
              <Barcode className="h-5 w-5" />
            </div>
            <input
              ref={searchInputRef}
              type="text"
              placeholder="Scan barcode atau ketik nama barang (Tekan Enter / F2)..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="flex-1 bg-transparent text-sm outline-none placeholder:text-slate-400 dark:text-white"
            />
            <button
              type="submit"
              className="rounded-xl bg-indigo-600 px-4 py-2 text-xs font-semibold text-white shadow hover:bg-indigo-700 transition-all flex items-center gap-1.5"
            >
              <Search className="h-3.5 w-3.5" /> Cari
            </button>
          </form>

          {/* Kategori Filter Pills */}
          <div className="flex gap-1.5 overflow-x-auto pb-1 shrink-0">
            <button
              onClick={() => setSelectedCategory("ALL")}
              className={`rounded-xl px-3 py-1.5 text-xs font-semibold whitespace-nowrap transition-colors ${
                selectedCategory === "ALL"
                  ? "bg-indigo-600 text-white shadow-sm"
                  : "bg-white text-slate-600 hover:bg-slate-100 border border-slate-200/70 dark:bg-slate-900 dark:border-slate-800 dark:text-slate-300"
              }`}
            >
              Semua
            </button>
            {categories.map((c) => (
              <button
                key={c.kategori_id}
                onClick={() => setSelectedCategory(c.kategori_id.toString())}
                className={`rounded-xl px-3 py-1.5 text-xs font-semibold whitespace-nowrap transition-colors ${
                  selectedCategory === c.kategori_id.toString()
                    ? "bg-indigo-600 text-white shadow-sm"
                    : "bg-white text-slate-600 hover:bg-slate-100 border border-slate-200/70 dark:bg-slate-900 dark:border-slate-800 dark:text-slate-300"
                }`}
              >
                {c.nama_kategori}
              </button>
            ))}
          </div>

          {/* Product Cards Grid */}
          <div className="flex-1 overflow-y-auto pr-1">
            <div className="grid grid-cols-2 sm:grid-cols-3 gap-3">
              {filteredProducts.map((p) => {
                const isOutOfStock = p.stok_saat_ini <= 0;
                return (
                  <button
                    key={p.barang_id}
                    disabled={isOutOfStock}
                    onClick={() => addToCart(p)}
                    className={`flex flex-col justify-between p-3.5 rounded-2xl border text-left transition-all duration-150 ${
                      isOutOfStock
                        ? "bg-slate-100/70 border-slate-200 opacity-60 cursor-not-allowed dark:bg-slate-800/40 dark:border-slate-800"
                        : "bg-white border-slate-200/80 hover:border-indigo-400 hover:shadow-md dark:bg-slate-900 dark:border-slate-800"
                    }`}
                  >
                    <div>
                      <div className="flex justify-between items-start gap-1">
                        <span className="text-[10px] font-mono font-medium text-slate-400 uppercase">
                          {p.kode_barang}
                        </span>
                        <span
                          className={`text-[10px] font-bold px-1.5 py-0.5 rounded-full ${
                            p.stok_saat_ini <= 0
                              ? "bg-rose-100 text-rose-600 dark:bg-rose-950/60 dark:text-rose-400"
                              : p.stok_saat_ini <= p.stok_minimum
                              ? "bg-amber-100 text-amber-600 dark:bg-amber-950/60 dark:text-amber-400"
                              : "bg-emerald-100 text-emerald-600 dark:bg-emerald-950/60 dark:text-emerald-400"
                          }`}
                        >
                          {p.stok_saat_ini} {p.nama_satuan}
                        </span>
                      </div>
                      <h4 className="font-bold text-xs text-slate-900 dark:text-white mt-1 line-clamp-2">
                        {p.nama_barang}
                      </h4>
                    </div>

                    <div className="mt-3 pt-2 border-t border-slate-100 dark:border-slate-800 flex justify-between items-baseline">
                      <span className="text-xs font-bold text-indigo-600 dark:text-indigo-400">
                        {formatRupiah(p.harga_jual)}
                      </span>
                    </div>
                  </button>
                );
              })}
            </div>
          </div>
        </div>

        {/* Kolom Kanan: Keranjang Kasir & Pembayaran (5/12) */}
        <div className="lg:col-span-5 flex flex-col rounded-2xl border border-slate-200/90 bg-white shadow-sm dark:border-slate-800 dark:bg-slate-900 overflow-hidden">
          {/* Header Keranjang */}
          <div className="flex items-center justify-between border-b border-slate-100 px-5 py-3.5 dark:border-slate-800">
            <div className="flex items-center gap-2">
              <div className="rounded-xl bg-indigo-50 p-1.5 text-indigo-600 dark:bg-indigo-950/60 dark:text-indigo-400">
                <ShoppingCart className="h-4 w-4" />
              </div>
              <h3 className="font-bold text-sm text-slate-900 dark:text-white">Keranjang Kasir</h3>
            </div>
            {cart.length > 0 && (
              <button
                onClick={clearCart}
                className="flex items-center gap-1 text-[11px] font-semibold text-rose-500 hover:text-rose-700"
              >
                <RotateCcw className="h-3 w-3" /> Kosongkan
              </button>
            )}
          </div>

          {/* Daftar Item Keranjang */}
          <div className="flex-1 overflow-y-auto p-4 space-y-2">
            {cart.length === 0 ? (
              <div className="flex h-full flex-col items-center justify-center text-center text-slate-400">
                <Receipt className="h-10 w-10 text-slate-300 dark:text-slate-700" />
                <p className="mt-2 text-xs font-medium">Keranjang masih kosong</p>
                <p className="text-[11px] text-slate-400 mt-0.5">
                  Scan barcode atau klik barang di katalog untuk menambahkan
                </p>
              </div>
            ) : (
              cart.map((item) => (
                <div
                  key={item.barang_id}
                  className="flex items-center justify-between p-2.5 rounded-xl border border-slate-100 bg-slate-50/60 dark:border-slate-800 dark:bg-slate-800/40 text-xs"
                >
                  <div className="flex-1 min-w-0 pr-2">
                    <div className="font-semibold text-slate-900 dark:text-white truncate">
                      {item.nama_barang}
                    </div>
                    <div className="text-[11px] text-slate-400 font-mono">
                      {formatRupiah(item.harga_jual_satuan)} / {item.nama_satuan}
                    </div>
                  </div>

                  {/* Quantity Stepper */}
                  <div className="flex items-center gap-1.5">
                    <button
                      onClick={() => updateQuantity(item.barang_id, -1)}
                      className="rounded-lg bg-white p-1 text-slate-600 border border-slate-200 hover:bg-slate-100 dark:bg-slate-800 dark:border-slate-700 dark:text-slate-300"
                    >
                      <Minus className="h-3 w-3" />
                    </button>
                    <span className="w-6 text-center font-bold font-mono text-slate-800 dark:text-slate-200">
                      {item.kuantitas}
                    </span>
                    <button
                      onClick={() => updateQuantity(item.barang_id, 1)}
                      className="rounded-lg bg-white p-1 text-slate-600 border border-slate-200 hover:bg-slate-100 dark:bg-slate-800 dark:border-slate-700 dark:text-slate-300"
                    >
                      <Plus className="h-3 w-3" />
                    </button>
                  </div>

                  <div className="w-24 text-right font-bold font-mono text-slate-900 dark:text-white pl-2">
                    {formatRupiah(item.kuantitas * item.harga_jual_satuan - item.diskon_item)}
                  </div>

                  <button
                    onClick={() => removeFromCart(item.barang_id)}
                    className="p-1 text-slate-400 hover:text-rose-500 pl-1"
                  >
                    <Trash2 className="h-3.5 w-3.5" />
                  </button>
                </div>
              ))
            )}
          </div>

          {/* Ringkasan Bayar & Tombol Checkout */}
          <div className="border-t border-slate-100 bg-slate-50/60 p-4 dark:border-slate-800 dark:bg-slate-900/60 space-y-2.5">
            <div className="flex justify-between text-xs text-slate-500">
              <span>Subtotal Kotor ({cart.reduce((a, b) => a + b.kuantitas, 0)} pcs)</span>
              <span className="font-mono font-medium">{formatRupiah(totalKotor)}</span>
            </div>

            <div className="flex justify-between items-center text-xs text-slate-500">
              <span>Diskon Tambahan (Rp)</span>
              <input
                type="number"
                min="0"
                value={diskonTransaksi}
                onChange={(e) => setDiskonTransaksi(Math.max(0, Number(e.target.value)))}
                className="w-28 rounded-lg border border-slate-200 px-2 py-0.5 text-right font-mono text-xs dark:border-slate-700 dark:bg-slate-800"
              />
            </div>

            <div className="flex justify-between text-base font-bold text-slate-900 dark:text-white pt-2 border-t border-dashed border-slate-200 dark:border-slate-700">
              <span>Total Bersih</span>
              <span className="text-indigo-600 font-mono text-xl">{formatRupiah(totalBersih)}</span>
            </div>

            <button
              disabled={cart.length === 0}
              onClick={openCheckoutModal}
              className="w-full mt-2 rounded-xl bg-indigo-600 py-3 text-sm font-bold text-white shadow-lg shadow-indigo-600/30 hover:bg-indigo-700 disabled:opacity-50 transition-all flex items-center justify-center gap-2"
            >
              <Receipt className="h-4 w-4" /> Proses Pembayaran (F9)
            </button>
          </div>
        </div>
      </div>

      {/* Modal Checkout Pembayaran */}
      {checkoutModalOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/60 backdrop-blur-sm p-4">
          <div className="w-full max-w-md rounded-2xl bg-white p-6 shadow-2xl dark:bg-slate-900 border border-slate-100 dark:border-slate-800">
            <div className="flex items-center justify-between border-b border-slate-100 pb-3 dark:border-slate-800">
              <h3 className="font-bold text-slate-900 dark:text-white text-base">
                Pilih Metode Pembayaran
              </h3>
              <button
                onClick={() => setCheckoutModalOpen(false)}
                className="text-slate-400 hover:text-slate-600"
              >
                <X className="h-5 w-5" />
              </button>
            </div>

            <div className="mt-4 space-y-4">
              <div className="rounded-xl bg-slate-50 p-4 text-center dark:bg-slate-800/60">
                <span className="text-xs font-semibold text-slate-500 uppercase tracking-wider">
                  Total Tagihan
                </span>
                <div className="text-2xl font-bold font-mono text-indigo-600 mt-1">
                  {formatRupiah(totalBersih)}
                </div>
              </div>

              {/* Method selector */}
              <div className="grid grid-cols-2 gap-3">
                <button
                  type="button"
                  onClick={() => setMetodeBayar("TUNAI")}
                  className={`flex items-center justify-center gap-2 rounded-xl py-2.5 text-xs font-bold border transition-colors ${
                    metodeBayar === "TUNAI"
                      ? "bg-indigo-600 text-white border-indigo-600 shadow"
                      : "border-slate-200 text-slate-700 hover:bg-slate-50 dark:border-slate-700 dark:text-slate-300"
                  }`}
                >
                  <Banknote className="h-4 w-4" /> TUNAI (Cash)
                </button>
                <button
                  type="button"
                  onClick={() => setMetodeBayar("QRIS")}
                  className={`flex items-center justify-center gap-2 rounded-xl py-2.5 text-xs font-bold border transition-colors ${
                    metodeBayar === "QRIS"
                      ? "bg-indigo-600 text-white border-indigo-600 shadow"
                      : "border-slate-200 text-slate-700 hover:bg-slate-50 dark:border-slate-700 dark:text-slate-300"
                  }`}
                >
                  <QrCode className="h-4 w-4" /> QRIS / Midtrans
                </button>
              </div>

              {metodeBayar === "TUNAI" && (
                <div className="space-y-3">
                  <div>
                    <label className="text-xs font-semibold text-slate-700 dark:text-slate-300">
                      Uang Diterima (Rp)
                    </label>
                    <input
                      type="number"
                      min={totalBersih}
                      value={nominalDiterima}
                      onChange={(e) => setNominalDiterima(Number(e.target.value))}
                      className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-base font-mono font-bold outline-none focus:ring-2 focus:ring-indigo-500/20 dark:border-slate-700 dark:bg-slate-800 dark:text-white"
                      autoFocus
                    />
                  </div>

                  {/* Quick Cash Buttons */}
                  <div className="grid grid-cols-3 gap-2">
                    {[totalBersih, 50000, 100000, 150000, 200000, 500000]
                      .filter((val, idx, arr) => arr.indexOf(val) === idx && val >= totalBersih)
                      .slice(0, 6)
                      .map((amt) => (
                        <button
                          key={amt}
                          type="button"
                          onClick={() => setNominalDiterima(amt)}
                          className="rounded-lg border border-slate-200 bg-white py-1.5 text-xs font-mono font-semibold text-slate-700 hover:bg-slate-100 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-300"
                        >
                          {amt === totalBersih ? "Uang Pas" : formatRupiah(amt)}
                        </button>
                      ))}
                  </div>

                  <div className="rounded-xl border border-dashed border-slate-200 p-3 text-xs dark:border-slate-800">
                    <div className="flex justify-between items-center">
                      <span className="text-slate-500">Kembalian:</span>
                      <span className="font-mono font-bold text-emerald-600 text-base">
                        {formatRupiah(kembalian)}
                      </span>
                    </div>
                  </div>
                </div>
              )}

              {metodeBayar === "QRIS" && (
                <div className="rounded-xl border border-dashed border-indigo-200 bg-indigo-50/50 p-4 text-center dark:border-slate-800 dark:bg-slate-800/40">
                  <QrCode className="h-16 w-16 text-indigo-600 mx-auto" />
                  <p className="mt-2 text-xs font-bold text-slate-800 dark:text-slate-200">
                    Midtrans Dynamic QRIS
                  </p>
                  <p className="text-[11px] text-slate-500 mt-0.5">
                    Pelanggan dapat melakukan scan QRIS menggunakan GoPay, OVO, Dana, BCA, atau bank
                    apapun.
                  </p>
                </div>
              )}

              <div className="flex justify-end gap-2 pt-2">
                <button
                  type="button"
                  onClick={() => setCheckoutModalOpen(false)}
                  className="rounded-xl border border-slate-200 px-4 py-2 text-xs font-semibold text-slate-600 hover:bg-slate-50 dark:border-slate-700 dark:text-slate-300"
                >
                  Batal
                </button>
                <button
                  type="button"
                  onClick={handleProcessPayment}
                  disabled={submitting}
                  className="rounded-xl bg-indigo-600 px-5 py-2 text-xs font-bold text-white shadow-md hover:bg-indigo-700 disabled:opacity-50"
                >
                  {submitting ? "Memproses..." : "Selesaikan Transaksi"}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Modal Struk Selesai (Receipt Dialog) */}
      {receiptData && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/60 backdrop-blur-sm p-4">
          <div className="w-full max-w-sm rounded-2xl bg-white p-6 shadow-2xl dark:bg-slate-900 border border-slate-100 dark:border-slate-800 text-xs">
            <div className="text-center pb-4 border-b border-dashed border-slate-200 dark:border-slate-800">
              <div className="flex justify-center mb-1 text-emerald-600">
                <CheckCircle2 className="h-8 w-8" />
              </div>
              <h3 className="font-bold text-slate-900 dark:text-white text-base">
                Transaksi Berhasil
              </h3>
              <span className="font-mono text-slate-400 text-[11px]">
                {receiptData.nomor_struk}
              </span>
              <div className="text-slate-400 text-[10px] mt-0.5">
                {receiptData.tanggal_penjualan}
              </div>
            </div>

            <div className="py-3 border-b border-dashed border-slate-200 dark:border-slate-800 space-y-1.5 max-h-48 overflow-y-auto">
              {receiptData.items.map((it, idx) => (
                <div key={idx} className="flex justify-between">
                  <div>
                    <div className="font-semibold text-slate-800 dark:text-slate-200">
                      {it.nama_barang}
                    </div>
                    <div className="text-[10px] text-slate-400 font-mono">
                      {it.kuantitas} x {formatRupiah(it.harga_jual_satuan)}
                    </div>
                  </div>
                  <div className="font-mono font-medium text-slate-900 dark:text-white">
                    {formatRupiah(it.subtotal)}
                  </div>
                </div>
              ))}
            </div>

            <div className="py-3 border-b border-dashed border-slate-200 dark:border-slate-800 space-y-1 text-[11px]">
              <div className="flex justify-between text-slate-500">
                <span>Total Bersih:</span>
                <span className="font-mono font-bold text-slate-900 dark:text-white">
                  {formatRupiah(receiptData.total_bersih)}
                </span>
              </div>
              <div className="flex justify-between text-slate-500">
                <span>Metode:</span>
                <span className="font-semibold">{receiptData.metode_pembayaran}</span>
              </div>
              <div className="flex justify-between text-slate-500">
                <span>Diterima:</span>
                <span className="font-mono">{formatRupiah(receiptData.nominal_diterima)}</span>
              </div>
              <div className="flex justify-between text-slate-500 font-bold">
                <span>Kembalian:</span>
                <span className="font-mono text-emerald-600">
                  {formatRupiah(receiptData.nominal_kembalian)}
                </span>
              </div>
            </div>

            <div className="flex gap-2 pt-4">
              <button
                type="button"
                onClick={() => alert("Mencetak ke printer thermal ESC/POS...")}
                className="flex-1 rounded-xl border border-slate-200 py-2.5 font-semibold text-slate-700 hover:bg-slate-50 dark:border-slate-700 dark:text-slate-300 flex items-center justify-center gap-1.5"
              >
                <Printer className="h-4 w-4" /> Cetak
              </button>
              <button
                type="button"
                onClick={() => setReceiptData(null)}
                className="flex-1 rounded-xl bg-indigo-600 py-2.5 font-bold text-white shadow hover:bg-indigo-700"
              >
                Transaksi Baru
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
