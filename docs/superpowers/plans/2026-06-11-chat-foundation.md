# Chat Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 增加第一版本地提问通路，让 Web UI 可以向 Rust 后端发送问题并显示占位助手回复。

**Architecture:** 后端在 `knowledge-agent-server` 中新增 `POST /api/ask`，只做输入校验和占位回复。前端新增 typed API client、导航项和 `AskPage`，保持当前 React + Vite 单页应用结构。

**Tech Stack:** Rust、Axum、Serde、Tower tests、React、TypeScript、Vitest、Testing Library。

---

## File Structure

```text
crates/knowledge-agent-server/
  src/routes.rs            # 新增 ask request/response 类型和 route handler
  tests/server_tests.rs    # 新增 /api/ask contract tests

web/src/
  types.ts                 # 新增 AskRequest / AskResponse / AskSource 类型
  api.ts                   # 新增 askVault() typed fetch wrapper
  api.test.ts              # 新增 askVault API client tests
  App.tsx                  # 新增“提问”导航和页面切换
  App.test.tsx             # 新增页面交互测试
  pages/AskPage.tsx        # 新增聊天式提问页面
  styles.css               # 新增消息列表、输入区、错误态样式
```

---

### Task 1: Add Ask API To Rust Server

**Files:**
- Modify: `crates/knowledge-agent-server/src/routes.rs`
- Modify: `crates/knowledge-agent-server/tests/server_tests.rs`

- [ ] **Step 1: Write failing API tests**

Append to `crates/knowledge-agent-server/tests/server_tests.rs`:

```rust
#[tokio::test]
async fn ask_returns_placeholder_answer() {
    let vault = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../knowledge-agent-core/tests/fixtures/basic-vault");
    let app = build_router(AppState::new(vault));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/ask")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"message":"什么是 Agent Harness？","mode":"vault"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn ask_rejects_empty_message() {
    let vault = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../knowledge-agent-core/tests/fixtures/basic-vault");
    let app = build_router(AppState::new(vault));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/ask")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"message":"   ","mode":"vault"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
```

- [ ] **Step 2: Run tests to verify RED**

Run:

```bash
cargo test -p knowledge-agent-server ask
```

Expected: both new tests fail because `/api/ask` does not exist and returns `404 Not Found`.

- [ ] **Step 3: Implement minimal route**

Modify `crates/knowledge-agent-server/src/routes.rs`:

```rust
use crate::state::AppState;
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use knowledge_agent_core::{maintenance::checks::run_maintenance_scan, vault::scanner::scan_vault};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Debug, Deserialize)]
struct AskRequest {
    message: String,
    mode: AskMode,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum AskMode {
    Vault,
}

#[derive(Debug, Serialize)]
struct AskResponse {
    answer: String,
    sources: Vec<AskSource>,
    requires_followup: bool,
}

#[derive(Debug, Serialize)]
struct AskSource {
    title: String,
    path: String,
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/vault/index", get(vault_index))
        .route("/api/maintenance/scan", post(maintenance_scan))
        .route("/api/ask", post(ask))
        .with_state(state)
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

type ApiResult<T> = Result<Json<T>, (StatusCode, String)>;

async fn vault_index(State(state): State<AppState>) -> ApiResult<impl Serialize> {
    scan_vault(&state.vault_root)
        .map(Json)
        .map_err(internal_error)
}

async fn maintenance_scan(State(state): State<AppState>) -> ApiResult<impl Serialize> {
    run_maintenance_scan(&state.vault_root)
        .map(Json)
        .map_err(internal_error)
}

async fn ask(Json(request): Json<AskRequest>) -> ApiResult<AskResponse> {
    if request.message.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "message cannot be empty".to_string()));
    }

    let answer = match request.mode {
        AskMode::Vault => {
            "已收到你的问题。当前提问通路已经连通，后续会接入知识库检索和 LLM Harness。".to_string()
        }
    };

    Ok(Json(AskResponse {
        answer,
        sources: Vec::new(),
        requires_followup: false,
    }))
}

fn internal_error(err: anyhow::Error) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
```

- [ ] **Step 4: Run tests to verify GREEN**

Run:

```bash
cargo test -p knowledge-agent-server ask
```

Expected: both ask tests pass.

- [ ] **Step 5: Commit backend API**

```bash
git add crates/knowledge-agent-server/src/routes.rs crates/knowledge-agent-server/tests/server_tests.rs
git commit -m "feat: add ask api endpoint"
```

---

### Task 2: Add Web API Client Types

**Files:**
- Modify: `web/src/types.ts`
- Modify: `web/src/api.ts`
- Modify: `web/src/api.test.ts`

- [ ] **Step 1: Write failing API client test**

Append to `web/src/api.test.ts`:

```ts
it("asks the vault through the ask endpoint", async () => {
  mockFetch({ answer: "收到", sources: [], requires_followup: false });

  const response = await askVault("什么是 Agent Harness？");

  expect(response.answer).toBe("收到");
  expect(fetch).toHaveBeenCalledWith("/api/ask", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ message: "什么是 Agent Harness？", mode: "vault" })
  });
});
```

Also add `askVault` to the import list:

```ts
import { askVault, getHealth, getVaultIndex, runMaintenanceScan } from "./api";
```

- [ ] **Step 2: Run test to verify RED**

Run:

```bash
cd web
npm test -- --run src/api.test.ts
```

Expected: test fails because `askVault` is not exported.

- [ ] **Step 3: Add ask types**

Append to `web/src/types.ts`:

```ts
export interface AskRequest {
  message: string;
  mode: "vault";
}

export interface AskSource {
  title: string;
  path: string;
}

export interface AskResponse {
  answer: string;
  sources: AskSource[];
  requires_followup: boolean;
}
```

- [ ] **Step 4: Implement askVault**

Modify `web/src/api.ts` imports and add function:

```ts
import type { AskResponse, HealthResponse, MaintenanceInbox, VaultScan } from "./types";

export function askVault(message: string): Promise<AskResponse> {
  return requestJson<AskResponse>("/api/ask", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ message, mode: "vault" })
  });
}
```

- [ ] **Step 5: Run API client tests**

Run:

```bash
cd web
npm test -- --run src/api.test.ts
```

Expected: all API client tests pass.

- [ ] **Step 6: Commit client API**

```bash
git add web/src/types.ts web/src/api.ts web/src/api.test.ts
git commit -m "feat: add ask api client"
```

---

### Task 3: Add Ask Page To Web UI

**Files:**
- Create: `web/src/pages/AskPage.tsx`
- Modify: `web/src/App.tsx`
- Modify: `web/src/App.test.tsx`
- Modify: `web/src/styles.css`

- [ ] **Step 1: Write failing page tests**

Append test helpers in `web/src/App.test.tsx` mock fetch:

```ts
      if (url === "/api/ask") {
        return Promise.resolve({
          ok: true,
          status: 200,
          json: async () => ({
            answer: "已收到你的问题。当前提问通路已经连通，后续会接入知识库检索和 LLM Harness。",
            sources: [],
            requires_followup: false
          })
        });
      }
```

Append tests:

```tsx
  it("asks a question and shows the assistant reply", async () => {
    mockFetch();
    render(<App />);

    await userEvent.click(screen.getByRole("button", { name: "提问" }));
    await userEvent.type(screen.getByLabelText("问题"), "什么是 Agent Harness？");
    await userEvent.click(screen.getByRole("button", { name: "发送" }));

    expect(await screen.findByText("什么是 Agent Harness？")).toBeInTheDocument();
    expect(await screen.findByText("已收到你的问题。当前提问通路已经连通，后续会接入知识库检索和 LLM Harness。")).toBeInTheDocument();
  });
```

- [ ] **Step 2: Run page tests to verify RED**

Run:

```bash
cd web
npm test -- --run src/App.test.tsx
```

Expected: test fails because the “提问” navigation item and page do not exist.

- [ ] **Step 3: Create AskPage**

Create `web/src/pages/AskPage.tsx`:

```tsx
import { FormEvent, useState } from "react";
import { askVault } from "../api";

interface ChatMessage {
  role: "user" | "assistant";
  content: string;
}

export function AskPage() {
  const [input, setInput] = useState("");
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [isSending, setIsSending] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const message = input.trim();
    if (!message || isSending) {
      return;
    }

    setMessages((current) => [...current, { role: "user", content: message }]);
    setInput("");
    setIsSending(true);
    setError(null);

    try {
      const response = await askVault(message);
      setMessages((current) => [...current, { role: "assistant", content: response.answer }]);
    } catch (err) {
      setError(err instanceof Error ? err.message : "发送失败");
    } finally {
      setIsSending(false);
    }
  }

  return (
    <section className="page ask-page">
      <header className="page-header">
        <h2>提问</h2>
      </header>
      <div className="message-list" aria-live="polite">
        {messages.map((message, index) => (
          <article className={`message ${message.role}`} key={`${message.role}-${index}`}>
            <span>{message.role === "user" ? "你" : "助手"}</span>
            <p>{message.content}</p>
          </article>
        ))}
      </div>
      {error && <p className="error-text">{error}</p>}
      <form className="ask-form" onSubmit={handleSubmit}>
        <label className="sr-only" htmlFor="ask-message">
          问题
        </label>
        <textarea
          id="ask-message"
          value={input}
          onChange={(event) => setInput(event.target.value)}
          placeholder="向知识库提问"
          rows={3}
        />
        <button className="primary-button" disabled={isSending || input.trim().length === 0} type="submit">
          {isSending ? "发送中" : "发送"}
        </button>
      </form>
    </section>
  );
}
```

- [ ] **Step 4: Add navigation entry**

Modify `web/src/App.tsx`:

```tsx
import { Activity, Database, MessageSquareText, ScanSearch, Settings } from "lucide-react";
import { AskPage } from "./pages/AskPage";
```

Update types and nav:

```tsx
type Page = "status" | "ask" | "vault" | "maintenance" | "settings";

const navItems: Array<{ id: Page; label: string; icon: typeof Activity }> = [
  { id: "status", label: "服务状态", icon: Activity },
  { id: "ask", label: "提问", icon: MessageSquareText },
  { id: "vault", label: "知识库", icon: Database },
  { id: "maintenance", label: "维护扫描", icon: ScanSearch },
  { id: "settings", label: "设置", icon: Settings }
];
```

Add render branch:

```tsx
{page === "ask" && <AskPage />}
```

- [ ] **Step 5: Add styles**

Append to `web/src/styles.css`:

```css
.ask-page {
  min-height: calc(100vh - 56px);
  grid-template-rows: auto minmax(220px, 1fr) auto auto;
}

.message-list {
  display: grid;
  align-content: start;
  gap: 10px;
}

.message {
  max-width: 720px;
  border: 1px solid #d8dde6;
  border-radius: 8px;
  background: #ffffff;
  padding: 12px 14px;
}

.message.user {
  justify-self: end;
  background: #eef4ff;
}

.message span {
  display: block;
  margin-bottom: 5px;
  color: #667085;
  font-size: 12px;
  font-weight: 700;
}

.message p {
  margin: 0;
}

.ask-form {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 10px;
  align-items: end;
}

.ask-form textarea {
  width: 100%;
  resize: vertical;
  border: 1px solid #cbd2df;
  border-radius: 8px;
  font: inherit;
  padding: 10px 12px;
}

.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
}
```

- [ ] **Step 6: Run page tests**

Run:

```bash
cd web
npm test -- --run src/App.test.tsx
```

Expected: all App tests pass.

- [ ] **Step 7: Commit page**

```bash
git add web/src/App.tsx web/src/App.test.tsx web/src/pages/AskPage.tsx web/src/styles.css
git commit -m "feat: add ask page"
```

---

### Task 4: Verify Full Stack

**Files:**
- No source changes expected.

- [ ] **Step 1: Run Rust verification**

Run:

```bash
cargo test
cargo fmt -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

Expected: all commands exit 0.

- [ ] **Step 2: Run Web verification**

Run:

```bash
cd web
npm test
npm run build
```

Expected: all commands exit 0.

- [ ] **Step 3: Browser smoke test**

Start backend:

```bash
cargo run -p knowledge-agent-cli -- serve crates/knowledge-agent-core/tests/fixtures/basic-vault --port 3030
```

Start frontend:

```bash
cd web
npm run dev -- --host 127.0.0.1
```

Open `http://127.0.0.1:5173` and verify:

- Status page shows “服务在线”。
- 提问页 can send “什么是 Agent Harness？”。
- Assistant reply shows “当前提问通路已经连通”。

---

## Self-Review

- Spec coverage: `POST /api/ask` in Task 1, Web client in Task 2, Web page in Task 3, verification in Task 4.
- No LLM, web search, vault retrieval, streaming, writeback, or session persistence is included.
- Type names match between backend response and frontend `AskResponse`.
- Empty message behavior is covered by Rust API test and frontend disabled submit button.
