import { useState } from "react";
import { AppLayout } from "@/components/layout/AppLayout";
import { LoginPage, type UserSession } from "@/features/auth/LoginPage";
import { KasirPage } from "@/features/kasir/KasirPage";
import { BarangPage } from "@/features/barang/BarangPage";
import { InventoriPage } from "@/features/inventori/InventoriPage";
import { PembelianPage } from "@/features/pembelian/PembelianPage";
import { UtangPage } from "@/features/utang/UtangPage";
import { PengaturanPage } from "@/features/pengaturan/PengaturanPage";
import { formatRupiah } from "@/lib/utils";
import {
  TrendingUp,
  Package,
  AlertTriangle,
  Plus,
  ArrowUpRight,
  ArrowDownLeft,
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
      {activeTab === "kasir" && <KasirPage currentUser={currentUser} />}
      {activeTab === "barang" && <BarangPage />}
      {activeTab === "inventori" && <InventoriPage />}
      {activeTab === "pembelian" && <PembelianPage />}
      {activeTab === "utang" && <UtangPage />}
      {activeTab === "pengaturan" && <PengaturanPage currentUser={currentUser} />}
      {activeTab !== "dashboard" &&
        activeTab !== "kasir" &&
        activeTab !== "barang" &&
        activeTab !== "inventori" &&
        activeTab !== "pembelian" &&
        activeTab !== "utang" &&
        activeTab !== "pengaturan" && (
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
