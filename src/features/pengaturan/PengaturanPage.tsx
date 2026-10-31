import React, { useState, useEffect } from "react";
import {
  Settings,
  ShieldCheck,
  Store,
  QrCode,
  KeyRound,
  Eye,
  EyeOff,
  Lock,
  Save,
  CheckCircle2,
  AlertCircle,
  Copy,
} from "lucide-react";
import type { UserSession } from "@/features/auth/LoginPage";

export const PengaturanPage: React.FC<{ currentUser?: UserSession }> = ({ currentUser }) => {
  const [activeTab, setActiveTab] = useState<"store" | "qris" | "security">("qris");

  // Store Profile State
  const [storeName, setStoreName] = useState(
    () => localStorage.getItem("olahpos_store_name") || "Toko OlahPOS Sejahtera"
  );
  const [storeAddress, setStoreAddress] = useState(
    () => localStorage.getItem("olahpos_store_address") || "Jl. Sudirman No. 45, Bandung"
  );
  const [storePhone, setStorePhone] = useState(
    () => localStorage.getItem("olahpos_store_phone") || "0812-3456-7890"
  );
  const [receiptFooter, setReceiptFooter] = useState(
    () =>
      localStorage.getItem("olahpos_receipt_footer") ||
      "Terima kasih atas kunjungan Anda!\nBarang yang sudah dibeli tidak dapat ditukar."
  );
  const [storeSaved, setStoreSaved] = useState(false);

  // Midtrans State
  const [serverKey, setServerKey] = useState("");
  const [clientKey, setClientKey] = useState("");
  const [merchantId, setMerchantId] = useState("");
  const [environment, setEnvironment] = useState<"sandbox" | "production">("sandbox");
  const [showServerKey, setShowServerKey] = useState(false);
  const [savingMidtrans, setSavingMidtrans] = useState(false);
  const [midtransSaved, setMidtransSaved] = useState(false);
  const [testedQr, setTestedQr] = useState<string | null>(null);

  // Security / Password Change State
  const [oldPassword, setOldPassword] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [pwError, setPwError] = useState<string | null>(null);
  const [pwSuccess, setPwSuccess] = useState(false);
  const [changingPw, setChangingPw] = useState(false);

  useEffect(() => {
    // Load existing encrypted config for Midtrans
    const loadConfig = async () => {
      if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
        try {
          const { invoke } = await import("@tauri-apps/api/core");
          const config = await invoke<any>("get_integration_config", {
            kodeLayanan: "MIDTRANS_QRIS",
          });
          if (config) {
            if (config.server_key) setServerKey(config.server_key);
            if (config.client_key) setClientKey(config.client_key);
            if (config.merchant_id) setMerchantId(config.merchant_id);
            if (config.environment) setEnvironment(config.environment);
          }
        } catch (e) {
          console.error("Gagal load config midtrans:", e);
        }
      }
    };
    loadConfig();
  }, []);

  const handleSaveStore = (e: React.FormEvent) => {
    e.preventDefault();
    localStorage.setItem("olahpos_store_name", storeName);
    localStorage.setItem("olahpos_store_address", storeAddress);
    localStorage.setItem("olahpos_store_phone", storePhone);
    localStorage.setItem("olahpos_receipt_footer", receiptFooter);
    setStoreSaved(true);
    setTimeout(() => setStoreSaved(false), 3000);
  };

  const handleSaveMidtrans = async (e: React.FormEvent) => {
    e.preventDefault();
    setSavingMidtrans(true);
    const payload = {
      server_key: serverKey,
      client_key: clientKey,
      merchant_id: merchantId,
      environment,
    };

    if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        await invoke("save_integration_config", {
          kodeLayanan: "MIDTRANS_QRIS",
          namaLayanan: "Midtrans Payment Gateway",
          payload,
          catatan: `Konfigurasi ${environment}`,
          userId: currentUser?.pengguna_id || 1,
        });
        setMidtransSaved(true);
        setTimeout(() => setMidtransSaved(false), 3000);
      } catch (err) {
        alert("Gagal menyimpan konfigurasi terenkripsi: " + err);
      }
    } else {
      setMidtransSaved(true);
      setTimeout(() => setMidtransSaved(false), 3000);
    }
    setSavingMidtrans(false);
  };

  const handleTestQRIS = async () => {
    if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        const res = await invoke<any>("generate_qris", {
          orderId: `TEST-${Date.now().toString().slice(-6)}`,
          grossAmount: 25000,
        });
        setTestedQr(res.qr_string);
      } catch (err) {
        alert("Gagal uji QRIS: " + err);
      }
    } else {
      setTestedQr("00020101021226670014ID.LINKAJA.WWW01189360050300000898740215TEST-9999");
    }
  };

  const handleChangePassword = async (e: React.FormEvent) => {
    e.preventDefault();
    setPwError(null);
    setPwSuccess(false);

    if (newPassword.length < 6) {
      setPwError("Password baru minimal harus 6 karakter!");
      return;
    }
    if (newPassword !== confirmPassword) {
      setPwError("Konfirmasi password baru tidak cocok!");
      return;
    }

    setChangingPw(true);
    if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        await invoke("change_password", {
          penggunaId: currentUser?.pengguna_id || 1,
          oldPassword,
          newPassword,
        });
        setPwSuccess(true);
        setOldPassword("");
        setNewPassword("");
        setConfirmPassword("");
      } catch (err) {
        setPwError(String(err));
      }
    } else {
      setPwSuccess(true);
      setOldPassword("");
      setNewPassword("");
      setConfirmPassword("");
    }
    setChangingPw(false);
  };

  return (
    <div className="space-y-6">
      {/* Top Header */}
      <div>
        <h2 className="text-2xl font-bold tracking-tight text-slate-900 dark:text-white flex items-center gap-2">
          <Settings className="h-6 w-6 text-indigo-600" />
          Pengaturan & Konfigurasi Integrasi
        </h2>
        <p className="text-xs text-slate-500 mt-1">
          Konfigurasi toko, integrasi API gateway dengan enkripsi simetris AES-256-GCM, dan manajemen
          keamanan akun.
        </p>
      </div>

      {/* Tabs */}
      <div className="flex border-b border-slate-200 dark:border-slate-800">
        <button
          onClick={() => setActiveTab("qris")}
          className={`flex items-center gap-2 border-b-2 px-4 py-2.5 text-xs font-semibold transition-colors ${
            activeTab === "qris"
              ? "border-indigo-600 text-indigo-600 dark:border-indigo-400 dark:text-indigo-400"
              : "border-transparent text-slate-500 hover:text-slate-800 dark:hover:text-slate-200"
          }`}
        >
          <QrCode className="h-4 w-4" />
          Integrasi Midtrans QRIS (AES Encrypted)
        </button>

        <button
          onClick={() => setActiveTab("store")}
          className={`flex items-center gap-2 border-b-2 px-4 py-2.5 text-xs font-semibold transition-colors ${
            activeTab === "store"
              ? "border-indigo-600 text-indigo-600 dark:border-indigo-400 dark:text-indigo-400"
              : "border-transparent text-slate-500 hover:text-slate-800 dark:hover:text-slate-200"
          }`}
        >
          <Store className="h-4 w-4" />
          Profil Usaha & Struk
        </button>

        <button
          onClick={() => setActiveTab("security")}
          className={`flex items-center gap-2 border-b-2 px-4 py-2.5 text-xs font-semibold transition-colors ${
            activeTab === "security"
              ? "border-indigo-600 text-indigo-600 dark:border-indigo-400 dark:text-indigo-400"
              : "border-transparent text-slate-500 hover:text-slate-800 dark:hover:text-slate-200"
          }`}
        >
          <Lock className="h-4 w-4" />
          Keamanan & Akun
        </button>
      </div>

      {/* Tab 1: Integrasi Midtrans QRIS */}
      {activeTab === "qris" && (
        <div className="max-w-2xl space-y-5">
          {/* Security Banner */}
          <div className="flex items-start gap-3 rounded-2xl border border-indigo-100 bg-indigo-50/60 p-4 dark:border-indigo-900/40 dark:bg-indigo-950/30">
            <ShieldCheck className="h-5 w-5 text-indigo-600 shrink-0 mt-0.5" />
            <div className="text-xs">
              <span className="font-bold text-indigo-950 dark:text-indigo-200">
                Enkripsi Simetris (AES-256-GCM) Aktif
              </span>
              <p className="text-slate-600 dark:text-slate-400 mt-0.5 leading-relaxed">
                Seluruh kredensial API (Server Key, Client Key) disimpan di SQLite dalam keadaan
                terenkripsi. Kunci enkripsi simetris disimpan secara aman pada{" "}
                <code className="rounded bg-indigo-100 px-1 py-0.5 font-mono text-[11px] dark:bg-indigo-900/60">
                  config.json
                </code>{" "}
                di direktori aplikasi lokal.
              </p>
            </div>
          </div>

          <form
            onSubmit={handleSaveMidtrans}
            className="rounded-2xl border border-slate-200/80 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-900 space-y-4"
          >
            <div>
              <label className="text-xs font-semibold text-slate-700 dark:text-slate-300">
                Lingkungan / Environment
              </label>
              <div className="mt-1 flex gap-2">
                <button
                  type="button"
                  onClick={() => setEnvironment("sandbox")}
                  className={`flex-1 rounded-xl py-2 text-xs font-bold border transition-colors ${
                    environment === "sandbox"
                      ? "bg-indigo-600 text-white border-indigo-600 shadow"
                      : "border-slate-200 text-slate-600 hover:bg-slate-50 dark:border-slate-700 dark:text-slate-300"
                  }`}
                >
                  Sandbox (Uji Coba)
                </button>
                <button
                  type="button"
                  onClick={() => setEnvironment("production")}
                  className={`flex-1 rounded-xl py-2 text-xs font-bold border transition-colors ${
                    environment === "production"
                      ? "bg-indigo-600 text-white border-indigo-600 shadow"
                      : "border-slate-200 text-slate-600 hover:bg-slate-50 dark:border-slate-700 dark:text-slate-300"
                  }`}
                >
                  Production (Live)
                </button>
              </div>
            </div>

            <div>
              <label className="text-xs font-semibold text-slate-700 dark:text-slate-300">
                Midtrans Server Key
              </label>
              <div className="relative mt-1">
                <input
                  type={showServerKey ? "text" : "password"}
                  value={serverKey}
                  onChange={(e) => setServerKey(e.target.value)}
                  placeholder="SB-Mid-server-xxxxxxxxxxxx"
                  className="w-full rounded-xl border border-slate-200 px-3 py-2 pr-10 text-xs font-mono outline-none focus:ring-2 focus:ring-indigo-500/20 dark:border-slate-700 dark:bg-slate-800 dark:text-white"
                />
                <button
                  type="button"
                  onClick={() => setShowServerKey(!showServerKey)}
                  className="absolute right-3 top-2.5 text-slate-400 hover:text-slate-600"
                >
                  {showServerKey ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                </button>
              </div>
              <span className="text-[11px] text-slate-400 mt-0.5 block">
                Digunakan untuk otorisasi charge QRIS ke API Midtrans.
              </span>
            </div>

            <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
              <div>
                <label className="text-xs font-semibold text-slate-700 dark:text-slate-300">
                  Client Key
                </label>
                <input
                  type="text"
                  value={clientKey}
                  onChange={(e) => setClientKey(e.target.value)}
                  placeholder="SB-Mid-client-xxxxxxxxxxxx"
                  className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-xs font-mono outline-none focus:ring-2 focus:ring-indigo-500/20 dark:border-slate-700 dark:bg-slate-800 dark:text-white"
                />
              </div>

              <div>
                <label className="text-xs font-semibold text-slate-700 dark:text-slate-300">
                  Merchant ID
                </label>
                <input
                  type="text"
                  value={merchantId}
                  onChange={(e) => setMerchantId(e.target.value)}
                  placeholder="Gxxxxxxxx"
                  className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-xs font-mono outline-none focus:ring-2 focus:ring-indigo-500/20 dark:border-slate-700 dark:bg-slate-800 dark:text-white"
                />
              </div>
            </div>

            {midtransSaved && (
              <div className="flex items-center gap-2 rounded-xl bg-emerald-50 p-3 text-xs font-semibold text-emerald-700 dark:bg-emerald-950/60 dark:text-emerald-400">
                <CheckCircle2 className="h-4 w-4 shrink-0" />
                Konfigurasi Midtrans QRIS berhasil disimpan dengan enkripsi AES-256-GCM.
              </div>
            )}

            <div className="flex items-center justify-between pt-2 border-t border-slate-100 dark:border-slate-800">
              <button
                type="button"
                onClick={handleTestQRIS}
                className="rounded-xl border border-slate-200 px-3.5 py-2 text-xs font-semibold text-slate-700 hover:bg-slate-50 dark:border-slate-700 dark:text-slate-300 flex items-center gap-1.5"
              >
                <QrCode className="h-3.5 w-3.5" /> Uji Buat String QRIS
              </button>

              <button
                type="submit"
                disabled={savingMidtrans}
                className="rounded-xl bg-indigo-600 px-5 py-2 text-xs font-bold text-white shadow-md hover:bg-indigo-700 flex items-center gap-1.5"
              >
                <Save className="h-3.5 w-3.5" />
                {savingMidtrans ? "Menyimpan..." : "Simpan Terenkripsi"}
              </button>
            </div>
          </form>

          {/* Test QR Result */}
          {testedQr && (
            <div className="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm dark:border-slate-800 dark:bg-slate-900 text-xs space-y-2">
              <div className="flex justify-between items-center">
                <span className="font-bold text-slate-800 dark:text-slate-200">
                  Hasil Simulasi QRIS EMVCo String:
                </span>
                <button
                  onClick={() => navigator.clipboard.writeText(testedQr)}
                  className="flex items-center gap-1 text-[11px] font-semibold text-indigo-600 hover:text-indigo-800"
                >
                  <Copy className="h-3.5 w-3.5" /> Salin String
                </button>
              </div>
              <p className="font-mono text-[10px] break-all bg-slate-50 p-3 rounded-xl border border-slate-100 dark:bg-slate-800 dark:border-slate-700 text-slate-600 dark:text-slate-300">
                {testedQr}
              </p>
            </div>
          )}
        </div>
      )}

      {/* Tab 2: Profil Usaha & Struk */}
      {activeTab === "store" && (
        <form
          onSubmit={handleSaveStore}
          className="max-w-xl rounded-2xl border border-slate-200/80 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-900 space-y-4 text-xs"
        >
          <div>
            <label className="font-semibold text-slate-700 dark:text-slate-300">Nama Usaha / Toko</label>
            <input
              type="text"
              value={storeName}
              onChange={(e) => setStoreName(e.target.value)}
              className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 outline-none focus:ring-2 focus:ring-indigo-500/20 dark:border-slate-700 dark:bg-slate-800 dark:text-white"
            />
          </div>

          <div>
            <label className="font-semibold text-slate-700 dark:text-slate-300">Alamat Usaha</label>
            <input
              type="text"
              value={storeAddress}
              onChange={(e) => setStoreAddress(e.target.value)}
              className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 outline-none focus:ring-2 focus:ring-indigo-500/20 dark:border-slate-700 dark:bg-slate-800 dark:text-white"
            />
          </div>

          <div>
            <label className="font-semibold text-slate-700 dark:text-slate-300">Nomor Telepon / WhatsApp</label>
            <input
              type="text"
              value={storePhone}
              onChange={(e) => setStorePhone(e.target.value)}
              className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 outline-none focus:ring-2 focus:ring-indigo-500/20 dark:border-slate-700 dark:bg-slate-800 dark:text-white"
            />
          </div>

          <div>
            <label className="font-semibold text-slate-700 dark:text-slate-300">Pesan Footer Struk Pembelian</label>
            <textarea
              value={receiptFooter}
              onChange={(e) => setReceiptFooter(e.target.value)}
              rows={3}
              className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 outline-none focus:ring-2 focus:ring-indigo-500/20 dark:border-slate-700 dark:bg-slate-800 dark:text-white"
            />
          </div>

          {storeSaved && (
            <div className="flex items-center gap-2 rounded-xl bg-emerald-50 p-3 text-xs font-semibold text-emerald-700 dark:bg-emerald-950/60 dark:text-emerald-400">
              <CheckCircle2 className="h-4 w-4 shrink-0" />
              Profil usaha berhasil disimpan.
            </div>
          )}

          <div className="flex justify-end pt-2">
            <button
              type="submit"
              className="rounded-xl bg-indigo-600 px-5 py-2 font-bold text-white shadow-md hover:bg-indigo-700 flex items-center gap-1.5"
            >
              <Save className="h-3.5 w-3.5" /> Simpan Profil
            </button>
          </div>
        </form>
      )}

      {/* Tab 3: Keamanan & Akun */}
      {activeTab === "security" && (
        <form
          onSubmit={handleChangePassword}
          className="max-w-md rounded-2xl border border-slate-200/80 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-900 space-y-4 text-xs"
        >
          <div className="rounded-xl bg-slate-50 p-3 dark:bg-slate-800/60 space-y-1">
            <span className="text-[11px] text-slate-400 uppercase font-semibold">Pengguna Aktif</span>
            <div className="font-bold text-slate-900 dark:text-white text-sm">
              {currentUser?.nama_lengkap}
            </div>
            <div className="text-slate-500 text-[11px]">
              Username: <span className="font-mono">{currentUser?.nama_pengguna}</span> • Peran:{" "}
              <span className="font-semibold text-indigo-600">{currentUser?.peran}</span>
            </div>
          </div>

          <div>
            <label className="font-semibold text-slate-700 dark:text-slate-300">Password Lama</label>
            <input
              type="password"
              value={oldPassword}
              onChange={(e) => setOldPassword(e.target.value)}
              required
              className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 outline-none focus:ring-2 focus:ring-indigo-500/20 dark:border-slate-700 dark:bg-slate-800 dark:text-white"
            />
          </div>

          <div>
            <label className="font-semibold text-slate-700 dark:text-slate-300">Password Baru</label>
            <input
              type="password"
              value={newPassword}
              onChange={(e) => setNewPassword(e.target.value)}
              required
              minLength={6}
              className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 outline-none focus:ring-2 focus:ring-indigo-500/20 dark:border-slate-700 dark:bg-slate-800 dark:text-white"
            />
          </div>

          <div>
            <label className="font-semibold text-slate-700 dark:text-slate-300">Konfirmasi Password Baru</label>
            <input
              type="password"
              value={confirmPassword}
              onChange={(e) => setConfirmPassword(e.target.value)}
              required
              className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 outline-none focus:ring-2 focus:ring-indigo-500/20 dark:border-slate-700 dark:bg-slate-800 dark:text-white"
            />
          </div>

          {pwError && (
            <div className="flex items-center gap-2 rounded-xl bg-rose-50 p-3 font-semibold text-rose-700 dark:bg-rose-950/60 dark:text-rose-400">
              <AlertCircle className="h-4 w-4 shrink-0" />
              {pwError}
            </div>
          )}

          {pwSuccess && (
            <div className="flex items-center gap-2 rounded-xl bg-emerald-50 p-3 font-semibold text-emerald-700 dark:bg-emerald-950/60 dark:text-emerald-400">
              <CheckCircle2 className="h-4 w-4 shrink-0" />
              Password berhasil diubah dengan hash Argon2id yang aman.
            </div>
          )}

          <div className="flex justify-end pt-2">
            <button
              type="submit"
              disabled={changingPw}
              className="rounded-xl bg-indigo-600 px-5 py-2 font-bold text-white shadow-md hover:bg-indigo-700 flex items-center gap-1.5"
            >
              <KeyRound className="h-3.5 w-3.5" />
              {changingPw ? "Menyimpan..." : "Perbarui Password"}
            </button>
          </div>
        </form>
      )}
    </div>
  );
};
