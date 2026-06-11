# Knowledge Agent Web Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 构建第一版本地 Web UI：能连接现有 Rust API，显示服务状态、vault 笔记概览，并手动触发维护扫描查看断链问题。

**Architecture:** 前端使用 `web/` 下的 React + TypeScript + Vite 单页应用。开发时 Vite 通过 proxy 调用 Rust 后端 `/api/*`；生产静态托管暂不纳入本计划，避免提前耦合 Axum 静态资源策略。UI 采用工作台结构，不做 landing page。

**Tech Stack:** React、TypeScript、Vite、Vitest、Testing Library、lucide-react、普通 CSS。

---

## Scope

本计划只实现 Web UI foundation：

- `web/` 前端工程。
- API client：`/api/health`、`/api/vault/index`、`/api/maintenance/scan`。
- App shell：侧边导航 + 主内容区。
- 页面：
  - Status：显示服务健康状态。
  - Vault：显示扫描到的 Markdown notes。
  - Maintenance：手动运行扫描并展示 inbox items。
  - Settings：占位页，说明本地配置后续接入。
- README 增加前端开发命令。

本计划不实现：

- Research Projects UI。
- Ask Vault 聊天。
- `llm-harness-core` 集成。
- 网页搜索。
- diff review UI。
- Rust 后端静态托管 `web/dist`。

## File Structure

```text
web/
  package.json
  index.html
  vite.config.ts
  tsconfig.json
  tsconfig.node.json
  src/
    main.tsx
    App.tsx
    App.test.tsx
    api.ts
    api.test.ts
    styles.css
    types.ts
    pages/
      StatusPage.tsx
      VaultPage.tsx
      MaintenancePage.tsx
      SettingsPage.tsx
README.md
```

Responsibilities:

- `api.ts`: typed fetch wrappers only.
- `types.ts`: frontend API response shapes matching Rust serde output.
- `App.tsx`: layout, navigation state, page switching.
- `pages/*`: focused view components.
- `styles.css`: global app shell and component styles.

---

### Task 1: Create Vite React TypeScript Project

**Files:**
- Create: `web/package.json`
- Create: `web/index.html`
- Create: `web/vite.config.ts`
- Create: `web/tsconfig.json`
- Create: `web/tsconfig.node.json`
- Create: `web/src/main.tsx`
- Create: `web/src/App.tsx`
- Create: `web/src/styles.css`
- Create: `web/src/vite-env.d.ts`

- [ ] **Step 1: Add package manifest**

Create `web/package.json`:

```json
{
  "name": "knowledge-agent-web",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc -b && vite build",
    "test": "vitest run",
    "test:watch": "vitest",
    "preview": "vite preview"
  },
  "dependencies": {
    "@vitejs/plugin-react": "^5.0.0",
    "lucide-react": "^0.468.0",
    "react": "^19.0.0",
    "react-dom": "^19.0.0"
  },
  "devDependencies": {
    "@testing-library/jest-dom": "^6.6.0",
    "@testing-library/react": "^16.0.0",
    "@testing-library/user-event": "^14.5.0",
    "@types/react": "^19.0.0",
    "@types/react-dom": "^19.0.0",
    "jsdom": "^25.0.0",
    "typescript": "^5.6.0",
    "vite": "^6.0.0",
    "vitest": "^2.1.0"
  }
}
```

- [ ] **Step 2: Add Vite config with API proxy**

Create `web/vite.config.ts`:

```ts
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    proxy: {
      "/api": "http://127.0.0.1:3030"
    }
  },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test-setup.ts"]
  }
});
```

- [ ] **Step 3: Add TypeScript config**

Create `web/tsconfig.json`:

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "useDefineForClassFields": true,
    "lib": ["DOM", "DOM.Iterable", "ES2022"],
    "allowJs": false,
    "skipLibCheck": true,
    "esModuleInterop": true,
    "allowSyntheticDefaultImports": true,
    "strict": true,
    "forceConsistentCasingInFileNames": true,
    "module": "ESNext",
    "moduleResolution": "Node",
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "react-jsx"
  },
  "include": ["src"],
  "references": [{ "path": "./tsconfig.node.json" }]
}
```

Create `web/tsconfig.node.json`:

```json
{
  "compilerOptions": {
    "composite": true,
    "module": "ESNext",
    "moduleResolution": "Node",
    "allowSyntheticDefaultImports": true,
    "strict": true
  },
  "include": ["vite.config.ts"]
}
```

- [ ] **Step 4: Add HTML entry**

Create `web/index.html`:

```html
<!doctype html>
<html lang="zh-CN">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Knowledge Agent</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

- [ ] **Step 5: Add minimal React app**

Create `web/src/main.tsx`:

```tsx
import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./App";
import "./styles.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
```

Create `web/src/App.tsx`:

```tsx
export function App() {
  return (
    <main className="app">
      <h1>Knowledge Agent</h1>
      <p>本地知识库研究助手</p>
    </main>
  );
}
```

Create `web/src/styles.css`:

```css
:root {
  color: #202124;
  background: #f7f8fa;
  font-family:
    Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI",
    sans-serif;
}

body {
  margin: 0;
}

.app {
  min-height: 100vh;
  padding: 32px;
}
```

Create `web/src/vite-env.d.ts`:

```ts
/// <reference types="vite/client" />
```

- [ ] **Step 6: Install dependencies**

Run:

```bash
cd web
npm install
```

Expected: `package-lock.json` is created and dependencies install successfully.

- [ ] **Step 7: Build frontend**

Run:

```bash
cd web
npm run build
```

Expected: build exits 0 and creates `web/dist/`.

- [ ] **Step 8: Commit project skeleton**

```bash
git add web/package.json web/package-lock.json web/index.html web/vite.config.ts web/tsconfig.json web/tsconfig.node.json web/src
git commit -m "feat: scaffold web ui"
```

---

### Task 2: Add Typed API Client

**Files:**
- Create: `web/src/types.ts`
- Create: `web/src/api.ts`
- Create: `web/src/test-setup.ts`
- Create: `web/src/api.test.ts`

- [ ] **Step 1: Add API response types**

Create `web/src/types.ts`:

```ts
export interface HealthResponse {
  status: "ok";
}

export interface WikiLink {
  target: string;
  alias: string | null;
}

export interface ScannedNote {
  relative_path: string;
  title: string | null;
  note_type: string | null;
  tags: string[];
  links: WikiLink[];
}

export interface VaultScan {
  root: string;
  notes: ScannedNote[];
}

export interface MaintenanceItem {
  priority: string;
  kind: string;
  file: string;
  evidence: string;
  requires_confirmation: boolean;
}

export interface MaintenanceInbox {
  items: MaintenanceItem[];
}
```

- [ ] **Step 2: Add API client tests**

Create `web/src/test-setup.ts`:

```ts
import "@testing-library/jest-dom/vitest";
```

Create `web/src/api.test.ts`:

```ts
import { afterEach, describe, expect, it, vi } from "vitest";
import { getHealth, getVaultIndex, runMaintenanceScan } from "./api";

afterEach(() => {
  vi.restoreAllMocks();
});

function mockFetch(body: unknown, ok = true) {
  vi.stubGlobal(
    "fetch",
    vi.fn().mockResolvedValue({
      ok,
      status: ok ? 200 : 500,
      text: async () => JSON.stringify(body),
      json: async () => body
    })
  );
}

describe("api client", () => {
  it("loads health", async () => {
    mockFetch({ status: "ok" });
    await expect(getHealth()).resolves.toEqual({ status: "ok" });
  });

  it("loads vault index", async () => {
    mockFetch({ root: "vault", notes: [] });
    await expect(getVaultIndex()).resolves.toEqual({ root: "vault", notes: [] });
  });

  it("runs maintenance scan", async () => {
    mockFetch({ items: [{ priority: "P0", kind: "broken_wikilink", file: "a.md", evidence: "Missing", requires_confirmation: false }] });
    const inbox = await runMaintenanceScan();
    expect(inbox.items[0].kind).toBe("broken_wikilink");
  });

  it("throws useful error for failed requests", async () => {
    mockFetch("failed", false);
    await expect(getHealth()).rejects.toThrow("GET /api/health failed with 500");
  });
});
```

- [ ] **Step 3: Implement API client**

Create `web/src/api.ts`:

```ts
import type { HealthResponse, MaintenanceInbox, VaultScan } from "./types";

async function requestJson<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(path, init);
  if (!response.ok) {
    throw new Error(`${init?.method ?? "GET"} ${path} failed with ${response.status}`);
  }
  return response.json() as Promise<T>;
}

export function getHealth(): Promise<HealthResponse> {
  return requestJson<HealthResponse>("/api/health");
}

export function getVaultIndex(): Promise<VaultScan> {
  return requestJson<VaultScan>("/api/vault/index");
}

export function runMaintenanceScan(): Promise<MaintenanceInbox> {
  return requestJson<MaintenanceInbox>("/api/maintenance/scan", { method: "POST" });
}
```

- [ ] **Step 4: Run API tests**

Run:

```bash
cd web
npm test -- --run src/api.test.ts
```

Expected: all API client tests pass.

- [ ] **Step 5: Commit API client**

```bash
git add web/src/types.ts web/src/api.ts web/src/test-setup.ts web/src/api.test.ts
git commit -m "feat: add web api client"
```

---

### Task 3: Build App Shell And Navigation

**Files:**
- Modify: `web/src/App.tsx`
- Modify: `web/src/styles.css`
- Create: `web/src/App.test.tsx`
- Create: `web/src/pages/StatusPage.tsx`
- Create: `web/src/pages/VaultPage.tsx`
- Create: `web/src/pages/MaintenancePage.tsx`
- Create: `web/src/pages/SettingsPage.tsx`

- [ ] **Step 1: Add page placeholders**

Create `web/src/pages/StatusPage.tsx`:

```tsx
export function StatusPage() {
  return <h2>服务状态</h2>;
}
```

Create `web/src/pages/VaultPage.tsx`:

```tsx
export function VaultPage() {
  return <h2>知识库</h2>;
}
```

Create `web/src/pages/MaintenancePage.tsx`:

```tsx
export function MaintenancePage() {
  return <h2>维护扫描</h2>;
}
```

Create `web/src/pages/SettingsPage.tsx`:

```tsx
export function SettingsPage() {
  return <h2>设置</h2>;
}
```

- [ ] **Step 2: Add navigation test**

Create `web/src/App.test.tsx`:

```tsx
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { App } from "./App";

describe("App", () => {
  it("starts on status page and navigates between sections", async () => {
    render(<App />);

    expect(screen.getByRole("heading", { name: "服务状态" })).toBeInTheDocument();

    await userEvent.click(screen.getByRole("button", { name: "知识库" }));
    expect(screen.getByRole("heading", { name: "知识库" })).toBeInTheDocument();

    await userEvent.click(screen.getByRole("button", { name: "维护扫描" }));
    expect(screen.getByRole("heading", { name: "维护扫描" })).toBeInTheDocument();

    await userEvent.click(screen.getByRole("button", { name: "设置" }));
    expect(screen.getByRole("heading", { name: "设置" })).toBeInTheDocument();
  });
});
```

- [ ] **Step 3: Implement app shell**

Modify `web/src/App.tsx`:

```tsx
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
```

- [ ] **Step 4: Replace CSS with app shell styles**

Modify `web/src/styles.css` with a restrained workbench layout:

```css
:root {
  color: #202124;
  background: #f6f7f9;
  font-family:
    Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI",
    sans-serif;
}

* {
  box-sizing: border-box;
}

body {
  margin: 0;
}

button {
  font: inherit;
}

.app-shell {
  display: grid;
  grid-template-columns: 260px minmax(0, 1fr);
  min-height: 100vh;
}

.sidebar {
  border-right: 1px solid #d8dde6;
  background: #ffffff;
  padding: 20px;
}

.brand {
  display: flex;
  gap: 12px;
  align-items: center;
  margin-bottom: 28px;
}

.brand-mark {
  display: inline-grid;
  place-items: center;
  width: 40px;
  height: 40px;
  border-radius: 8px;
  background: #1f6feb;
  color: #ffffff;
  font-weight: 700;
}

.brand h1 {
  margin: 0;
  font-size: 17px;
}

.brand p {
  margin: 3px 0 0;
  color: #667085;
  font-size: 13px;
}

.nav-list {
  display: grid;
  gap: 6px;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  min-height: 40px;
  border: 0;
  border-radius: 8px;
  background: transparent;
  color: #3d4654;
  cursor: pointer;
  padding: 0 10px;
  text-align: left;
}

.nav-item:hover,
.nav-item.active {
  background: #eef4ff;
  color: #1f4fbf;
}

.content {
  padding: 28px;
}

@media (max-width: 760px) {
  .app-shell {
    grid-template-columns: 1fr;
  }

  .sidebar {
    border-right: 0;
    border-bottom: 1px solid #d8dde6;
  }

  .nav-list {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}
```

- [ ] **Step 5: Run app tests**

Run:

```bash
cd web
npm test -- --run src/App.test.tsx
```

Expected: navigation test passes.

- [ ] **Step 6: Commit app shell**

```bash
git add web/src
git commit -m "feat: add web app shell"
```

---

### Task 4: Implement Status, Vault, And Maintenance Pages

**Files:**
- Modify: `web/src/pages/StatusPage.tsx`
- Modify: `web/src/pages/VaultPage.tsx`
- Modify: `web/src/pages/MaintenancePage.tsx`
- Modify: `web/src/pages/SettingsPage.tsx`
- Modify: `web/src/styles.css`
- Modify: `web/src/App.test.tsx`

- [ ] **Step 1: Add UI behavior tests**

Modify `web/src/App.test.tsx` to mock API calls:

```tsx
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { App } from "./App";

afterEach(() => {
  vi.restoreAllMocks();
});

function mockFetch() {
  vi.stubGlobal(
    "fetch",
    vi.fn((input: RequestInfo | URL) => {
      const url = String(input);
      if (url === "/api/health") {
        return Promise.resolve({ ok: true, status: 200, json: async () => ({ status: "ok" }) });
      }
      if (url === "/api/vault/index") {
        return Promise.resolve({
          ok: true,
          status: 200,
          json: async () => ({
            root: "fixture",
            notes: [
              {
                relative_path: "docs/concepts/agent-harness.md",
                title: "Agent Harness",
                note_type: "concept",
                tags: ["agent", "runtime"],
                links: [{ target: "LLM Harness", alias: null }]
              }
            ]
          })
        });
      }
      if (url === "/api/maintenance/scan") {
        return Promise.resolve({
          ok: true,
          status: 200,
          json: async () => ({
            items: [
              {
                priority: "P0",
                kind: "broken_wikilink",
                file: "docs/concepts/agent-harness.md",
                evidence: "Missing target [[LLM Harness]]",
                requires_confirmation: false
              }
            ]
          })
        });
      }
      return Promise.resolve({ ok: false, status: 404, json: async () => ({}) });
    })
  );
}

describe("App", () => {
  it("loads service status", async () => {
    mockFetch();
    render(<App />);

    await waitFor(() => expect(screen.getByText("服务在线")).toBeInTheDocument());
  });

  it("shows vault notes", async () => {
    mockFetch();
    render(<App />);

    await userEvent.click(screen.getByRole("button", { name: "知识库" }));

    expect(await screen.findByText("Agent Harness")).toBeInTheDocument();
    expect(screen.getByText("docs/concepts/agent-harness.md")).toBeInTheDocument();
  });

  it("runs maintenance scan", async () => {
    mockFetch();
    render(<App />);

    await userEvent.click(screen.getByRole("button", { name: "维护扫描" }));
    await userEvent.click(screen.getByRole("button", { name: "开始扫描" }));

    expect(await screen.findByText("broken_wikilink")).toBeInTheDocument();
    expect(screen.getByText("Missing target [[LLM Harness]]")).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Implement Status page**

Modify `web/src/pages/StatusPage.tsx`:

```tsx
import { useEffect, useState } from "react";
import { getHealth } from "../api";

export function StatusPage() {
  const [status, setStatus] = useState<"loading" | "online" | "offline">("loading");

  useEffect(() => {
    getHealth()
      .then(() => setStatus("online"))
      .catch(() => setStatus("offline"));
  }, []);

  return (
    <section className="page">
      <header className="page-header">
        <h2>服务状态</h2>
      </header>
      <div className={`status-pill ${status}`}>
        {status === "loading" && "检查中"}
        {status === "online" && "服务在线"}
        {status === "offline" && "服务离线"}
      </div>
    </section>
  );
}
```

- [ ] **Step 3: Implement Vault page**

Modify `web/src/pages/VaultPage.tsx`:

```tsx
import { useEffect, useState } from "react";
import { getVaultIndex } from "../api";
import type { VaultScan } from "../types";

export function VaultPage() {
  const [scan, setScan] = useState<VaultScan | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    getVaultIndex().then(setScan).catch((err: Error) => setError(err.message));
  }, []);

  return (
    <section className="page">
      <header className="page-header">
        <h2>知识库</h2>
        {scan && <span>{scan.notes.length} 篇 Markdown</span>}
      </header>
      {error && <p className="error-text">{error}</p>}
      <div className="note-list">
        {scan?.notes.map((note) => (
          <article className="note-row" key={note.relative_path}>
            <div>
              <h3>{note.title ?? note.relative_path}</h3>
              <p>{note.relative_path}</p>
            </div>
            <div className="tag-list">
              {note.tags.map((tag) => (
                <span className="tag" key={tag}>
                  #{tag}
                </span>
              ))}
            </div>
          </article>
        ))}
      </div>
    </section>
  );
}
```

- [ ] **Step 4: Implement Maintenance page**

Modify `web/src/pages/MaintenancePage.tsx`:

```tsx
import { useState } from "react";
import { runMaintenanceScan } from "../api";
import type { MaintenanceInbox } from "../types";

export function MaintenancePage() {
  const [inbox, setInbox] = useState<MaintenanceInbox | null>(null);
  const [isScanning, setIsScanning] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleScan() {
    setIsScanning(true);
    setError(null);
    try {
      setInbox(await runMaintenanceScan());
    } catch (err) {
      setError(err instanceof Error ? err.message : "扫描失败");
    } finally {
      setIsScanning(false);
    }
  }

  return (
    <section className="page">
      <header className="page-header">
        <h2>维护扫描</h2>
        <button className="primary-button" type="button" onClick={handleScan} disabled={isScanning}>
          {isScanning ? "扫描中" : "开始扫描"}
        </button>
      </header>
      {error && <p className="error-text">{error}</p>}
      <div className="inbox-list">
        {inbox?.items.map((item, index) => (
          <article className="inbox-item" key={`${item.file}-${item.kind}-${index}`}>
            <span className="priority">{item.priority}</span>
            <div>
              <h3>{item.kind}</h3>
              <p>{item.file}</p>
              <p>{item.evidence}</p>
            </div>
          </article>
        ))}
        {inbox && inbox.items.length === 0 && <p>没有发现维护问题。</p>}
      </div>
    </section>
  );
}
```

- [ ] **Step 5: Implement Settings placeholder**

Modify `web/src/pages/SettingsPage.tsx`:

```tsx
export function SettingsPage() {
  return (
    <section className="page">
      <header className="page-header">
        <h2>设置</h2>
      </header>
      <p className="muted">本机模型 provider、搜索 provider 和 API key 后续会写入 `.knowledge-agent/local.toml`。</p>
    </section>
  );
}
```

- [ ] **Step 6: Add page styles**

Append to `web/src/styles.css`:

```css
.page {
  display: grid;
  gap: 18px;
  max-width: 1040px;
}

.page-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}

.page-header h2 {
  margin: 0;
  font-size: 24px;
}

.page-header span,
.muted {
  color: #667085;
}

.status-pill {
  width: fit-content;
  border-radius: 999px;
  padding: 8px 12px;
  font-weight: 600;
}

.status-pill.loading {
  background: #fff7d6;
  color: #7a5800;
}

.status-pill.online {
  background: #dcfce7;
  color: #166534;
}

.status-pill.offline,
.error-text {
  color: #b42318;
}

.status-pill.offline {
  background: #fee4e2;
}

.note-list,
.inbox-list {
  display: grid;
  gap: 10px;
}

.note-row,
.inbox-item {
  display: flex;
  justify-content: space-between;
  gap: 16px;
  border: 1px solid #d8dde6;
  border-radius: 8px;
  background: #ffffff;
  padding: 14px;
}

.note-row h3,
.inbox-item h3 {
  margin: 0 0 4px;
  font-size: 16px;
}

.note-row p,
.inbox-item p {
  margin: 0;
  color: #667085;
  font-size: 13px;
}

.tag-list {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  justify-content: flex-end;
}

.tag {
  border-radius: 999px;
  background: #eef4ff;
  color: #1f4fbf;
  padding: 4px 8px;
  font-size: 12px;
}

.primary-button {
  min-height: 36px;
  border: 0;
  border-radius: 8px;
  background: #1f6feb;
  color: #ffffff;
  cursor: pointer;
  padding: 0 14px;
}

.primary-button:disabled {
  cursor: wait;
  opacity: 0.7;
}

.priority {
  align-self: flex-start;
  border-radius: 6px;
  background: #fee4e2;
  color: #b42318;
  font-weight: 700;
  padding: 4px 7px;
}
```

- [ ] **Step 7: Run UI tests**

Run:

```bash
cd web
npm test
```

Expected: API and App tests pass.

- [ ] **Step 8: Build frontend**

Run:

```bash
cd web
npm run build
```

Expected: TypeScript and Vite build pass.

- [ ] **Step 9: Commit pages**

```bash
git add web/src
git commit -m "feat: show vault and maintenance pages"
```

---

### Task 5: Document And Verify Web Development Flow

**Files:**
- Modify: `README.md`
- Modify: `.gitignore`

- [ ] **Step 1: Ensure build output is ignored**

Modify `.gitignore` if needed:

```gitignore
.superpowers/
.worktrees/
.knowledge-agent/
target/
node_modules/
dist/
web/dist/
```

- [ ] **Step 2: Document web dev flow in Chinese**

Append to `README.md`:

```md
## Web UI 开发

先启动 Rust 后端：

```bash
cargo run -p knowledge-agent-cli -- serve crates/knowledge-agent-core/tests/fixtures/basic-vault --port 3030
```

再启动前端开发服务器：

```bash
cd web
npm install
npm run dev
```

Vite 会把 `/api/*` 请求代理到 `http://127.0.0.1:3030`。

前端验证命令：

```bash
cd web
npm test
npm run build
```
```

- [ ] **Step 3: Run full verification**

Run:

```bash
cargo test
cargo fmt -- --check
cargo clippy --workspace --all-targets -- -D warnings
cd web
npm test
npm run build
```

Expected: all commands exit 0.

- [ ] **Step 4: Run manual smoke test**

In terminal 1:

```bash
cargo run -p knowledge-agent-cli -- serve crates/knowledge-agent-core/tests/fixtures/basic-vault --port 3030
```

In terminal 2:

```bash
cd web
npm run dev
```

Open `http://127.0.0.1:5173`.

Expected:

- Status page shows `服务在线`.
- Vault page shows `Agent Harness`.
- Maintenance page `开始扫描` shows `broken_wikilink` and `Missing target [[LLM Harness]]`.

- [ ] **Step 5: Commit docs**

```bash
git add .gitignore README.md
git commit -m "docs: document web development flow"
```

---

## Self-Review

Spec coverage:

- Web UI foundation: Tasks 1, 3, 4.
- Existing Rust API integration: Task 2.
- Vault overview: Task 4.
- Maintenance scan UI: Task 4.
- Chinese README update: Task 5.
- No Research/Ask Vault/LLM/search scope creep: enforced by Scope.

Known exclusions for later plans:

- Axum static file serving for `web/dist`.
- Research Projects page.
- Ask Vault chat page.
- Maintenance diff review and apply workflow.
- Provider settings persistence.
