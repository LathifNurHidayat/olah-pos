import React, { useState } from "react";
import { ShieldCheck, Lock, User, KeyRound, AlertCircle, CheckCircle2, ArrowRight } from "lucide-react";

export interface UserSession {
  pengguna_id: number;
  nama_lengkap: string;
  nama_pengguna: string;
  peran: string;
  harus_ganti_password: boolean;
  terakhir_login: string | null;
}

interface LoginPageProps {
  onLoginSuccess: (session: UserSession) => void;
}

export const LoginPage: React.FC<LoginPageProps> = ({ onLoginSuccess }) => {
  const [username, setUsername] = useState("developer");
  const [password, setPassword] = useState("developer123");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Change password state for first-time login
  const [pendingSession, setPendingSession] = useState<UserSession | null>(null);
  const [newPassword, setNewPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [changePwError, setChangePwError] = useState<string | null>(null);

  const handleLogin = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setLoading(true);

    try {
      // If running inside Tauri desktop environment
      if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
        const { invoke } = await import("@tauri-apps/api/core");
        const session = await invoke<UserSession>("login", { username, password });
        if (session.harus_ganti_password) {
          setPendingSession(session);
        } else {
          onLoginSuccess(session);
        }
      } else {
        // Mock fallback for browser dev / preview
        if (username === "developer" && password === "developer123") {
          const session: UserSession = {
            pengguna_id: 1,
            nama_lengkap: "Developer Admin",
            nama_pengguna: "developer",
            peran: "ADMIN",
            harus_ganti_password: true,
            terakhir_login: null,
          };
          setPendingSession(session);
        } else {
          throw new Error("Nama pengguna atau kata sandi tidak valid");
        }
      }
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  };

  const handleChangePassword = async (e: React.FormEvent) => {
    e.preventDefault();
    setChangePwError(null);

    if (newPassword.length < 6) {
      setChangePwError("Kata sandi baru minimal 6 karakter.");
      return;
    }

    if (newPassword !== confirmPassword) {
      setChangePwError("Konfirmasi kata sandi tidak cocok.");
      return;
    }

    if (!pendingSession) return;

    setLoading(true);
    try {
      if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
        const { invoke } = await import("@tauri-apps/api/core");
        await invoke("change_password", {
          userId: pendingSession.pengguna_id,
          oldPassword: password,
          newPassword: newPassword,
        });
      }
      onLoginSuccess({
        ...pendingSession,
        harus_ganti_password: false,
      });
    } catch (err: unknown) {
      setChangePwError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="flex min-h-screen w-screen items-center justify-center bg-slate-100 p-4 dark:bg-slate-950">
      <div className="w-full max-w-md overflow-hidden rounded-3xl border border-slate-200/80 bg-white shadow-2xl dark:border-slate-800 dark:bg-slate-900">
        {/* Header Header */}
        <div className="bg-gradient-to-br from-indigo-900 via-indigo-800 to-slate-900 p-8 text-center text-white">
          <div className="mx-auto flex h-14 w-14 items-center justify-center rounded-2xl bg-white/10 shadow-inner backdrop-blur-md">
            <ShieldCheck className="h-8 w-8 text-indigo-300" />
          </div>
          <h1 className="mt-4 text-2xl font-black tracking-tight">
            Olah<span className="text-indigo-400">POS</span>
          </h1>
          <p className="mt-1 text-xs text-indigo-200">
            Sistem Kasir & Inventori Desktop Local-First
          </p>
        </div>

        {/* Form Body */}
        <div className="p-8">
          {!pendingSession ? (
            <form onSubmit={handleLogin} className="space-y-4">
              <div>
                <h2 className="text-base font-bold text-slate-800 dark:text-slate-100">
                  Masuk ke Akun Anda
                </h2>
                <p className="text-xs text-slate-400 mt-0.5">
                  Gunakan kredensial Admin atau Kasir yang terdaftar.
                </p>
              </div>

              {error && (
                <div className="flex items-center gap-2 rounded-xl border border-rose-200 bg-rose-50 p-3 text-xs font-semibold text-rose-700 dark:border-rose-900/50 dark:bg-rose-950/40 dark:text-rose-300">
                  <AlertCircle className="h-4 w-4 shrink-0" />
                  <span>{error}</span>
                </div>
              )}

              <div className="space-y-1.5">
                <label className="text-xs font-semibold text-slate-700 dark:text-slate-300">
                  Nama Pengguna
                </label>
                <div className="flex items-center gap-2 rounded-xl border border-slate-200 bg-slate-50/50 px-3.5 py-2.5 dark:border-slate-800 dark:bg-slate-800/40 focus-within:border-indigo-500 focus-within:ring-2 focus-within:ring-indigo-500/20">
                  <User className="h-4 w-4 text-slate-400" />
                  <input
                    type="text"
                    value={username}
                    onChange={(e) => setUsername(e.target.value)}
                    className="w-full bg-transparent text-sm outline-none placeholder:text-slate-400"
                    placeholder="Contoh: developer / kasir1"
                    required
                  />
                </div>
              </div>

              <div className="space-y-1.5">
                <label className="text-xs font-semibold text-slate-700 dark:text-slate-300">
                  Kata Sandi
                </label>
                <div className="flex items-center gap-2 rounded-xl border border-slate-200 bg-slate-50/50 px-3.5 py-2.5 dark:border-slate-800 dark:bg-slate-800/40 focus-within:border-indigo-500 focus-within:ring-2 focus-within:ring-indigo-500/20">
                  <Lock className="h-4 w-4 text-slate-400" />
                  <input
                    type="password"
                    value={password}
                    onChange={(e) => setPassword(e.target.value)}
                    className="w-full bg-transparent text-sm outline-none placeholder:text-slate-400"
                    placeholder="••••••••"
                    required
                  />
                </div>
              </div>

              <button
                type="submit"
                disabled={loading}
                className="w-full mt-2 flex items-center justify-center gap-2 rounded-xl bg-indigo-600 py-3 text-sm font-bold text-white shadow-md shadow-indigo-600/30 hover:bg-indigo-700 active:scale-[0.98] transition-all disabled:opacity-50"
              >
                {loading ? "Memverifikasi..." : "Masuk ke Sistem"}
                <ArrowRight className="h-4 w-4" />
              </button>

              <div className="rounded-xl border border-indigo-100 bg-indigo-50/60 p-3 text-center dark:border-indigo-950 dark:bg-indigo-950/30">
                <span className="text-[11px] text-slate-500 dark:text-slate-400">
                  Akun Developer Bootstrap:{" "}
                  <strong className="text-indigo-600 dark:text-indigo-400 font-mono">
                    developer / developer123
                  </strong>
                </span>
              </div>
            </form>
          ) : (
            /* First-time login change password form */
            <form onSubmit={handleChangePassword} className="space-y-4">
              <div className="flex items-center gap-2 text-amber-600 dark:text-amber-400">
                <KeyRound className="h-5 w-5 shrink-0" />
                <h2 className="text-sm font-bold">Wajib Ubah Kata Sandi Perdana</h2>
              </div>
              <p className="text-xs text-slate-500">
                Untuk keamanan akun bawaan, Anda diwajibkan mengganti kata sandi awal sebelum dapat menggunakan OlahPOS.
              </p>

              {changePwError && (
                <div className="flex items-center gap-2 rounded-xl border border-rose-200 bg-rose-50 p-3 text-xs font-semibold text-rose-700 dark:border-rose-900/50 dark:bg-rose-950/40 dark:text-rose-300">
                  <AlertCircle className="h-4 w-4 shrink-0" />
                  <span>{changePwError}</span>
                </div>
              )}

              <div className="space-y-1.5">
                <label className="text-xs font-semibold text-slate-700 dark:text-slate-300">
                  Kata Sandi Baru (Min. 6 Karakter)
                </label>
                <input
                  type="password"
                  value={newPassword}
                  onChange={(e) => setNewPassword(e.target.value)}
                  className="w-full rounded-xl border border-slate-200 bg-slate-50/50 px-3.5 py-2.5 text-sm outline-none dark:border-slate-800 dark:bg-slate-800/40 focus:border-indigo-500"
                  placeholder="Kata sandi baru"
                  required
                  autoFocus
                />
              </div>

              <div className="space-y-1.5">
                <label className="text-xs font-semibold text-slate-700 dark:text-slate-300">
                  Konfirmasi Kata Sandi Baru
                </label>
                <input
                  type="password"
                  value={confirmPassword}
                  onChange={(e) => setConfirmPassword(e.target.value)}
                  className="w-full rounded-xl border border-slate-200 bg-slate-50/50 px-3.5 py-2.5 text-sm outline-none dark:border-slate-800 dark:bg-slate-800/40 focus:border-indigo-500"
                  placeholder="Ulangi kata sandi baru"
                  required
                />
              </div>

              <button
                type="submit"
                disabled={loading}
                className="w-full flex items-center justify-center gap-2 rounded-xl bg-emerald-600 py-3 text-sm font-bold text-white shadow-md shadow-emerald-600/30 hover:bg-emerald-700 active:scale-[0.98] transition-all disabled:opacity-50"
              >
                <CheckCircle2 className="h-4 w-4" />
                {loading ? "Menyimpan..." : "Simpan & Lanjutkan"}
              </button>
            </form>
          )}
        </div>
      </div>
    </div>
  );
};
