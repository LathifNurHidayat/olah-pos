import { useState, useEffect } from "react";
import { AppLayout } from "@/components/layout/AppLayout";
import { LoginPage, type UserSession } from "@/features/auth/LoginPage";
import { KasirPage } from "@/features/kasir/KasirPage";
import { BarangPage } from "@/features/barang/BarangPage";
import { InventoriPage } from "@/features/inventori/InventoriPage";
import { PembelianPage } from "@/features/pembelian/PembelianPage";
import { UtangPage } from "@/features/utang/UtangPage";
import { LaporanPage } from "@/features/laporan/LaporanPage";
import { BackupPage } from "@/features/backup/BackupPage";
import { DeveloperPage } from "@/features/developer/DeveloperPage";
import { PengaturanPage } from "@/features/pengaturan/PengaturanPage";
import { formatRupiah } from "@/lib/utils";
import {
  TrendingUp,
  Package,
  AlertTriangle,
  Plus,
  ArrowUpRight,
  ArrowDownLeft,
  Coins,
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
      {activeTab === "laporan" && <LaporanPage />}
      {activeTab === "backup" && <BackupPage currentUser={currentUser} />}
      {activeTab === "pengaturan" && <PengaturanPage currentUser={currentUser} />}
      {activeTab === "developer" && <DeveloperPage currentUser={currentUser} />}
      {activeTab !== "dashboard" &&
        activeTab !== "kasir" &&
        activeTab !== "barang" &&
        activeTab !== "inventori" &&
        activeTab !== "pembelian" &&
        activeTab !== "utang" &&
        activeTab !== "laporan" &&
        activeTab !== "backup" &&
        activeTab !== "pengaturan" &&
        activeTab !== "developer" && (
          <GenericModulePlaceholder title={activeTab} />
        )}
    </AppLayout>
  );
}

function DashboardView({ user }: { user: UserSession }) {
  const [stats, setStats] = useState({
    omset_hari_ini: 0,
    transaksi_hari_ini: 0,
    omset_bulan_ini: 0,
    laba_kotor_hari_ini: 0,
    total_barang_aktif: 0,
    stok_menipis: 0,
    stok_habis: 0,
    sisa_utang: 0,
    kas_masuk_hari_ini: 0,
    kas_keluar_hari_ini: 0,
  });

  useEffect(() => {
    const loadDashboard = async () => {
      if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
        try {
          const { invoke } = await import("@tauri-apps/api/core");
          const data = await invoke<any>("get_dashboard_summary");
          setStats(data);
        } catch (e) {
          console.error("Gagal load summary dashboard:", e);
        }
      } else {
        // Fallback demo numbers
        setStats({
          omset_hari_ini: 2450000,
          transaksi_hari_ini: 18,
          omset_bulan_ini: 15400000,
          laba_kotor_hari_ini: 780000,
          total_barang_aktif: 142,
          stok_menipis: 3,
          stok_habis: 1,
          sisa_utang: 850000,
          kas_masuk_hari_ini: 2450000,
          kas_keluar_hari_ini: 500000,
        });
      }
    };
    loadDashboard();
  }, []);

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
              {formatRupiah(stats.omset_hari_ini)}
            </span>
            <span className="mt-1 flex items-center gap-1 text-xs text-emerald-600 font-medium">
              <ArrowUpRight className="h-3.5 w-3.5" /> {stats.transaksi_hari_ini} Transaksi Berhasil
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
            <span className="text-2xl font-bold text-slate-900 dark:text-white">
              {stats.total_barang_aktif}
            </span>
            <span className="mt-1 text-xs text-slate-400 block font-medium">
              Katalog SKU Terdaftar
            </span>
          </div>
        </div>

        <div className="rounded-2xl border border-slate-200/80 bg-white p-5 shadow-sm dark:border-slate-800 dark:bg-slate-900">
          <div className="flex items-center justify-between">
            <span className="text-xs font-semibold uppercase tracking-wider text-slate-400">
              Stok Menipis / Habis
            </span>
            <div className="rounded-xl bg-amber-50 p-2 text-amber-600 dark:bg-amber-950/60 dark:text-amber-400">
              <AlertTriangle className="h-4 w-4" />
            </div>
          </div>
          <div className="mt-3">
            <span className="text-2xl font-bold text-slate-900 dark:text-white">
              {stats.stok_menipis + stats.stok_habis}
            </span>
            <span className="mt-1 text-xs text-amber-600 font-medium block">
              {stats.stok_habis} habis, {stats.stok_menipis} menipis
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
              {formatRupiah(stats.sisa_utang)}
            </span>
            <span className="mt-1 text-xs text-slate-400 font-medium block">
              Total tagihan supplier belum lunas
            </span>
          </div>
        </div>
      </div>

      {/* Row 2: Laba Kotor & Arus Kas */}
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
        <div className="rounded-2xl border border-slate-200/80 bg-white p-5 shadow-sm dark:border-slate-800 dark:bg-slate-900">
          <div className="flex items-center justify-between">
            <span className="text-xs font-semibold uppercase tracking-wider text-slate-400">
              Estimasi Laba Kotor Hari Ini
            </span>
            <div className="rounded-xl bg-emerald-50 p-2 text-emerald-600 dark:bg-emerald-950/60 dark:text-emerald-400">
              <Coins className="h-4 w-4" />
            </div>
          </div>
          <div className="mt-3">
            <span className="text-2xl font-bold text-emerald-600 dark:text-emerald-400">
              {formatRupiah(stats.laba_kotor_hari_ini)}
            </span>
            <span className="mt-1 text-xs text-slate-400 block font-medium">
              Margin kotor setelah dikurangi HPP produk terjual
            </span>
          </div>
        </div>

        <div className="rounded-2xl border border-slate-200/80 bg-white p-5 shadow-sm dark:border-slate-800 dark:bg-slate-900">
          <div className="flex items-center justify-between">
            <span className="text-xs font-semibold uppercase tracking-wider text-slate-400">
              Omset Penjualan Bulan Ini
            </span>
            <div className="rounded-xl bg-indigo-50 p-2 text-indigo-600 dark:bg-indigo-950/60 dark:text-indigo-400">
              <TrendingUp className="h-4 w-4" />
            </div>
          </div>
          <div className="mt-3">
            <span className="text-2xl font-bold text-indigo-600 dark:text-indigo-400">
              {formatRupiah(stats.omset_bulan_ini)}
            </span>
            <span className="mt-1 text-xs text-slate-400 block font-medium">
              Total penerimaan kasir pada bulan berjalan
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
