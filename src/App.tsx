import { useState } from "react";
import { AppLayout } from "@/components/layout/AppLayout";
import { LoginPage, type UserSession } from "@/features/auth/LoginPage";
import { BarangPage } from "@/features/barang/BarangPage";
import { InventoriPage } from "@/features/inventori/InventoriPage";
import { formatRupiah } from "@/lib/utils";
import {
  TrendingUp,
  Package,
  AlertTriangle,
  Receipt,
  Search,
  Plus,
  ArrowUpRight,
  ArrowDownLeft,
  Barcode,
} from "lucide-react";

export function App() {
  const [currentUser, setCurrentUser] = useState<UserSession | null>(null);
  const [activeTab, setActiveTab] = useState("dashboard");

  if (!currentUser) {
    return <LoginPage onLoginSuccess={setCurrentUser} />;
  }

  return (
    <AppLayout
      activeTab={activeTab}
      onSelectTab={setActiveTab}
      currentUser={currentUser}
      onLogout={() => setCurrentUser(null)}
    >
      {activeTab === "dashboard" && <DashboardView user={currentUser} />}
      {activeTab === "kasir" && <KasirPreviewView />}
      {activeTab === "barang" && <BarangPage />}
      {activeTab === "inventori" && <InventoriPage />}
      {activeTab !== "dashboard" &&
        activeTab !== "kasir" &&
        activeTab !== "barang" &&
        activeTab !== "inventori" && (
          <GenericModulePlaceholder title={activeTab} />
        )}
    </AppLayout>
  );
}

function DashboardView({ user }: { user: UserSession }) {
  return (
    <div className="space-y-6">
      {/* Top Banner */}
      <div className="flex flex-col gap-2 rounded-2xl bg-gradient-to-r from-indigo-900 via-indigo-800 to-slate-900 p-6 text-white shadow-lg">
        <div className="inline-flex w-fit items-center gap-1.5 rounded-full bg-white/10 px-3 py-1 text-xs font-semibold backdrop-blur-sm">
          <span>OlahPOS v1.0 Desktop</span>
          <span>•</span>
          <span className="text-emerald-400">Local-First Engine</span>
          <span>•</span>
          <span className="text-indigo-200">Peran: {user.peran}</span>
        </div>
        <h2 className="text-2xl font-bold tracking-tight">
          Selamat Datang, {user.nama_lengkap}
        </h2>
        <p className="text-sm text-indigo-200 max-w-2xl">
          Sistem kasir dan manajemen inventori lokal berkecepatan tinggi dengan dual SQLite database dan pencatatan audit log otomatis.
        </p>
      </div>

      {/* KPI Stats Cards */}
      <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <div className="rounded-2xl border border-slate-200/80 bg-white p-5 shadow-sm dark:border-slate-800 dark:bg-slate-900">
          <div className="flex items-center justify-between">
            <span className="text-xs font-semibold uppercase tracking-wider text-slate-400">
              Penjualan Hari Ini
            </span>
            <div className="rounded-xl bg-emerald-50 p-2 text-emerald-600 dark:bg-emerald-950/60 dark:text-emerald-400">
              <TrendingUp className="h-4 w-4" />
            </div>
          </div>
          <div className="mt-3">
            <span className="text-2xl font-bold text-slate-900 dark:text-white">
              {formatRupiah(2450000)}
            </span>
            <span className="mt-1 flex items-center gap-1 text-xs text-emerald-600 font-medium">
              <ArrowUpRight className="h-3.5 w-3.5" /> 18 Transaksi Berhasil
            </span>
          </div>
        </div>

        <div className="rounded-2xl border border-slate-200/80 bg-white p-5 shadow-sm dark:border-slate-800 dark:bg-slate-900">
          <div className="flex items-center justify-between">
            <span className="text-xs font-semibold uppercase tracking-wider text-slate-400">
              Total Barang Aktif
            </span>
            <div className="rounded-xl bg-indigo-50 p-2 text-indigo-600 dark:bg-indigo-950/60 dark:text-indigo-400">
              <Package className="h-4 w-4" />
            </div>
          </div>
          <div className="mt-3">
            <span className="text-2xl font-bold text-slate-900 dark:text-white">142</span>
            <span className="mt-1 text-xs text-slate-400 block font-medium">
              Tercakup dalam 8 Kategori
            </span>
          </div>
        </div>

        <div className="rounded-2xl border border-slate-200/80 bg-white p-5 shadow-sm dark:border-slate-800 dark:bg-slate-900">
          <div className="flex items-center justify-between">
            <span className="text-xs font-semibold uppercase tracking-wider text-slate-400">
              Stok Menipis
            </span>
            <div className="rounded-xl bg-amber-50 p-2 text-amber-600 dark:bg-amber-950/60 dark:text-amber-400">
              <AlertTriangle className="h-4 w-4" />
            </div>
          </div>
          <div className="mt-3">
            <span className="text-2xl font-bold text-slate-900 dark:text-white">3</span>
            <span className="mt-1 text-xs text-amber-600 font-medium block">
              Perlu pengadaan segera
            </span>
          </div>
        </div>

        <div className="rounded-2xl border border-slate-200/80 bg-white p-5 shadow-sm dark:border-slate-800 dark:bg-slate-900">
          <div className="flex items-center justify-between">
            <span className="text-xs font-semibold uppercase tracking-wider text-slate-400">
              Sisa Utang Pembelian
            </span>
            <div className="rounded-xl bg-rose-50 p-2 text-rose-600 dark:bg-rose-950/60 dark:text-rose-400">
              <ArrowDownLeft className="h-4 w-4" />
            </div>
          </div>
          <div className="mt-3">
            <span className="text-2xl font-bold text-slate-900 dark:text-white">
              {formatRupiah(850000)}
            </span>
            <span className="mt-1 text-xs text-slate-400 font-medium block">
              2 Faktur Tempo Pekan Ini
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}

function KasirPreviewView() {
  return (
    <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
      {/* Katalog Barang / Search */}
      <div className="lg:col-span-2 space-y-4">
        <div className="flex items-center gap-3 bg-white p-3 rounded-2xl border border-slate-200 shadow-sm dark:bg-slate-900 dark:border-slate-800">
          <Barcode className="h-5 w-5 text-indigo-600 shrink-0" />
          <input
            type="text"
            placeholder="Scan barcode atau ketik nama barang (tekan F2 untuk fokus)..."
            className="flex-1 bg-transparent text-sm outline-none placeholder:text-slate-400"
            autoFocus
          />
          <button className="flex items-center gap-1.5 rounded-xl bg-indigo-600 px-3.5 py-1.5 text-xs font-semibold text-white shadow hover:bg-indigo-700">
            <Search className="h-3.5 w-3.5" /> Cari
          </button>
        </div>

        <div className="rounded-2xl border border-slate-200 bg-white p-5 dark:border-slate-800 dark:bg-slate-900">
          <h3 className="font-bold text-slate-900 dark:text-white text-sm mb-3">
            Pintasan Barang Terpopuler
          </h3>
          <div className="grid grid-cols-2 sm:grid-cols-3 gap-3">
            {[
              { id: 1, nama: "Beras Premium 5kg", harga: 72000, stok: 24 },
              { id: 2, nama: "Minyak Goreng 2L", harga: 34000, stok: 48 },
              { id: 3, nama: "Gula Pasir 1kg", harga: 17500, stok: 35 },
              { id: 4, nama: "Kopi Arabika 250g", harga: 45000, stok: 12 },
              { id: 5, nama: "Teh Celup Kotak", harga: 8500, stok: 60 },
              { id: 6, nama: "Susu UHT Full Cream 1L", harga: 19000, stok: 20 },
            ].map((b) => (
              <button
                key={b.id}
                className="flex flex-col text-left p-3 rounded-xl border border-slate-100 hover:border-indigo-300 hover:shadow-md transition-all duration-150 bg-slate-50/50 dark:bg-slate-800/40 dark:border-slate-800"
              >
                <span className="font-semibold text-xs text-slate-800 dark:text-slate-200 truncate">
                  {b.nama}
                </span>
                <span className="text-xs font-bold text-indigo-600 dark:text-indigo-400 mt-1">
                  {formatRupiah(b.harga)}
                </span>
                <span className="text-[10px] text-slate-400 mt-0.5">Sisa: {b.stok} pcs</span>
              </button>
            ))}
          </div>
        </div>
      </div>

      {/* Keranjang Belanja */}
      <div className="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm dark:border-slate-800 dark:bg-slate-900 flex flex-col h-[520px]">
        <div className="flex items-center justify-between border-b border-slate-100 pb-3 dark:border-slate-800">
          <div className="flex items-center gap-2">
            <Receipt className="h-4 w-4 text-indigo-600" />
            <h3 className="font-bold text-sm text-slate-900 dark:text-white">Keranjang Kasir</h3>
          </div>
          <span className="text-xs text-slate-400 font-mono">TRX-20261101-001</span>
        </div>

        <div className="flex-1 overflow-y-auto py-3 space-y-2">
          <div className="flex items-center justify-between p-2 rounded-xl bg-slate-50 dark:bg-slate-800/60 text-xs">
            <div className="flex flex-col">
              <span className="font-semibold text-slate-800 dark:text-slate-200">
                Beras Premium 5kg
              </span>
              <span className="text-slate-400 text-[11px]">1 x {formatRupiah(72000)}</span>
            </div>
            <span className="font-bold text-slate-900 dark:text-white">
              {formatRupiah(72000)}
            </span>
          </div>
        </div>

        {/* Ringkasan Bayar */}
        <div className="border-t border-slate-100 pt-3 dark:border-slate-800 space-y-2">
          <div className="flex justify-between text-xs text-slate-500">
            <span>Subtotal</span>
            <span className="font-semibold">{formatRupiah(72000)}</span>
          </div>
          <div className="flex justify-between text-xs text-slate-500">
            <span>Diskon</span>
            <span className="font-semibold text-emerald-600">Rp0</span>
          </div>
          <div className="flex justify-between text-sm font-bold text-slate-900 dark:text-white pt-2 border-t border-dashed border-slate-200 dark:border-slate-700">
            <span>Total Tagihan</span>
            <span className="text-indigo-600 text-base">{formatRupiah(72000)}</span>
          </div>

          <button className="w-full mt-3 rounded-xl bg-indigo-600 py-3 text-sm font-bold text-white shadow-md shadow-indigo-600/30 hover:bg-indigo-700 transition-all flex items-center justify-center gap-2">
            <Receipt className="h-4 w-4" /> Proses Pembayaran (F9)
          </button>
        </div>
      </div>
    </div>
  );
}

function GenericModulePlaceholder({ title }: { title: string }) {
  return (
    <div className="flex flex-col items-center justify-center rounded-2xl border border-dashed border-slate-300 bg-white p-12 text-center shadow-sm dark:border-slate-800 dark:bg-slate-900">
      <div className="flex h-12 w-12 items-center justify-center rounded-2xl bg-indigo-50 text-indigo-600 dark:bg-indigo-950/60 dark:text-indigo-400">
        <Plus className="h-6 w-6" />
      </div>
      <h3 className="mt-4 text-base font-bold text-slate-900 dark:text-white capitalize">
        Modul {title}
      </h3>
      <p className="mt-1 max-w-sm text-xs text-slate-500">
        Modul ini disiapkan sesuai urutan fase implementasi backend & frontend terpadu OlahPOS.
      </p>
    </div>
  );
}

export default App;
