import React, { useState, useEffect } from "react";
import {
  HardDriveDownload,
  Database,
  Clock,
  RefreshCw,
  CheckCircle2,
  AlertTriangle,
  FolderArchive,
  RotateCcw,
  Calendar,
  X,
} from "lucide-react";
import type { UserSession } from "@/features/auth/LoginPage";

export interface BackupItem {
  backup_id: string;
  folder_name: string;
  path: string;
  total_size_bytes: number;
  total_size_formatted: string;
  created_at: string;
  cloud_synced: boolean;
}

export interface BackupResult {
  backup_id: string;
  path: string;
  total_size_bytes: number;
  deleted_old_backups: string[];
  cloud_status: string;
}

export const BackupPage: React.FC<{ currentUser?: UserSession }> = ({ currentUser: _currentUser }) => {
  const [backups, setBackups] = useState<BackupItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [backingUp, setBackingUp] = useState(false);
  const [backupSuccess, setBackupSuccess] = useState<BackupResult | null>(null);
  const [schedulerMessage, setSchedulerMessage] = useState<string | null>(null);

  // Restore Modal State
  const [restoreModalOpen, setRestoreModalOpen] = useState(false);
  const [selectedBackup, setSelectedBackup] = useState<BackupItem | null>(null);
  const [restoring, setRestoring] = useState(false);

  const loadBackups = async () => {
    setLoading(true);
    if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        const list = await invoke<BackupItem[]>("list_database_backups");
        setBackups(list);
      } catch (err) {
        console.error("Gagal load daftar backup:", err);
      }
    } else {
      // Local demo fallback
      setBackups([
        {
          backup_id: "backup_20261101_060000",
          folder_name: "backup_20261101_060000",
          path: "/home/user/.local/share/olahpos/backups/backup_20261101_060000",
          total_size_bytes: 524288,
          total_size_formatted: "512.0 KB",
          created_at: "2026-11-01T06:00:00Z",
          cloud_synced: true,
        },
      ]);
    }
    setLoading(false);
  };

  useEffect(() => {
    loadBackups();
  }, []);

  const handleCreateBackup = async () => {
    setBackingUp(true);
    setBackupSuccess(null);
    if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        const res = await invoke<BackupResult>("create_database_backup");
        setBackupSuccess(res);
        await loadBackups();
      } catch (err) {
        alert("Gagal melakukan pencadangan database: " + err);
      }
    } else {
      // Demo fallback
      const fakeResult: BackupResult = {
        backup_id: `backup_${Date.now()}`,
        path: `/home/user/.local/share/olahpos/backups/backup_${Date.now()}`,
        total_size_bytes: 532000,
        deleted_old_backups: [],
        cloud_status: "LOCAL_SUCCESS",
      };
      setBackupSuccess(fakeResult);
      await loadBackups();
    }
    setBackingUp(false);
  };

  const handleSetupScheduler = async () => {
    if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        const msg = await invoke<string>("setup_backup_scheduler");
        setSchedulerMessage(msg);
      } catch (err) {
        alert("Gagal mengonfigurasi scheduler OS: " + err);
      }
    } else {
      setSchedulerMessage(
        "Scheduler OS aktif via systemd user timer: ~/.config/systemd/user/olahpos-backup.timer"
      );
    }
  };

  const openRestoreModal = (item: BackupItem) => {
    setSelectedBackup(item);
    setRestoreModalOpen(true);
  };

  const handleConfirmRestore = async () => {
    if (!selectedBackup) return;
    setRestoring(true);
    if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        await invoke("restore_database_backup", { backupFolder: selectedBackup.path });
        alert(
          "Database berhasil dipulihkan dari snapshot! Aplikasi akan memuat data terbaru."
        );
        window.location.reload();
      } catch (err) {
        alert("Gagal memulihkan database: " + err);
      }
    } else {
      alert("Simulasi pemulihan database dari snapshot berhasil.");
      setRestoreModalOpen(false);
    }
    setRestoring(false);
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h2 className="text-2xl font-bold tracking-tight text-slate-900 dark:text-white flex items-center gap-2">
            <HardDriveDownload className="h-6 w-6 text-indigo-600" />
            Pencadangan & Pemulihan (Backup & Restore)
          </h2>
          <p className="text-xs text-slate-500 mt-1">
            Snapshot atomik dual SQLite database (kasir_pos.db & kasir_pos_log.db). Kebijakan
            retensi otomatis 10 backup lokal terbaru.
          </p>
        </div>

        <button
          onClick={loadBackups}
          disabled={loading}
          className="flex items-center gap-1.5 rounded-xl border border-slate-200 bg-white px-3 py-2 text-xs font-semibold text-slate-700 shadow-sm hover:bg-slate-50 dark:border-slate-800 dark:bg-slate-900 dark:text-slate-300"
        >
          <RefreshCw className={`h-3.5 w-3.5 ${loading ? "animate-spin" : ""}`} />
          Segarkan
        </button>
      </div>

      {/* Action Cards */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {/* Card 1: Trigger Backup */}
        <div className="rounded-2xl border border-slate-200/80 bg-white p-5 shadow-sm dark:border-slate-800 dark:bg-slate-900 flex flex-col justify-between space-y-4">
          <div className="space-y-2">
            <div className="flex items-center gap-2 text-indigo-600">
              <Database className="h-5 w-5" />
              <h3 className="font-bold text-sm text-slate-900 dark:text-white">
                Buat Snapshot Database Baru
              </h3>
            </div>
            <p className="text-xs text-slate-500 leading-relaxed">
              Merekam salinan konsisten database transaksi dan database audit log tanpa menghentikan
              operasional kasir. Data dikemas dengan manifest metadata versi.
            </p>
            <div className="inline-flex items-center gap-1.5 rounded-full bg-slate-100 px-2.5 py-1 text-[11px] font-semibold text-slate-600 dark:bg-slate-800 dark:text-slate-300">
              <Clock className="h-3.5 w-3.5 text-indigo-500" />
              Retensi: Maksimal 10 snapshot terbaru dipertahankan
            </div>
          </div>

          <button
            onClick={handleCreateBackup}
            disabled={backingUp}
            className="w-full rounded-xl bg-indigo-600 py-2.5 text-xs font-bold text-white shadow-md shadow-indigo-600/30 hover:bg-indigo-700 transition-all flex items-center justify-center gap-2 disabled:opacity-50"
          >
            <FolderArchive className="h-4 w-4" />
            {backingUp ? "Memproses Snapshot Atomic..." : "Cadangkan Database Sekarang"}
          </button>
        </div>

        {/* Card 2: Scheduler OS */}
        <div className="rounded-2xl border border-slate-200/80 bg-white p-5 shadow-sm dark:border-slate-800 dark:bg-slate-900 flex flex-col justify-between space-y-4">
          <div className="space-y-2">
            <div className="flex items-center gap-2 text-emerald-600">
              <Calendar className="h-5 w-5" />
              <h3 className="font-bold text-sm text-slate-900 dark:text-white">
                Otomasi Background Scheduler OS
              </h3>
            </div>
            <p className="text-xs text-slate-500 leading-relaxed">
              Konfigurasi systemd user timer (Linux) atau Windows Task Scheduler untuk mengeksekusi
              pencadangan otomatis setiap hari di latar belakang.
            </p>
            {schedulerMessage ? (
              <div className="rounded-xl bg-emerald-50 p-2.5 text-[11px] font-medium text-emerald-700 dark:bg-emerald-950/60 dark:text-emerald-400">
                {schedulerMessage}
              </div>
            ) : (
              <div className="text-[11px] text-slate-400">
                Belum diatur atau menggunakan interval harian sistem.
              </div>
            )}
          </div>

          <button
            onClick={handleSetupScheduler}
            className="w-full rounded-xl border border-slate-200 bg-slate-50 py-2.5 text-xs font-bold text-slate-700 hover:bg-slate-100 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-300 transition-all flex items-center justify-center gap-2"
          >
            <Clock className="h-4 w-4 text-emerald-600" />
            Pasang / Perbarui Timer Otomatis
          </button>
        </div>
      </div>

      {/* Success Banner */}
      {backupSuccess && (
        <div className="flex items-start gap-3 rounded-2xl border border-emerald-200 bg-emerald-50 p-4 dark:border-emerald-900/60 dark:bg-emerald-950/40 text-xs">
          <CheckCircle2 className="h-5 w-5 text-emerald-600 shrink-0 mt-0.5" />
          <div className="space-y-1">
            <div className="font-bold text-emerald-900 dark:text-emerald-200 text-sm">
              Snapshot Berhasil Disimpan: {backupSuccess.backup_id}
            </div>
            <div className="text-emerald-800 dark:text-emerald-300 font-mono text-[11px]">
              Lokasi: {backupSuccess.path}
            </div>
            <div className="text-emerald-700 dark:text-emerald-400 text-[11px]">
              Status Cloud:{" "}
              {backupSuccess.cloud_status === "SYNCED_TO_GDRIVE"
                ? "Tersinkronisasi ke Google Drive"
                : "Tersimpan di Penyimpanan Lokal"}
              {backupSuccess.deleted_old_backups.length > 0 &&
                ` • ${backupSuccess.deleted_old_backups.length} snapshot kadaluarsa dibersihkan per kebijakan retensi 10 item.`}
            </div>
          </div>
        </div>
      )}

      {/* Table of Backups */}
      <div className="space-y-3">
        <h3 className="font-bold text-slate-900 dark:text-white text-sm">
          Riwayat Snapshot Backup Lokal ({backups.length} / 10 item)
        </h3>

        <div className="overflow-hidden rounded-2xl border border-slate-200/80 bg-white shadow-sm dark:border-slate-800 dark:bg-slate-900">
          <div className="overflow-x-auto">
            <table className="w-full text-left text-xs">
              <thead className="bg-slate-50/80 text-[11px] font-semibold uppercase tracking-wider text-slate-400 dark:bg-slate-800/50">
                <tr>
                  <th className="px-4 py-3">Nama Snapshot</th>
                  <th className="px-4 py-3">Waktu Pembuatan</th>
                  <th className="px-4 py-3 text-right">Ukuran Dual DB</th>
                  <th className="px-4 py-3 text-center">Status</th>
                  <th className="px-4 py-3 text-center">Aksi</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
                {backups.length === 0 ? (
                  <tr>
                    <td colSpan={5} className="py-8 text-center text-slate-400">
                      Belum ada snapshot backup tersimpan. Klik tombol di atas untuk membuat.
                    </td>
                  </tr>
                ) : (
                  backups.map((b) => (
                    <tr
                      key={b.backup_id}
                      className="hover:bg-slate-50/50 dark:hover:bg-slate-800/30 transition-colors"
                    >
                      <td className="px-4 py-3 font-mono font-semibold text-slate-900 dark:text-white">
                        <div className="flex items-center gap-2">
                          <FolderArchive className="h-4 w-4 text-indigo-500 shrink-0" />
                          <span>{b.folder_name}</span>
                        </div>
                      </td>
                      <td className="px-4 py-3 font-mono text-slate-500 text-[11px]">
                        {b.created_at !== "-" ? new Date(b.created_at).toLocaleString("id-ID") : "-"}
                      </td>
                      <td className="px-4 py-3 text-right font-mono font-bold text-slate-700 dark:text-slate-300">
                        {b.total_size_formatted}
                      </td>
                      <td className="px-4 py-3 text-center">
                        <span className="inline-flex items-center gap-1 rounded-full bg-slate-100 px-2.5 py-0.5 text-[10px] font-bold text-slate-600 dark:bg-slate-800 dark:text-slate-300">
                          <CheckCircle2 className="h-3 w-3 text-emerald-500" /> Tersimpan
                        </span>
                      </td>
                      <td className="px-4 py-3 text-center">
                        <button
                          onClick={() => openRestoreModal(b)}
                          className="inline-flex items-center gap-1 rounded-xl bg-rose-50 px-3 py-1 text-xs font-semibold text-rose-600 hover:bg-rose-100 dark:bg-rose-950/60 dark:text-rose-400 dark:hover:bg-rose-900"
                        >
                          <RotateCcw className="h-3.5 w-3.5" />
                          Pulihkan (Restore)
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

      {/* Modal Konfirmasi Pemulihan (Restore) */}
      {restoreModalOpen && selectedBackup && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/60 backdrop-blur-sm p-4">
          <div className="w-full max-w-md rounded-2xl bg-white p-6 shadow-2xl dark:bg-slate-900 border border-slate-100 dark:border-slate-800 text-xs space-y-4">
            <div className="flex items-center justify-between border-b border-slate-100 pb-3 dark:border-slate-800">
              <div className="flex items-center gap-2 text-rose-600">
                <AlertTriangle className="h-5 w-5" />
                <h3 className="font-bold text-slate-900 dark:text-white text-base">
                  Konfirmasi Pemulihan Database
                </h3>
              </div>
              <button
                onClick={() => setRestoreModalOpen(false)}
                className="text-slate-400 hover:text-slate-600"
              >
                <X className="h-5 w-5" />
              </button>
            </div>

            <div className="rounded-xl border border-rose-200 bg-rose-50/70 p-3.5 text-rose-800 dark:border-rose-900/60 dark:bg-rose-950/40 dark:text-rose-300 space-y-1">
              <span className="font-bold block">Peringatan Kritis!</span>
              <p className="leading-relaxed text-[11px]">
                Tindakan ini akan menimpa seluruh data operasional dan data log saat ini dengan isi
                snapshot <span className="font-mono font-bold">{selectedBackup.folder_name}</span>.
                Data transaksi yang dibuat setelah snapshot ini akan hilang.
              </p>
            </div>

            <div className="rounded-xl bg-slate-50 p-3 dark:bg-slate-800/60 space-y-1">
              <div className="flex justify-between">
                <span className="text-slate-500">Snapshot:</span>
                <span className="font-mono font-bold text-slate-800 dark:text-slate-200">
                  {selectedBackup.folder_name}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-slate-500">Ukuran:</span>
                <span className="font-mono">{selectedBackup.total_size_formatted}</span>
              </div>
            </div>

            <div className="flex justify-end gap-2 pt-2">
              <button
                type="button"
                onClick={() => setRestoreModalOpen(false)}
                className="rounded-xl border border-slate-200 px-4 py-2 font-semibold text-slate-600 hover:bg-slate-50 dark:border-slate-700 dark:text-slate-300"
              >
                Batal
              </button>
              <button
                type="button"
                onClick={handleConfirmRestore}
                disabled={restoring}
                className="rounded-xl bg-rose-600 px-5 py-2 font-bold text-white shadow-md hover:bg-rose-700 disabled:opacity-50"
              >
                {restoring ? "Memulihkan..." : "Ya, Pulihkan Sekarang"}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
