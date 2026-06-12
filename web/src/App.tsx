import { FileInput, MessageSquareText, ScanSearch, Settings } from "lucide-react";
import { useState } from "react";
import { AskPage } from "./pages/AskPage";
import { IngestPage } from "./pages/IngestPage";
import { MaintenancePage } from "./pages/MaintenancePage";
import { SettingsPage } from "./pages/SettingsPage";

type Page = "ask" | "ingest" | "maintenance" | "settings";

const navItems: Array<{ id: Page; label: string; icon: typeof MessageSquareText }> = [
  { id: "ask", label: "提问", icon: MessageSquareText },
  { id: "ingest", label: "资料摄入", icon: FileInput },
  { id: "maintenance", label: "维护扫描", icon: ScanSearch },
  { id: "settings", label: "设置", icon: Settings }
];

export function App() {
  const [page, setPage] = useState<Page>("ask");

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
        {page === "ask" && <AskPage />}
        {page === "ingest" && <IngestPage />}
        {page === "maintenance" && <MaintenancePage />}
        {page === "settings" && <SettingsPage />}
      </main>
    </div>
  );
}
