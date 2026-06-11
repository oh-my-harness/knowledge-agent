import { Activity, Database, ScanSearch, Settings } from "lucide-react";
import { useState } from "react";
import { MaintenancePage } from "./pages/MaintenancePage";
import { SettingsPage } from "./pages/SettingsPage";
import { StatusPage } from "./pages/StatusPage";
import { VaultPage } from "./pages/VaultPage";

type Page = "status" | "vault" | "maintenance" | "settings";

const navItems: Array<{ id: Page; label: string; icon: typeof Activity }> = [
  { id: "status", label: "服务状态", icon: Activity },
  { id: "vault", label: "知识库", icon: Database },
  { id: "maintenance", label: "维护扫描", icon: ScanSearch },
  { id: "settings", label: "设置", icon: Settings }
];

export function App() {
  const [page, setPage] = useState<Page>("status");

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <span className="brand-mark">KA</span>
          <div>
            <h1>Knowledge Agent</h1>
            <p>本地知识库工作台</p>
          </div>
        </div>
        <nav className="nav-list" aria-label="主导航">
          {navItems.map((item) => {
            const Icon = item.icon;
            return (
              <button
                key={item.id}
                className={page === item.id ? "nav-item active" : "nav-item"}
                onClick={() => setPage(item.id)}
                type="button"
              >
                <Icon size={18} aria-hidden="true" />
                {item.label}
              </button>
            );
          })}
        </nav>
      </aside>
      <main className="content">
        {page === "status" && <StatusPage />}
        {page === "vault" && <VaultPage />}
        {page === "maintenance" && <MaintenancePage />}
        {page === "settings" && <SettingsPage />}
      </main>
    </div>
  );
}
