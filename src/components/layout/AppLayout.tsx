import React, { useState } from "react";
import {
  LayoutDashboard,
  ShoppingCart,
  Package,
  Layers,
  Truck,
  CreditCard,
  FileBarChart,
  Settings,
  HardDriveDownload,
  Menu,
  X,
  Bell,
  UserCheck,
  ShieldCheck,
  Terminal,
  LogOut,
} from "lucide-react";
import { cn } from "@/lib/utils";
import type { UserSession } from "@/features/auth/LoginPage";

export type NavItem = {
  id: string;
  label: string;
  icon: React.ElementType;
  badge?: string;
  developerOnly?: boolean;
};

const NAV_ITEMS: NavItem[] = [
  { id: "kasir", label: "Kasir (POS)", icon: ShoppingCart },
  { id: "dashboard", label: "Dashboard", icon: LayoutDashboard },
  { id: "barang", label: "Data Barang", icon: Package },
  { id: "inventori", label: "Inventori & Stok", icon: Layers },
  { id: "pembelian", label: "Pembelian", icon: Truck },
  { id: "utang", label: "Utang Pembelian", icon: CreditCard },
  { id: "laporan", label: "Laporan & Keuangan", icon: FileBarChart },
  { id: "backup", label: "Backup & Restore", icon: HardDriveDownload },
  { id: "pengaturan", label: "Pengaturan", icon: Settings },
  { id: "developer", label: "Menu Developer", icon: Terminal, developerOnly: true },
];

interface AppLayoutProps {
  activeTab: string;
  onSelectTab: (tabId: string) => void;
  currentUser?: UserSession;
  onLogout?: () => void;
  children: React.ReactNode;
}

export const AppLayout: React.FC<AppLayoutProps> = ({
  activeTab,
  onSelectTab,
  currentUser,
  onLogout,
  children,
}) => {
  const [sidebarOpen, setSidebarOpen] = useState(true);

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-slate-50 text-slate-800 dark:bg-slate-950 dark:text-slate-100">
      {/* Sidebar */}
      <aside
        className={cn(
          "flex flex-col border-r border-slate-200/80 bg-white/95 backdrop-blur-md transition-all duration-300 dark:border-slate-800 dark:bg-slate-900/95 z-20",
          sidebarOpen ? "w-64" : "w-20"
        )}
      >
        {/* Brand Header */}
        <div className="flex h-16 items-center justify-between px-4 border-b border-slate-100 dark:border-slate-800/80">
          <div className="flex items-center gap-3 overflow-hidden">
            <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-gradient-to-br from-indigo-600 to-violet-600 text-white shadow-md shadow-indigo-500/20">
              <ShieldCheck className="h-5 w-5" />
            </div>
            {sidebarOpen && (
              <div className="flex flex-col">
                <span className="font-bold tracking-tight text-slate-900 dark:text-white text-lg leading-tight">
                  Olah<span className="text-indigo-600 dark:text-indigo-400">POS</span>
                </span>
                <span className="text-[11px] font-medium text-slate-400">Desktop System v1.0</span>
              </div>
            )}
          </div>
          <button
            onClick={() => setSidebarOpen(!sidebarOpen)}
            className="rounded-lg p-1.5 text-slate-400 hover:bg-slate-100 hover:text-slate-600 dark:hover:bg-slate-800 dark:hover:text-slate-300"
            title={sidebarOpen ? "Ciutkan menu" : "Bentangkan menu"}
          >
            {sidebarOpen ? <X className="h-5 w-5" /> : <Menu className="h-5 w-5" />}
          </button>
        </div>

        {/* Navigation Links */}
        <nav className="flex-1 space-y-1 overflow-y-auto px-3 py-4">
          {NAV_ITEMS.map((item) => {
            const isDeveloper =
              currentUser?.nama_pengguna === "developer" || currentUser?.peran === "ADMIN";
            if (item.developerOnly && !isDeveloper) {
              return null;
            }

            const Icon = item.icon;
            const isActive = activeTab === item.id;
            return (
              <button
                key={item.id}
                onClick={() => onSelectTab(item.id)}
                className={cn(
                  "group flex w-full items-center gap-3 rounded-xl px-3 py-2.5 text-sm font-semibold transition-all duration-150",
                  isActive
                    ? "bg-indigo-600 text-white shadow-sm shadow-indigo-600/30 dark:bg-indigo-600"
                    : "text-slate-600 hover:bg-slate-100/80 hover:text-slate-900 dark:text-slate-400 dark:hover:bg-slate-800/60 dark:hover:text-slate-100"
                )}
                title={!sidebarOpen ? item.label : undefined}
              >
                <Icon
                  className={cn(
                    "h-5 w-5 shrink-0 transition-transform duration-150 group-hover:scale-110",
                    isActive ? "text-white" : "text-slate-500 group-hover:text-slate-700 dark:text-slate-400 dark:group-hover:text-slate-200"
                  )}
                />
                {sidebarOpen && <span className="truncate">{item.label}</span>}
              </button>
            );
          })}
        </nav>

        {/* User Pill Footer with Logout */}
        <div className="border-t border-slate-100 p-3 dark:border-slate-800/80">
          <div
            className={cn(
              "flex items-center gap-3 rounded-xl bg-slate-100/70 p-2.5 dark:bg-slate-800/50",
              !sidebarOpen && "justify-center"
            )}
          >
            <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-indigo-100 text-indigo-700 dark:bg-indigo-950 dark:text-indigo-300 font-bold text-sm">
              <UserCheck className="h-5 w-5" />
            </div>
            {sidebarOpen && (
              <div className="flex flex-col min-w-0 flex-1">
                <span className="truncate text-xs font-semibold text-slate-800 dark:text-slate-200">
                  {currentUser?.nama_lengkap ?? "Administrator"}
                </span>
                <span className="truncate text-[10px] text-emerald-600 dark:text-emerald-400 font-medium flex items-center gap-1">
                  <span className="h-1.5 w-1.5 rounded-full bg-emerald-500 animate-pulse"></span>
                  {currentUser?.peran ?? "ADMIN"} • Local Aktif
                </span>
              </div>
            )}
            {sidebarOpen && onLogout && (
              <button
                onClick={onLogout}
                className="rounded-lg p-1.5 text-slate-400 hover:bg-rose-50 hover:text-rose-600 dark:hover:bg-rose-950/40 dark:hover:text-rose-400 transition-colors"
                title="Keluar"
              >
                <LogOut className="h-4 w-4" />
              </button>
            )}
          </div>
        </div>
      </aside>

      {/* Main Content Area */}
      <div className="flex flex-1 flex-col overflow-hidden">
        {/* Top Navbar */}
        <header className="flex h-16 shrink-0 items-center justify-between border-b border-slate-200/80 bg-white/70 px-6 backdrop-blur-md dark:border-slate-800 dark:bg-slate-900/70">
          <div className="flex items-center gap-2">
            <h1 className="text-base font-bold text-slate-900 dark:text-white capitalize">
              {NAV_ITEMS.find((n) => n.id === activeTab)?.label ?? "OlahPOS"}
            </h1>
            <span className="text-slate-300 dark:text-slate-700">•</span>
            <span className="text-xs text-slate-500 font-medium">Single Desktop Mode</span>
          </div>

          <div className="flex items-center gap-3">
            <div className="flex items-center gap-2 rounded-full border border-emerald-200 bg-emerald-50/80 px-3 py-1 text-xs font-semibold text-emerald-700 dark:border-emerald-900/50 dark:bg-emerald-950/40 dark:text-emerald-300">
              <span className="h-2 w-2 rounded-full bg-emerald-500"></span>
              SQLite Dual-DB Ready
            </div>
            <button
              className="rounded-xl border border-slate-200 p-2 text-slate-500 hover:bg-slate-100 hover:text-slate-700 dark:border-slate-800 dark:text-slate-400 dark:hover:bg-slate-800"
              title="Notifikasi"
            >
              <Bell className="h-4 w-4" />
            </button>
          </div>
        </header>

        {/* Scrollable Viewport */}
        <main className="flex-1 overflow-y-auto p-6">
          <div className="mx-auto max-w-7xl">{children}</div>
        </main>
      </div>
    </div>
  );
};
