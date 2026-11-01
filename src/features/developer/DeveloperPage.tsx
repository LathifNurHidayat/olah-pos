import React, { useState, useEffect } from "react";
import {
  Terminal,
  Activity,
  Users,
  Database,
  Download,
  RefreshCw,
  Search,
  CheckCircle2,
  AlertTriangle,
  XCircle,
  Eye,
  Copy,
  Check,
  UserPlus,
  KeyRound,
  ShieldCheck,
  UserX,
  UserCheck,
  X,
  Play,
} from "lucide-react";
import type { UserSession } from "@/features/auth/LoginPage";

interface SystemDiagnostics {
  app_name: String;
  app_version: String;
  os_info: String;
  main_db_path: String;
  main_db_size_bytes: number;
  log_db_path: String;
  log_db_size_bytes: number;
  sqlite_version: String;
  journal_mode: String;
  dual_db_attached: boolean;
  db_integrity_status: String;
  encryption_key_status: String;
  uptime_seconds: number;
  total_audit_tables_count: number;
  total_audit_records_count: number;
}

interface AuditLogRow {
  log_id: number;
  table_name: string;
  created_at: string;
  created_by: number | null;
  primary_key_id: string;
  aksi: string;
  content: Record<string, unknown> | string;
}

interface RuntimeLogEntry {
  id: number;
  timestamp: string;
  level: string;
  source: string;
  message: string;
}

interface UserAdminItem {
  pengguna_id: number;
  nama_lengkap: string;
  nama_pengguna: string;
  peran: string;
  active_st: number;
  harus_ganti_password: boolean;
  terakhir_login: string | null;
  created_at: string;
}

interface QueryResult {
  columns: string[];
  rows: (string | number | boolean | null)[][];
  row_count: number;
  execution_time_ms: number;
}

async function getTauriInvoke() {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke;
}

export const DeveloperPage: React.FC<{ currentUser?: UserSession }> = ({ currentUser }) => {
  const [activeTab, setActiveTab] = useState<"audit" | "diagnostics" | "users" | "sql">("audit");
  const [loading, setLoading] = useState(false);
  const [toastMessage, setToastMessage] = useState<string | null>(null);

  // Tab 1: Audit Logs State
  const [auditTables, setAuditTables] = useState<string[]>([]);
  const [selectedTable, setSelectedTable] = useState<string>("");
  const [actionFilter, setActionFilter] = useState<string>("ALL");
  const [searchAudit, setSearchAudit] = useState<string>("");
  const [auditLogs, setAuditLogs] = useState<AuditLogRow[]>([]);
  const [selectedPayload, setSelectedPayload] = useState<Record<string, unknown> | null>(null);
  const [copiedPayload, setCopiedPayload] = useState(false);

  // Tab 2: Diagnostics State
  const [diagnostics, setDiagnostics] = useState<SystemDiagnostics | null>(null);
  const [runtimeLogs, setRuntimeLogs] = useState<RuntimeLogEntry[]>([]);
  const [logLevelFilter, setLogLevelFilter] = useState<string>("ALL");

  // Tab 3: User Management State
  const [users, setUsers] = useState<UserAdminItem[]>([]);
  const [userModalOpen, setUserModalOpen] = useState(false);
  const [resetPassModalOpen, setResetPassModalOpen] = useState(false);
  const [selectedUserId, setSelectedUserId] = useState<number | null>(null);
  const [newPasswordInput, setNewPasswordInput] = useState("");
  const [newUserForm, setNewUserForm] = useState({
    nama_lengkap: "",
    nama_pengguna: "",
    password: "",
    peran: "CASHIER",
  });

  // Tab 4: SQL Inspector State
  const [sqlQuery, setSqlQuery] = useState("SELECT * FROM mst_barang LIMIT 10;");
  const [queryResult, setQueryResult] = useState<QueryResult | null>(null);
  const [queryError, setQueryError] = useState<string | null>(null);
  const [runningQuery, setRunningQuery] = useState(false);

  const showToast = (msg: string) => {
    setToastMessage(msg);
    setTimeout(() => setToastMessage(null), 3500);
  };

  // Helper download file via browser blob
  const triggerDownload = (content: string, filename: string, mimeType = "text/plain;charset=utf-8") => {
    const blob = new Blob([content], { type: mimeType });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = filename;
    a.click();
    URL.revokeObjectURL(url);
    showToast(`File ${filename} berhasil diunduh!`);
  };

  // 1. Fetch Audit Tables
  const loadAuditTables = async () => {
    try {
      const invoke = await getTauriInvoke();
      const tables = await invoke<string[]>("get_audit_tables");
      setAuditTables(tables);
      if (tables.length > 0 && !selectedTable) {
        setSelectedTable(tables[0]);
      }
    } catch (e) {
      console.error("Gagal memuat tabel audit:", e);
    }
  };

  // 2. Fetch Audit Logs
  const loadAuditLogs = async () => {
    if (!selectedTable) return;
    setLoading(true);
    try {
      const invoke = await getTauriInvoke();
      const logs = await invoke<AuditLogRow[]>("get_developer_audit_logs", {
        tableName: selectedTable,
        limit: 100,
        action: actionFilter === "ALL" ? null : actionFilter,
        search: searchAudit.trim() ? searchAudit.trim() : null,
      });
      setAuditLogs(logs);
    } catch (e) {
      console.error("Gagal memuat log audit:", e);
    } finally {
      setLoading(false);
    }
  };

  // 3. Fetch Diagnostics & Runtime Logs
  const loadDiagnostics = async () => {
    setLoading(true);
    try {
      const invoke = await getTauriInvoke();
      const diag = await invoke<SystemDiagnostics>("get_system_diagnostics");
      setDiagnostics(diag);
      const rLogs = await invoke<RuntimeLogEntry[]>("get_runtime_logs", {
        limit: 200,
        level: logLevelFilter === "ALL" ? null : logLevelFilter,
      });
      setRuntimeLogs(rLogs);
    } catch (e) {
      console.error("Gagal memuat diagnostik:", e);
    } finally {
      setLoading(false);
    }
  };

  // 4. Fetch Users
  const loadUsers = async () => {
    setLoading(true);
    try {
      const invoke = await getTauriInvoke();
      const userList = await invoke<UserAdminItem[]>("developer_list_users");
      setUsers(userList);
    } catch (e) {
      console.error("Gagal memuat pengguna:", e);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadAuditTables();
    loadDiagnostics();
    loadUsers();
  }, []);

  useEffect(() => {
    if (selectedTable) {
      loadAuditLogs();
    }
  }, [selectedTable, actionFilter]);

  // Handle Download Audit Log
  const handleDownloadAuditJson = () => {
    if (auditLogs.length === 0) {
      showToast("Tidak ada data log untuk diunduh");
      return;
    }
    const timestamp = new Date().toISOString().replace(/[:.]/g, "-");
    const filename = `audit_log_${selectedTable}_${timestamp}.json`;
    triggerDownload(JSON.stringify(auditLogs, null, 2), filename, "application/json");
  };

  // Handle Download System Logs
  const handleDownloadSystemLogs = async () => {
    try {
      const invoke = await getTauriInvoke();
      const exportText = await invoke<string>("export_system_logs");
      const timestamp = new Date().toISOString().replace(/[:.]/g, "-");
      const filename = `olahpos_system_log_${timestamp}.log`;
      triggerDownload(exportText, filename);
    } catch (e) {
      showToast(`Gagal mengunduh log sistem: ${e}`);
    }
  };

  // Handle Trigger Test Error
  const handleTriggerTestError = async () => {
    try {
      const invoke = await getTauriInvoke();
      await invoke("record_client_error", {
        level: "ERROR",
        source: "DeveloperConsole",
        message: `Uji coba simulasi error developer pada ${new Date().toLocaleTimeString()} WIB`,
      });
      showToast("Simulasi error berhasil dicatat ke runtime log!");
      loadDiagnostics();
    } catch (e) {
      showToast(`Gagal mencatat error: ${e}`);
    }
  };

  // Handle Create User
  const handleCreateUser = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!currentUser) return;
    try {
      const invoke = await getTauriInvoke();
      await invoke("developer_create_user", {
        input: newUserForm,
        adminId: currentUser.pengguna_id,
      });
      showToast(`Pengguna '${newUserForm.nama_pengguna}' berhasil dibuat!`);
      setUserModalOpen(false);
      setNewUserForm({
        nama_lengkap: "",
        nama_pengguna: "",
        password: "",
        peran: "CASHIER",
      });
      loadUsers();
    } catch (err) {
      showToast(`Gagal membuat pengguna: ${err}`);
    }
  };

  // Handle Reset Password
  const handleResetPassword = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!currentUser || selectedUserId === null) return;
    try {
      const invoke = await getTauriInvoke();
      await invoke("developer_reset_password", {
        userId: selectedUserId,
        newPassword: newPasswordInput,
        adminId: currentUser.pengguna_id,
      });
      showToast("Kata sandi berhasil direset! Pengguna wajib mengganti password saat login berikutnya.");
      setResetPassModalOpen(false);
      setNewPasswordInput("");
      loadUsers();
    } catch (err) {
      showToast(`Gagal reset password: ${err}`);
    }
  };

  // Handle Toggle Status
  const handleToggleStatus = async (userId: number, currentStatus: number) => {
    if (!currentUser) return;
    const newStatus = currentStatus === 1 ? 0 : 1;
    try {
      const invoke = await getTauriInvoke();
      await invoke("developer_toggle_user_status", {
        userId,
        activeSt: newStatus,
        adminId: currentUser.pengguna_id,
      });
      showToast(`Status pengguna berhasil diubah!`);
      loadUsers();
    } catch (err) {
      showToast(`Gagal ubah status: ${err}`);
    }
  };

  // Handle Update Role
  const handleUpdateRole = async (userId: number, newRole: string) => {
    if (!currentUser) return;
    try {
      const invoke = await getTauriInvoke();
      await invoke("developer_update_user_role", {
        userId,
        peran: newRole,
        adminId: currentUser.pengguna_id,
      });
      showToast(`Peran pengguna diubah menjadi ${newRole}`);
      loadUsers();
    } catch (err) {
      showToast(`Gagal ubah peran: ${err}`);
    }
  };

  // Handle Execute SQL Query
  const handleExecuteSql = async () => {
    if (!sqlQuery.trim()) return;
    setRunningQuery(true);
    setQueryError(null);
    try {
      const invoke = await getTauriInvoke();
      const res = await invoke<QueryResult>("execute_developer_query", {
        sql: sqlQuery,
      });
      setQueryResult(res);
    } catch (err) {
      setQueryError(String(err));
      setQueryResult(null);
    } finally {
      setRunningQuery(false);
    }
  };

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
    setCopiedPayload(true);
    setTimeout(() => setCopiedPayload(false), 2000);
  };

  return (
    <div className="flex h-full flex-col space-y-4 overflow-y-auto p-6 bg-slate-50 dark:bg-slate-950">
      {/* Toast Notification */}
      {toastMessage && (
        <div className="fixed bottom-6 right-6 z-50 flex items-center gap-2 rounded-xl bg-slate-900 px-4 py-3 text-sm text-white shadow-xl dark:bg-slate-800">
          <CheckCircle2 className="h-5 w-5 text-emerald-400" />
          <span>{toastMessage}</span>
        </div>
      )}

      {/* Header */}
      <div className="flex flex-col gap-2 md:flex-row md:items-center md:justify-between">
        <div>
          <div className="flex items-center gap-2">
            <span className="rounded-lg bg-indigo-500/10 px-2.5 py-1 text-xs font-bold uppercase tracking-wider text-indigo-600 dark:text-indigo-400">
              Khusus Developer
            </span>
            <span className="flex items-center gap-1.5 text-xs font-semibold text-emerald-600 dark:text-emerald-400">
              <span className="h-2 w-2 rounded-full bg-emerald-500 animate-pulse"></span>
              Live Diagnostics
            </span>
          </div>
          <h1 className="text-2xl font-black tracking-tight text-slate-900 dark:text-slate-100 flex items-center gap-2 mt-1">
            <Terminal className="h-7 w-7 text-indigo-600 dark:text-indigo-400" />
            Menu Khusus Developer
          </h1>
          <p className="text-sm text-slate-500 dark:text-slate-400">
            Pengecekan log data dinamis, pemantauan error sistem runtime, download log, dan manajemen akses user.
          </p>
        </div>

        {/* Global Action Buttons */}
        <div className="flex items-center gap-2">
          <button
            onClick={handleDownloadSystemLogs}
            className="flex items-center gap-2 rounded-xl bg-indigo-600 px-4 py-2.5 text-sm font-bold text-white shadow-md shadow-indigo-600/20 hover:bg-indigo-700 transition"
          >
            <Download className="h-4 w-4" />
            Unduh Log Sistem (.log)
          </button>
        </div>
      </div>

      {/* Modern Tabs Navigation */}
      <div className="flex gap-2 border-b border-slate-200 dark:border-slate-800 pb-2">
        <button
          onClick={() => setActiveTab("audit")}
          className={`flex items-center gap-2 rounded-xl px-4 py-2 text-sm font-bold transition-all ${
            activeTab === "audit"
              ? "bg-indigo-600 text-white shadow-sm"
              : "text-slate-600 hover:bg-slate-100 dark:text-slate-400 dark:hover:bg-slate-900"
          }`}
        >
          <Database className="h-4 w-4" />
          Pengecekan Log Data
        </button>

        <button
          onClick={() => {
            setActiveTab("diagnostics");
            loadDiagnostics();
          }}
          className={`flex items-center gap-2 rounded-xl px-4 py-2 text-sm font-bold transition-all ${
            activeTab === "diagnostics"
              ? "bg-indigo-600 text-white shadow-sm"
              : "text-slate-600 hover:bg-slate-100 dark:text-slate-400 dark:hover:bg-slate-900"
          }`}
        >
          <Activity className="h-4 w-4" />
          Debugging Error Sistem
        </button>

        <button
          onClick={() => {
            setActiveTab("users");
            loadUsers();
          }}
          className={`flex items-center gap-2 rounded-xl px-4 py-2 text-sm font-bold transition-all ${
            activeTab === "users"
              ? "bg-indigo-600 text-white shadow-sm"
              : "text-slate-600 hover:bg-slate-100 dark:text-slate-400 dark:hover:bg-slate-900"
          }`}
        >
          <Users className="h-4 w-4" />
          Manajemen Akses User
        </button>

        <button
          onClick={() => setActiveTab("sql")}
          className={`flex items-center gap-2 rounded-xl px-4 py-2 text-sm font-bold transition-all ${
            activeTab === "sql"
              ? "bg-indigo-600 text-white shadow-sm"
              : "text-slate-600 hover:bg-slate-100 dark:text-slate-400 dark:hover:bg-slate-900"
          }`}
        >
          <Terminal className="h-4 w-4" />
          SQL Inspector (Read-Only)
        </button>
      </div>

      {/* ======================================================== */}
      {/* TAB 1: PENGECEKAN LOG DATA (AUDIT EXPLORER) */}
      {/* ======================================================== */}
      {activeTab === "audit" && (
        <div className="space-y-4">
          {/* Controls Bar */}
          <div className="flex flex-wrap items-center justify-between gap-3 rounded-2xl bg-white p-4 shadow-sm border border-slate-200/80 dark:border-slate-800 dark:bg-slate-900">
            <div className="flex flex-wrap items-center gap-3">
              {/* Table Selector */}
              <div className="flex items-center gap-2">
                <span className="text-xs font-bold text-slate-500 uppercase">Tabel Audit:</span>
                <select
                  value={selectedTable}
                  onChange={(e) => setSelectedTable(e.target.value)}
                  className="rounded-xl border border-slate-200 bg-slate-50 px-3 py-1.5 text-sm font-bold text-slate-800 focus:border-indigo-500 focus:outline-none dark:border-slate-700 dark:bg-slate-800 dark:text-slate-100"
                >
                  {auditTables.map((t) => (
                    <option key={t} value={t}>
                      {t}
                    </option>
                  ))}
                </select>
              </div>

              {/* Action Filter */}
              <div className="flex items-center gap-2">
                <span className="text-xs font-bold text-slate-500 uppercase">Aksi:</span>
                <select
                  value={actionFilter}
                  onChange={(e) => setActionFilter(e.target.value)}
                  className="rounded-xl border border-slate-200 bg-slate-50 px-3 py-1.5 text-sm font-bold text-slate-800 focus:border-indigo-500 focus:outline-none dark:border-slate-700 dark:bg-slate-800 dark:text-slate-100"
                >
                  <option value="ALL">Semua Aksi</option>
                  <option value="INSERT">INSERT</option>
                  <option value="UPDATE">UPDATE</option>
                  <option value="DELETE">DELETE</option>
                  <option value="RESTORE">RESTORE</option>
                </select>
              </div>

              {/* Search Bar */}
              <div className="relative">
                <Search className="absolute left-3 top-2.5 h-4 w-4 text-slate-400" />
                <input
                  type="text"
                  placeholder="Cari ID / Konten..."
                  value={searchAudit}
                  onChange={(e) => setSearchAudit(e.target.value)}
                  onKeyDown={(e) => e.key === "Enter" && loadAuditLogs()}
                  className="w-52 rounded-xl border border-slate-200 bg-slate-50 pl-9 pr-3 py-1.5 text-sm focus:border-indigo-500 focus:outline-none dark:border-slate-700 dark:bg-slate-800 dark:text-slate-100"
                />
              </div>

              <button
                onClick={loadAuditLogs}
                disabled={loading}
                className="flex items-center gap-1.5 rounded-xl border border-slate-200 px-3 py-1.5 text-xs font-bold text-slate-700 hover:bg-slate-100 dark:border-slate-700 dark:text-slate-300 dark:hover:bg-slate-800"
              >
                <RefreshCw className={`h-3.5 w-3.5 ${loading ? "animate-spin" : ""}`} />
                Refresh
              </button>
            </div>

            {/* Download JSON Button */}
            <button
              onClick={handleDownloadAuditJson}
              className="flex items-center gap-1.5 rounded-xl bg-slate-100 px-3.5 py-1.5 text-xs font-bold text-slate-700 hover:bg-slate-200 dark:bg-slate-800 dark:text-slate-200 dark:hover:bg-slate-700 transition"
            >
              <Download className="h-3.5 w-3.5 text-indigo-500" />
              Unduh Log Data ({auditLogs.length} baris)
            </button>
          </div>

          {/* Audit Log Table */}
          <div className="overflow-hidden rounded-2xl bg-white shadow-sm border border-slate-200/80 dark:border-slate-800 dark:bg-slate-900">
            <div className="overflow-x-auto">
              <table className="w-full text-left text-sm">
                <thead className="border-b border-slate-200/80 bg-slate-50/70 text-xs font-black uppercase tracking-wider text-slate-500 dark:border-slate-800 dark:bg-slate-800/40">
                  <tr>
                    <th className="px-4 py-3">Log ID</th>
                    <th className="px-4 py-3">Waktu Kejadian</th>
                    <th className="px-4 py-3">Aksi</th>
                    <th className="px-4 py-3">Primary Key</th>
                    <th className="px-4 py-3">User ID</th>
                    <th className="px-4 py-3">Cuplikan Perubahan</th>
                    <th className="px-4 py-3 text-right">Detail</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-slate-100 dark:divide-slate-800/60 font-mono text-xs">
                  {auditLogs.length === 0 ? (
                    <tr>
                      <td colSpan={7} className="px-4 py-8 text-center font-sans text-slate-400">
                        Tidak ada log perubahan yang cocok dengan filter.
                      </td>
                    </tr>
                  ) : (
                    auditLogs.map((log) => {
                      const payloadStr =
                        typeof log.content === "string" ? log.content : JSON.stringify(log.content);

                      return (
                        <tr key={log.log_id} className="hover:bg-slate-50/50 dark:hover:bg-slate-800/30">
                          <td className="px-4 py-3 text-slate-400">#{log.log_id}</td>
                          <td className="px-4 py-3 text-slate-600 dark:text-slate-300">
                            {log.created_at.replace("T", " ").substring(0, 19)}
                          </td>
                          <td className="px-4 py-3 font-sans">
                            <span
                              className={`inline-block rounded-md px-2 py-0.5 text-[10px] font-black uppercase ${
                                log.aksi === "INSERT"
                                  ? "bg-emerald-100 text-emerald-800 dark:bg-emerald-950 dark:text-emerald-300"
                                  : log.aksi === "UPDATE"
                                  ? "bg-blue-100 text-blue-800 dark:bg-blue-950 dark:text-blue-300"
                                  : log.aksi === "DELETE"
                                  ? "bg-rose-100 text-rose-800 dark:bg-rose-950 dark:text-rose-300"
                                  : "bg-amber-100 text-amber-800 dark:bg-amber-950 dark:text-amber-300"
                              }`}
                            >
                              {log.aksi}
                            </span>
                          </td>
                          <td className="px-4 py-3 font-bold text-slate-900 dark:text-slate-100">
                            {log.primary_key_id}
                          </td>
                          <td className="px-4 py-3 text-slate-500">
                            {log.created_by ? `User #${log.created_by}` : "System / Bootstrap"}
                          </td>
                          <td className="px-4 py-3 max-w-xs truncate text-slate-500 dark:text-slate-400" title={payloadStr}>
                            {payloadStr}
                          </td>
                          <td className="px-4 py-3 text-right">
                            <button
                              onClick={() => {
                                const parsed =
                                  typeof log.content === "string" ? JSON.parse(log.content) : log.content;
                                setSelectedPayload(parsed);
                              }}
                              className="rounded-lg p-1.5 text-indigo-600 hover:bg-indigo-50 dark:text-indigo-400 dark:hover:bg-indigo-950/40"
                              title="Lihat Payload Lengkap"
                            >
                              <Eye className="h-4 w-4" />
                            </button>
                          </td>
                        </tr>
                      );
                    })
                  )}
                </tbody>
              </table>
            </div>
          </div>
        </div>
      )}

      {/* ======================================================== */}
      {/* TAB 2: DEBUGGING ERROR SISTEM */}
      {/* ======================================================== */}
      {activeTab === "diagnostics" && (
        <div className="space-y-4">
          {/* Diagnostic Health Cards */}
          <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-4">
            {/* Card 1: SQLite Integrity */}
            <div className="rounded-2xl bg-white p-4 shadow-sm border border-slate-200/80 dark:border-slate-800 dark:bg-slate-900">
              <div className="flex items-center justify-between">
                <span className="text-xs font-bold uppercase text-slate-500">SQLite Integrity</span>
                {diagnostics?.db_integrity_status === "ok" ? (
                  <CheckCircle2 className="h-5 w-5 text-emerald-500" />
                ) : (
                  <XCircle className="h-5 w-5 text-rose-500" />
                )}
              </div>
              <div className="mt-2 text-xl font-black text-slate-900 dark:text-slate-100">
                {diagnostics?.db_integrity_status || "Checking..."}
              </div>
              <p className="mt-1 text-xs text-slate-400">
                PRAGMA integrity_check • Mode: {diagnostics?.journal_mode || "WAL"}
              </p>
            </div>

            {/* Card 2: Dual DB ATTACH Status */}
            <div className="rounded-2xl bg-white p-4 shadow-sm border border-slate-200/80 dark:border-slate-800 dark:bg-slate-900">
              <div className="flex items-center justify-between">
                <span className="text-xs font-bold uppercase text-slate-500">Dual Database Attach</span>
                {diagnostics?.dual_db_attached ? (
                  <CheckCircle2 className="h-5 w-5 text-emerald-500" />
                ) : (
                  <AlertTriangle className="h-5 w-5 text-amber-500" />
                )}
              </div>
              <div className="mt-2 text-xl font-black text-slate-900 dark:text-slate-100">
                {diagnostics?.dual_db_attached ? "TERHUBUNG (log_db)" : "TIDAK TERPASANG"}
              </div>
              <p className="mt-1 text-xs text-slate-400">
                {diagnostics?.total_audit_tables_count || 0} tabel audit terdeteksi
              </p>
            </div>

            {/* Card 3: Symmetric Encryption Key */}
            <div className="rounded-2xl bg-white p-4 shadow-sm border border-slate-200/80 dark:border-slate-800 dark:bg-slate-900">
              <div className="flex items-center justify-between">
                <span className="text-xs font-bold uppercase text-slate-500">Enkripsi Simetris</span>
                <ShieldCheck className="h-5 w-5 text-indigo-500" />
              </div>
              <div className="mt-2 text-xl font-black text-slate-900 dark:text-slate-100">
                {diagnostics?.encryption_key_status === "VALID_AES256_GCM" ? "AES-256-GCM OK" : "ERROR"}
              </div>
              <p className="mt-1 text-xs text-slate-400">Kunci enkripsi config.json terverifikasi</p>
            </div>

            {/* Card 4: Database Sizes */}
            <div className="rounded-2xl bg-white p-4 shadow-sm border border-slate-200/80 dark:border-slate-800 dark:bg-slate-900">
              <div className="flex items-center justify-between">
                <span className="text-xs font-bold uppercase text-slate-500">Total Audit Records</span>
                <Database className="h-5 w-5 text-violet-500" />
              </div>
              <div className="mt-2 text-xl font-black text-slate-900 dark:text-slate-100">
                {diagnostics?.total_audit_records_count || 0} Records
              </div>
              <p className="mt-1 text-xs text-slate-400">
                Main: {((diagnostics?.main_db_size_bytes || 0) / 1024).toFixed(1)} KB • Log:{" "}
                {((diagnostics?.log_db_size_bytes || 0) / 1024).toFixed(1)} KB
              </p>
            </div>
          </div>

          {/* Runtime Logging Stream */}
          <div className="rounded-2xl bg-slate-900 text-slate-100 p-4 shadow-md border border-slate-800 font-mono">
            {/* Log Action Header */}
            <div className="flex flex-wrap items-center justify-between gap-3 border-b border-slate-800 pb-3 mb-3">
              <div className="flex items-center gap-3">
                <span className="text-xs font-bold text-slate-400 uppercase">Streaming Log Sistem:</span>
                <select
                  value={logLevelFilter}
                  onChange={(e) => setLogLevelFilter(e.target.value)}
                  className="rounded-lg bg-slate-800 border border-slate-700 px-2.5 py-1 text-xs font-bold text-slate-200 focus:outline-none"
                >
                  <option value="ALL">Semua Level</option>
                  <option value="ERROR">ERROR Saja</option>
                  <option value="WARN">WARN Saja</option>
                  <option value="INFO">INFO Saja</option>
                </select>

                <button
                  onClick={loadDiagnostics}
                  className="flex items-center gap-1 rounded-lg bg-slate-800 px-2.5 py-1 text-xs font-bold text-slate-300 hover:bg-slate-700"
                >
                  <RefreshCw className="h-3 w-3" />
                  Refresh
                </button>
              </div>

              <div className="flex items-center gap-2">
                <button
                  onClick={handleTriggerTestError}
                  className="flex items-center gap-1.5 rounded-lg bg-rose-950/80 border border-rose-800 px-3 py-1 text-xs font-bold text-rose-300 hover:bg-rose-900 transition"
                >
                  <AlertTriangle className="h-3.5 w-3.5 text-rose-400" />
                  Trigger Test Error
                </button>

                <button
                  onClick={handleDownloadSystemLogs}
                  className="flex items-center gap-1.5 rounded-lg bg-indigo-600 px-3 py-1 text-xs font-bold text-white hover:bg-indigo-500 transition"
                >
                  <Download className="h-3.5 w-3.5" />
                  Download .log
                </button>
              </div>
            </div>

            {/* Log Stream Output */}
            <div className="h-96 overflow-y-auto space-y-1.5 text-xs">
              {runtimeLogs.length === 0 ? (
                <div className="py-8 text-center text-slate-500">
                  Belum ada catatan runtime error atau peringatan pada sesi ini.
                </div>
              ) : (
                runtimeLogs.map((log) => (
                  <div
                    key={log.id}
                    className={`flex items-start gap-2 rounded px-2 py-1 ${
                      log.level === "ERROR"
                        ? "bg-rose-950/40 text-rose-300 border-l-2 border-rose-500"
                        : log.level === "WARN"
                        ? "bg-amber-950/30 text-amber-300 border-l-2 border-amber-500"
                        : "text-slate-300 border-l-2 border-slate-700"
                    }`}
                  >
                    <span className="text-slate-500 shrink-0">
                      [{log.timestamp.replace("T", " ").substring(11, 19)}]
                    </span>
                    <span
                      className={`font-black shrink-0 ${
                        log.level === "ERROR"
                          ? "text-rose-400"
                          : log.level === "WARN"
                          ? "text-amber-400"
                          : "text-emerald-400"
                      }`}
                    >
                      [{log.level}]
                    </span>
                    <span className="text-slate-400 font-bold shrink-0">[{log.source}]</span>
                    <span className="break-all">{log.message}</span>
                  </div>
                ))
              )}
            </div>
          </div>
        </div>
      )}

      {/* ======================================================== */}
      {/* TAB 3: MANAJEMEN AKSES USER */}
      {/* ======================================================== */}
      {activeTab === "users" && (
        <div className="space-y-4">
          {/* Header Action Bar */}
          <div className="flex items-center justify-between rounded-2xl bg-white p-4 shadow-sm border border-slate-200/80 dark:border-slate-800 dark:bg-slate-900">
            <div>
              <h3 className="text-base font-black text-slate-900 dark:text-slate-100">
                Daftar Pengguna Sistem & Hak Akses
              </h3>
              <p className="text-xs text-slate-500 dark:text-slate-400">
                Developer dapat menambah akun kasir/admin, mereset kata sandi, dan menonaktifkan pengguna.
              </p>
            </div>
            <button
              onClick={() => setUserModalOpen(true)}
              className="flex items-center gap-2 rounded-xl bg-indigo-600 px-4 py-2 text-sm font-bold text-white shadow-sm hover:bg-indigo-700 transition"
            >
              <UserPlus className="h-4 w-4" />
              + Tambah Pengguna Baru
            </button>
          </div>

          {/* User Table */}
          <div className="overflow-hidden rounded-2xl bg-white shadow-sm border border-slate-200/80 dark:border-slate-800 dark:bg-slate-900">
            <table className="w-full text-left text-sm">
              <thead className="border-b border-slate-200/80 bg-slate-50/70 text-xs font-black uppercase tracking-wider text-slate-500 dark:border-slate-800 dark:bg-slate-800/40">
                <tr>
                  <th className="px-4 py-3">ID</th>
                  <th className="px-4 py-3">Nama Lengkap</th>
                  <th className="px-4 py-3">Username</th>
                  <th className="px-4 py-3">Peran Akses</th>
                  <th className="px-4 py-3">Status</th>
                  <th className="px-4 py-3">Terakhir Login</th>
                  <th className="px-4 py-3 text-right">Tindakan Developer</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-100 dark:divide-slate-800/60">
                {users.map((u) => (
                  <tr key={u.pengguna_id} className="hover:bg-slate-50/50 dark:hover:bg-slate-800/30">
                    <td className="px-4 py-3 font-mono text-xs text-slate-400">#{u.pengguna_id}</td>
                    <td className="px-4 py-3 font-bold text-slate-900 dark:text-slate-100">
                      {u.nama_lengkap}
                    </td>
                    <td className="px-4 py-3 font-mono text-xs text-indigo-600 dark:text-indigo-400 font-bold">
                      {u.nama_pengguna}
                    </td>
                    <td className="px-4 py-3">
                      <select
                        value={u.peran}
                        onChange={(e) => handleUpdateRole(u.pengguna_id, e.target.value)}
                        className={`rounded-lg px-2.5 py-1 text-xs font-black uppercase border border-transparent focus:border-indigo-500 focus:outline-none ${
                          u.peran === "ADMIN"
                            ? "bg-purple-100 text-purple-800 dark:bg-purple-950 dark:text-purple-300"
                            : "bg-blue-100 text-blue-800 dark:bg-blue-950 dark:text-blue-300"
                        }`}
                      >
                        <option value="ADMIN">ADMIN</option>
                        <option value="CASHIER">CASHIER</option>
                      </select>
                    </td>
                    <td className="px-4 py-3">
                      <span
                        className={`inline-flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-xs font-bold ${
                          u.active_st === 1
                            ? "bg-emerald-100 text-emerald-800 dark:bg-emerald-950 dark:text-emerald-300"
                            : "bg-rose-100 text-rose-800 dark:bg-rose-950 dark:text-rose-300"
                        }`}
                      >
                        <span
                          className={`h-1.5 w-1.5 rounded-full ${
                            u.active_st === 1 ? "bg-emerald-500" : "bg-rose-500"
                          }`}
                        />
                        {u.active_st === 1 ? "Aktif" : "Nonaktif"}
                      </span>
                    </td>
                    <td className="px-4 py-3 text-xs text-slate-500">
                      {u.terakhir_login ? u.terakhir_login.replace("T", " ").substring(0, 19) : "Belum pernah login"}
                    </td>
                    <td className="px-4 py-3 text-right">
                      <div className="flex items-center justify-end gap-1.5">
                        <button
                          onClick={() => {
                            setSelectedUserId(u.pengguna_id);
                            setResetPassModalOpen(true);
                          }}
                          className="flex items-center gap-1 rounded-lg px-2.5 py-1 text-xs font-bold text-amber-700 bg-amber-50 hover:bg-amber-100 dark:bg-amber-950/40 dark:text-amber-400"
                          title="Reset Password Pengguna"
                        >
                          <KeyRound className="h-3.5 w-3.5" />
                          Reset Pass
                        </button>

                        <button
                          onClick={() => handleToggleStatus(u.pengguna_id, u.active_st)}
                          disabled={currentUser?.pengguna_id === u.pengguna_id}
                          className={`flex items-center gap-1 rounded-lg px-2.5 py-1 text-xs font-bold transition ${
                            u.active_st === 1
                              ? "text-rose-700 bg-rose-50 hover:bg-rose-100 dark:bg-rose-950/40 dark:text-rose-400"
                              : "text-emerald-700 bg-emerald-50 hover:bg-emerald-100 dark:bg-emerald-950/40 dark:text-emerald-400"
                          } disabled:opacity-30 disabled:cursor-not-allowed`}
                          title={u.active_st === 1 ? "Nonaktifkan Akun" : "Aktifkan Akun"}
                        >
                          {u.active_st === 1 ? (
                            <>
                              <UserX className="h-3.5 w-3.5" />
                              Nonaktifkan
                            </>
                          ) : (
                            <>
                              <UserCheck className="h-3.5 w-3.5" />
                              Aktifkan
                            </>
                          )}
                        </button>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {/* ======================================================== */}
      {/* TAB 4: SQL INSPECTOR (READ-ONLY) */}
      {/* ======================================================== */}
      {activeTab === "sql" && (
        <div className="space-y-4">
          <div className="rounded-2xl bg-white p-4 shadow-sm border border-slate-200/80 dark:border-slate-800 dark:bg-slate-900 space-y-3">
            <div className="flex items-center justify-between">
              <span className="text-xs font-black uppercase text-slate-500">
                Konsol SQL Read-Only (Hanya SELECT, PRAGMA, EXPLAIN)
              </span>
              <div className="flex items-center gap-1.5 text-xs text-slate-400">
                <span>Preset Cepat:</span>
                <button
                  onClick={() => setSqlQuery("SELECT * FROM mst_barang LIMIT 10;")}
                  className="rounded bg-slate-100 px-2 py-0.5 hover:bg-slate-200 dark:bg-slate-800 text-slate-700 dark:text-slate-300"
                >
                  Barang
                </button>
                <button
                  onClick={() => setSqlQuery("SELECT * FROM dat_penjualan ORDER BY penjualan_id DESC LIMIT 10;")}
                  className="rounded bg-slate-100 px-2 py-0.5 hover:bg-slate-200 dark:bg-slate-800 text-slate-700 dark:text-slate-300"
                >
                  Penjualan
                </button>
                <button
                  onClick={() => setSqlQuery("PRAGMA integrity_check;")}
                  className="rounded bg-slate-100 px-2 py-0.5 hover:bg-slate-200 dark:bg-slate-800 text-slate-700 dark:text-slate-300"
                >
                  Integrity
                </button>
                <button
                  onClick={() => setSqlQuery("SELECT name FROM log_db.sqlite_master WHERE type='table';")}
                  className="rounded bg-slate-100 px-2 py-0.5 hover:bg-slate-200 dark:bg-slate-800 text-slate-700 dark:text-slate-300"
                >
                  Log Tables
                </button>
              </div>
            </div>

            <textarea
              rows={3}
              value={sqlQuery}
              onChange={(e) => setSqlQuery(e.target.value)}
              className="w-full rounded-xl border border-slate-200 bg-slate-50 p-3 font-mono text-xs text-slate-800 focus:border-indigo-500 focus:outline-none dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100"
              placeholder="Ketik query SQL di sini..."
            />

            <div className="flex items-center justify-between">
              <span className="text-xs text-slate-400">
                Maksimal 200 baris ditampilkan demi performa antarmuka.
              </span>
              <button
                onClick={handleExecuteSql}
                disabled={runningQuery}
                className="flex items-center gap-2 rounded-xl bg-indigo-600 px-4 py-2 text-xs font-bold text-white shadow hover:bg-indigo-700 disabled:opacity-50"
              >
                <Play className="h-3.5 w-3.5" />
                {runningQuery ? "Menjalankan..." : "Eksekusi Query"}
              </button>
            </div>
          </div>

          {queryError && (
            <div className="rounded-2xl bg-rose-50 p-4 border border-rose-200 text-rose-800 text-xs font-mono dark:bg-rose-950/40 dark:border-rose-900 dark:text-rose-300">
              {queryError}
            </div>
          )}

          {queryResult && (
            <div className="rounded-2xl bg-white shadow-sm border border-slate-200/80 dark:border-slate-800 dark:bg-slate-900 p-4 space-y-3">
              <div className="flex items-center justify-between text-xs text-slate-500 font-bold border-b border-slate-100 dark:border-slate-800 pb-2">
                <span>Hasil Query: {queryResult.row_count} baris</span>
                <span>Waktu Eksekusi: {queryResult.execution_time_ms} ms</span>
              </div>

              <div className="overflow-x-auto max-h-96">
                <table className="w-full text-left font-mono text-xs">
                  <thead className="bg-slate-50 dark:bg-slate-800/60 sticky top-0">
                    <tr>
                      {queryResult.columns.map((col, idx) => (
                        <th key={idx} className="px-3 py-2 font-black text-slate-700 dark:text-slate-300">
                          {col}
                        </th>
                      ))}
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
                    {queryResult.rows.map((row, rIdx) => (
                      <tr key={rIdx} className="hover:bg-slate-50 dark:hover:bg-slate-800/30">
                        {row.map((val, cIdx) => (
                          <td key={cIdx} className="px-3 py-2 text-slate-600 dark:text-slate-400">
                            {val === null ? (
                              <span className="italic text-slate-300">NULL</span>
                            ) : typeof val === "object" ? (
                              JSON.stringify(val)
                            ) : (
                              String(val)
                            )}
                          </td>
                        ))}
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          )}
        </div>
      )}

      {/* ======================================================== */}
      {/* MODAL: PRETTY JSON PAYLOAD VIEWER */}
      {/* ======================================================== */}
      {selectedPayload && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-900/60 backdrop-blur-sm p-4">
          <div className="w-full max-w-2xl rounded-2xl bg-white p-6 shadow-2xl dark:bg-slate-900 border border-slate-200 dark:border-slate-800 space-y-4">
            <div className="flex items-center justify-between border-b border-slate-100 pb-3 dark:border-slate-800">
              <h3 className="text-base font-black text-slate-900 dark:text-slate-100 flex items-center gap-2">
                <Database className="h-5 w-5 text-indigo-600" />
                Payload Audit JSON Lengkap
              </h3>
              <button
                onClick={() => setSelectedPayload(null)}
                className="rounded-lg p-1.5 text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800"
              >
                <X className="h-5 w-5" />
              </button>
            </div>

            <div className="relative rounded-xl bg-slate-950 p-4 font-mono text-xs text-emerald-400 max-h-96 overflow-y-auto">
              <button
                onClick={() => copyToClipboard(JSON.stringify(selectedPayload, null, 2))}
                className="absolute top-3 right-3 flex items-center gap-1 rounded bg-slate-800 px-2.5 py-1 text-slate-300 hover:bg-slate-700 text-xs font-bold"
              >
                {copiedPayload ? <Check className="h-3.5 w-3.5 text-emerald-400" /> : <Copy className="h-3.5 w-3.5" />}
                {copiedPayload ? "Tersalin" : "Salin"}
              </button>
              <pre className="whitespace-pre-wrap">{JSON.stringify(selectedPayload, null, 2)}</pre>
            </div>

            <div className="flex justify-end">
              <button
                onClick={() => setSelectedPayload(null)}
                className="rounded-xl bg-slate-100 px-4 py-2 text-xs font-bold text-slate-700 hover:bg-slate-200 dark:bg-slate-800 dark:text-slate-300"
              >
                Tutup
              </button>
            </div>
          </div>
        </div>
      )}

      {/* ======================================================== */}
      {/* MODAL: TAMBAH PENGGUNA BARU */}
      {/* ======================================================== */}
      {userModalOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-900/60 backdrop-blur-sm p-4">
          <div className="w-full max-w-md rounded-2xl bg-white p-6 shadow-2xl dark:bg-slate-900 border border-slate-200 dark:border-slate-800 space-y-4">
            <div className="flex items-center justify-between border-b border-slate-100 pb-3 dark:border-slate-800">
              <h3 className="text-base font-black text-slate-900 dark:text-slate-100 flex items-center gap-2">
                <UserPlus className="h-5 w-5 text-indigo-600" />
                Tambah Pengguna Sistem Baru
              </h3>
              <button
                onClick={() => setUserModalOpen(false)}
                className="rounded-lg p-1.5 text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800"
              >
                <X className="h-5 w-5" />
              </button>
            </div>

            <form onSubmit={handleCreateUser} className="space-y-3">
              <div>
                <label className="text-xs font-bold text-slate-500 uppercase">Nama Lengkap</label>
                <input
                  type="text"
                  required
                  placeholder="Contoh: Budi Santoso"
                  value={newUserForm.nama_lengkap}
                  onChange={(e) => setNewUserForm({ ...newUserForm, nama_lengkap: e.target.value })}
                  className="mt-1 w-full rounded-xl border border-slate-200 bg-slate-50 p-2.5 text-sm font-semibold focus:border-indigo-500 focus:outline-none dark:border-slate-700 dark:bg-slate-800 dark:text-slate-100"
                />
              </div>

              <div>
                <label className="text-xs font-bold text-slate-500 uppercase">Nama Pengguna (Username)</label>
                <input
                  type="text"
                  required
                  placeholder="Contoh: budi_kasir"
                  value={newUserForm.nama_pengguna}
                  onChange={(e) => setNewUserForm({ ...newUserForm, nama_pengguna: e.target.value })}
                  className="mt-1 w-full rounded-xl border border-slate-200 bg-slate-50 p-2.5 text-sm font-semibold focus:border-indigo-500 focus:outline-none dark:border-slate-700 dark:bg-slate-800 dark:text-slate-100"
                />
              </div>

              <div>
                <label className="text-xs font-bold text-slate-500 uppercase">Kata Sandi Awal</label>
                <input
                  type="password"
                  required
                  minLength={6}
                  placeholder="Minimal 6 karakter"
                  value={newUserForm.password}
                  onChange={(e) => setNewUserForm({ ...newUserForm, password: e.target.value })}
                  className="mt-1 w-full rounded-xl border border-slate-200 bg-slate-50 p-2.5 text-sm font-semibold focus:border-indigo-500 focus:outline-none dark:border-slate-700 dark:bg-slate-800 dark:text-slate-100"
                />
              </div>

              <div>
                <label className="text-xs font-bold text-slate-500 uppercase">Peran Akses</label>
                <select
                  value={newUserForm.peran}
                  onChange={(e) => setNewUserForm({ ...newUserForm, peran: e.target.value })}
                  className="mt-1 w-full rounded-xl border border-slate-200 bg-slate-50 p-2.5 text-sm font-bold focus:border-indigo-500 focus:outline-none dark:border-slate-700 dark:bg-slate-800 dark:text-slate-100"
                >
                  <option value="CASHIER">CASHIER (Akses Layar Kasir POS)</option>
                  <option value="ADMIN">ADMIN (Akses Penuh Manajemen Toko)</option>
                </select>
              </div>

              <div className="flex justify-end gap-2 pt-2">
                <button
                  type="button"
                  onClick={() => setUserModalOpen(false)}
                  className="rounded-xl px-4 py-2 text-xs font-bold text-slate-600 hover:bg-slate-100 dark:text-slate-400 dark:hover:bg-slate-800"
                >
                  Batal
                </button>
                <button
                  type="submit"
                  className="rounded-xl bg-indigo-600 px-5 py-2 text-xs font-bold text-white hover:bg-indigo-700 shadow"
                >
                  Simpan Pengguna
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* ======================================================== */}
      {/* MODAL: RESET PASSWORD PENGGUNA */}
      {/* ======================================================== */}
      {resetPassModalOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-900/60 backdrop-blur-sm p-4">
          <div className="w-full max-w-sm rounded-2xl bg-white p-6 shadow-2xl dark:bg-slate-900 border border-slate-200 dark:border-slate-800 space-y-4">
            <div className="flex items-center justify-between border-b border-slate-100 pb-3 dark:border-slate-800">
              <h3 className="text-base font-black text-slate-900 dark:text-slate-100 flex items-center gap-2">
                <KeyRound className="h-5 w-5 text-amber-500" />
                Reset Kata Sandi Pengguna
              </h3>
              <button
                onClick={() => setResetPassModalOpen(false)}
                className="rounded-lg p-1.5 text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800"
              >
                <X className="h-5 w-5" />
              </button>
            </div>

            <form onSubmit={handleResetPassword} className="space-y-3">
              <div>
                <label className="text-xs font-bold text-slate-500 uppercase">Kata Sandi Baru</label>
                <input
                  type="password"
                  required
                  minLength={6}
                  placeholder="Minimal 6 karakter"
                  value={newPasswordInput}
                  onChange={(e) => setNewPasswordInput(e.target.value)}
                  className="mt-1 w-full rounded-xl border border-slate-200 bg-slate-50 p-2.5 text-sm font-semibold focus:border-indigo-500 focus:outline-none dark:border-slate-700 dark:bg-slate-800 dark:text-slate-100"
                />
                <p className="mt-1 text-[11px] text-slate-400">
                  Pengguna akan dipaksa mengganti kata sandi ini saat pertama kali login.
                </p>
              </div>

              <div className="flex justify-end gap-2 pt-2">
                <button
                  type="button"
                  onClick={() => setResetPassModalOpen(false)}
                  className="rounded-xl px-4 py-2 text-xs font-bold text-slate-600 hover:bg-slate-100 dark:text-slate-400 dark:hover:bg-slate-800"
                >
                  Batal
                </button>
                <button
                  type="submit"
                  className="rounded-xl bg-amber-500 px-5 py-2 text-xs font-bold text-white hover:bg-amber-600 shadow"
                >
                  Reset Sekarang
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
};
