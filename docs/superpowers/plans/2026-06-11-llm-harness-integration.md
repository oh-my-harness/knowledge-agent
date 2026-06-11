# LLM Harness Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the fixed `/api/ask` placeholder with a single-turn LLM answer produced through `oh-my-harness/llm-harness-core`.

**Architecture:** Add a focused `knowledge-agent-harness` crate that exposes an `AskRunner` trait, a fake runner for tests, and a DeepSeek-backed runner using `llm-harness-core` plus `llm_adapter`. Inject the runner through server `AppState` so default tests never call the network.

**Tech Stack:** Rust 2024, Axum, Tokio, Serde, llm-harness, llm-harness-loop, llm_adapter, anyhow, Tower tests.

---

## File Structure

```text
Cargo.toml
  # Add knowledge-agent-harness to workspace members and shared dependency pins.

crates/knowledge-agent-harness/
  Cargo.toml
  src/lib.rs
  src/ask.rs
  tests/ask_tests.rs

crates/knowledge-agent-server/
  Cargo.toml
  src/state.rs
  src/routes.rs
  tests/server_tests.rs
```

---

### Task 1: Add Harness Crate With Testable Ask Runner

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/knowledge-agent-harness/Cargo.toml`
- Create: `crates/knowledge-agent-harness/src/lib.rs`
- Create: `crates/knowledge-agent-harness/src/ask.rs`
- Create: `crates/knowledge-agent-harness/tests/ask_tests.rs`

- [ ] **Step 1: Write failing tests**

Create `crates/knowledge-agent-harness/tests/ask_tests.rs`:

```rust
use knowledge_agent_harness::{
    AskError, AskRequest, AskRunner, DeepSeekAskRunner, FakeAskRunner,
};

#[tokio::test]
async fn fake_runner_returns_configured_answer() {
    let runner = FakeAskRunner::new("fake answer");

    let response = runner
        .ask(AskRequest {
            message: "hello".to_string(),
        })
        .await
        .unwrap();

    assert_eq!(response.answer, "fake answer");
}

#[tokio::test]
async fn deepseek_runner_reports_missing_api_key() {
    let result = DeepSeekAskRunner::from_env_with(|name| {
        if name == "DEEPSEEK_MODEL" {
            Some("deepseek-v4-flash".to_string())
        } else {
            None
        }
    });

    assert!(matches!(result, Err(AskError::MissingApiKey)));
}
```

- [ ] **Step 2: Run tests to verify RED**

Run:

```bash
cargo test -p knowledge-agent-harness
```

Expected: fails because package does not exist.

- [ ] **Step 3: Add workspace dependencies**

Modify root `Cargo.toml`:

```toml
[workspace]
members = [
  "crates/knowledge-agent-cli",
  "crates/knowledge-agent-core",
  "crates/knowledge-agent-harness",
  "crates/knowledge-agent-server",
]
resolver = "2"

[workspace.dependencies]
anyhow = "1"
async-trait = "0.1"
axum = "0.8"
clap = { version = "4", features = ["derive"] }
gray_matter = "0.2"
llm-harness = { git = "https://github.com/oh-my-harness/llm-harness-core.git", package = "llm-harness" }
llm-harness-loop = { git = "https://github.com/oh-my-harness/llm-harness-core.git", package = "llm-harness-loop" }
llm_adapter = { git = "https://github.com/oh-my-harness/llm-api-adapter.git", rev = "c1d2cb87cb2bb94803144cfc28133a394b7fca18" }
pulldown-cmark = "0.13"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tempfile = "3"
thiserror = "2"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "net"] }
toml = "0.9"
walkdir = "2"
```

- [ ] **Step 4: Add crate manifest**

Create `crates/knowledge-agent-harness/Cargo.toml`:

```toml
[package]
name = "knowledge-agent-harness"
edition.workspace = true
license.workspace = true
version.workspace = true

[dependencies]
anyhow.workspace = true
async-trait.workspace = true
llm-harness.workspace = true
llm-harness-loop.workspace = true
llm_adapter.workspace = true
serde.workspace = true
thiserror.workspace = true
tokio.workspace = true
```

- [ ] **Step 5: Add public module**

Create `crates/knowledge-agent-harness/src/lib.rs`:

```rust
pub mod ask;

pub use ask::{
    AskError, AskRequest, AskResponse, AskRunner, DeepSeekAskRunner, FakeAskRunner,
};
```

- [ ] **Step 6: Implement ask runner**

Create `crates/knowledge-agent-harness/src/ask.rs`:

```rust
use std::sync::Arc;

use async_trait::async_trait;
use llm_adapter::deepseek;
use llm_harness::prelude::{Agent, AgentMessage, AgentOptions, ContentBlock};
use llm_harness_loop::LlmClient;
use serde::{Deserialize, Serialize};
use thiserror::Error;

const DEFAULT_DEEPSEEK_MODEL: &str = "deepseek-v4-flash";
const SYSTEM_PROMPT: &str = "你是 Knowledge Agent，一个本地 Obsidian 知识库研究助手。当前版本尚未接入 vault 检索，所以不要声称已经阅读用户的本地知识库。请用中文简洁回答；如果问题需要本地知识库上下文，请说明需要后续接入检索后才能准确回答。";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AskRequest {
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct AskResponse {
    pub answer: String,
}

#[derive(Debug, Error)]
pub enum AskError {
    #[error("missing DEEPSEEK_API_KEY")]
    MissingApiKey,
    #[error("llm returned no assistant text")]
    EmptyAnswer,
    #[error(transparent)]
    Harness(#[from] anyhow::Error),
}

#[async_trait]
pub trait AskRunner: Send + Sync {
    async fn ask(&self, request: AskRequest) -> Result<AskResponse, AskError>;
}

#[derive(Debug)]
pub struct FakeAskRunner {
    answer: String,
}

impl FakeAskRunner {
    pub fn new(answer: impl Into<String>) -> Self {
        Self {
            answer: answer.into(),
        }
    }
}

#[async_trait]
impl AskRunner for FakeAskRunner {
    async fn ask(&self, _request: AskRequest) -> Result<AskResponse, AskError> {
        Ok(AskResponse {
            answer: self.answer.clone(),
        })
    }
}

pub struct DeepSeekAskRunner {
    api_key: String,
    model: String,
}

impl DeepSeekAskRunner {
    pub fn from_env() -> Result<Self, AskError> {
        Self::from_env_with(|name| std::env::var(name).ok())
    }

    pub fn from_env_with(get_var: impl Fn(&str) -> Option<String>) -> Result<Self, AskError> {
        let api_key = get_var("DEEPSEEK_API_KEY")
            .filter(|value| !value.trim().is_empty())
            .ok_or(AskError::MissingApiKey)?;
        let model = get_var("DEEPSEEK_MODEL")
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_DEEPSEEK_MODEL.to_string());

        Ok(Self { api_key, model })
    }
}

#[async_trait]
impl AskRunner for DeepSeekAskRunner {
    async fn ask(&self, request: AskRequest) -> Result<AskResponse, AskError> {
        let client = Arc::new(deepseek::client(self.api_key.clone())) as Arc<dyn LlmClient>;
        let mut options = AgentOptions::new(self.model.clone());
        options.system_prompt = Some(SYSTEM_PROMPT.to_string());
        let agent = Agent::new(client, options);

        agent
            .prompt(request.message)
            .await
            .map_err(|err| AskError::Harness(anyhow::anyhow!(err)))?;

        let answer = assistant_text(&agent.state().messages);
        if answer.trim().is_empty() {
            return Err(AskError::EmptyAnswer);
        }

        Ok(AskResponse { answer })
    }
}

fn assistant_text(messages: &[AgentMessage]) -> String {
    let mut output = String::new();

    for message in messages {
        let AgentMessage::Assistant(assistant) = message else {
            continue;
        };

        for block in &assistant.content {
            if let ContentBlock::Text { text } = block {
                output.push_str(text);
            }
        }
    }

    output
}
```

- [ ] **Step 7: Run tests to verify GREEN**

Run:

```bash
cargo test -p knowledge-agent-harness
```

Expected: harness tests pass.

- [ ] **Step 8: Commit harness crate**

```bash
git add Cargo.toml Cargo.lock crates/knowledge-agent-harness
git commit -m "feat: add llm harness ask runner"
```

---

### Task 2: Inject Ask Runner Into Server

**Files:**
- Modify: `crates/knowledge-agent-server/Cargo.toml`
- Modify: `crates/knowledge-agent-server/src/state.rs`
- Modify: `crates/knowledge-agent-server/src/routes.rs`
- Modify: `crates/knowledge-agent-server/tests/server_tests.rs`

- [ ] **Step 1: Write failing server expectation**

Modify `ask_returns_placeholder_answer` in `crates/knowledge-agent-server/tests/server_tests.rs` to read the body and expect fake answer:

```rust
#[tokio::test]
async fn ask_returns_runner_answer() {
    let vault = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../knowledge-agent-core/tests/fixtures/basic-vault");
    let app = build_router(AppState::new_with_fake_ask_runner(vault, "fake llm answer"));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/ask")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"message":"什么是 Agent Harness？","mode":"vault"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["answer"], "fake llm answer");
}
```

- [ ] **Step 2: Run test to verify RED**

Run:

```bash
cargo test -p knowledge-agent-server ask_returns_runner_answer
```

Expected: fails because `AppState::new_with_fake_ask_runner` does not exist.

- [ ] **Step 3: Add dependency**

Modify `crates/knowledge-agent-server/Cargo.toml`:

```toml
knowledge-agent-harness = { path = "../knowledge-agent-harness" }
```

- [ ] **Step 4: Inject runner in AppState**

Modify `crates/knowledge-agent-server/src/state.rs`:

```rust
use std::{path::PathBuf, sync::Arc};

use knowledge_agent_harness::{AskRunner, DeepSeekAskRunner, FakeAskRunner};

#[derive(Clone)]
pub struct AppState {
    pub vault_root: Arc<PathBuf>,
    pub ask_runner: Arc<dyn AskRunner>,
}

impl AppState {
    pub fn new(vault_root: PathBuf) -> Self {
        let ask_runner: Arc<dyn AskRunner> = match DeepSeekAskRunner::from_env() {
            Ok(runner) => Arc::new(runner),
            Err(err) => Arc::new(FakeAskRunner::new(err.to_string())),
        };

        Self {
            vault_root: Arc::new(vault_root),
            ask_runner,
        }
    }

    pub fn new_with_ask_runner(vault_root: PathBuf, ask_runner: Arc<dyn AskRunner>) -> Self {
        Self {
            vault_root: Arc::new(vault_root),
            ask_runner,
        }
    }

    #[cfg(test)]
    pub fn new_with_fake_ask_runner(vault_root: PathBuf, answer: impl Into<String>) -> Self {
        Self::new_with_ask_runner(vault_root, Arc::new(FakeAskRunner::new(answer)))
    }
}
```

- [ ] **Step 5: Route ask through runner**

Modify `crates/knowledge-agent-server/src/routes.rs`:

```rust
use knowledge_agent_harness;
```

Update `ask` signature and body:

```rust
async fn ask(
    State(state): State<AppState>,
    Json(request): Json<AskRequest>,
) -> ApiResult<AskResponse> {
    if request.message.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "message cannot be empty".to_string(),
        ));
    }

    let answer = match request.mode {
        AskMode::Vault => state
            .ask_runner
            .ask(knowledge_agent_harness::AskRequest {
                message: request.message,
            })
            .await
            .map_err(internal_error)?
            .answer,
    };

    Ok(Json(AskResponse {
        answer,
        sources: Vec::new(),
        requires_followup: false,
    }))
}
```

- [ ] **Step 6: Run server tests**

Run:

```bash
cargo test -p knowledge-agent-server ask
```

Expected: ask tests pass.

- [ ] **Step 7: Commit server integration**

```bash
git add crates/knowledge-agent-server
git commit -m "feat: route ask through llm harness"
```

---

### Task 3: Use Explicit Missing-Config Error Instead Of Fake Default

**Files:**
- Modify: `crates/knowledge-agent-harness/src/ask.rs`
- Modify: `crates/knowledge-agent-harness/tests/ask_tests.rs`
- Modify: `crates/knowledge-agent-server/src/state.rs`

- [ ] **Step 1: Add failing unavailable runner test**

Append to `crates/knowledge-agent-harness/tests/ask_tests.rs`:

```rust
use knowledge_agent_harness::UnavailableAskRunner;

#[tokio::test]
async fn unavailable_runner_returns_its_error() {
    let runner = UnavailableAskRunner::new(AskError::MissingApiKey);

    let result = runner
        .ask(AskRequest {
            message: "hello".to_string(),
        })
        .await;

    assert!(matches!(result, Err(AskError::MissingApiKey)));
}
```

- [ ] **Step 2: Run test to verify RED**

Run:

```bash
cargo test -p knowledge-agent-harness unavailable_runner_returns_its_error
```

Expected: fails because `UnavailableAskRunner` does not exist.

- [ ] **Step 3: Implement unavailable runner**

Modify exports in `crates/knowledge-agent-harness/src/lib.rs`:

```rust
pub use ask::{
    AskError, AskRequest, AskResponse, AskRunner, DeepSeekAskRunner, FakeAskRunner,
    UnavailableAskRunner,
};
```

Append to `crates/knowledge-agent-harness/src/ask.rs`:

```rust
pub struct UnavailableAskRunner {
    error: AskError,
}

impl UnavailableAskRunner {
    pub fn new(error: AskError) -> Self {
        Self { error }
    }
}

#[async_trait]
impl AskRunner for UnavailableAskRunner {
    async fn ask(&self, _request: AskRequest) -> Result<AskResponse, AskError> {
        Err(self.error.clone())
    }
}
```

To support cloning, update `AskError`:

```rust
#[derive(Debug, Error, Clone)]
pub enum AskError {
    #[error("missing DEEPSEEK_API_KEY")]
    MissingApiKey,
    #[error("llm returned no assistant text")]
    EmptyAnswer,
    #[error("llm harness error: {0}")]
    Harness(String),
}
```

And update the DeepSeek error mapping:

```rust
.map_err(|err| AskError::Harness(err.to_string()))?;
```

- [ ] **Step 4: Use unavailable runner in AppState**

Modify `crates/knowledge-agent-server/src/state.rs` imports:

```rust
use knowledge_agent_harness::{AskRunner, DeepSeekAskRunner, FakeAskRunner, UnavailableAskRunner};
```

Update `AppState::new`:

```rust
let ask_runner: Arc<dyn AskRunner> = match DeepSeekAskRunner::from_env() {
    Ok(runner) => Arc::new(runner),
    Err(err) => Arc::new(UnavailableAskRunner::new(err)),
};
```

- [ ] **Step 5: Run tests**

Run:

```bash
cargo test -p knowledge-agent-harness
cargo test -p knowledge-agent-server ask
```

Expected: tests pass.

- [ ] **Step 6: Commit unavailable runner**

```bash
git add crates/knowledge-agent-harness crates/knowledge-agent-server/src/state.rs
git commit -m "fix: report missing llm configuration"
```

---

### Task 4: Document LLM Runtime Configuration

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Append Chinese LLM configuration docs**

Append to `README.md`:

```md
## LLM 配置

当前第一版 LLM 接入使用 `llm-harness-core` 的 DeepSeek 示例路径。启动后端前设置：

```powershell
$env:DEEPSEEK_API_KEY="sk-..."
$env:DEEPSEEK_MODEL="deepseek-v4-flash"
cargo run -p knowledge-agent-cli -- serve . --port 3030
```

`DEEPSEEK_MODEL` 可省略，默认使用 `deepseek-v4-flash`。

如果没有设置 `DEEPSEEK_API_KEY`，`/api/ask` 会返回明确错误。API key 属于个人配置，不应提交到 Git；后续会迁移到 `.knowledge-agent/local.toml`。
```

- [ ] **Step 2: Commit docs**

```bash
git add README.md
git commit -m "docs: document llm configuration"
```

---

### Task 5: Full Verification

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

- [ ] **Step 3: Optional real LLM smoke test**

Only run when `DEEPSEEK_API_KEY` is available in the environment:

```bash
cargo run -p knowledge-agent-cli -- serve crates/knowledge-agent-core/tests/fixtures/basic-vault --port 3030
```

Then ask from Web UI. Expected: answer comes from DeepSeek through `llm-harness-core`.

---

## Self-Review

- Spec coverage: crate integration in Task 1, server wiring in Task 2, missing config error in Task 3, README in Task 4, verification in Task 5.
- No vault retrieval, web search, streaming, sessions, provider registry, or local TOML writes are included.
- Tests use fake/unavailable runners and do not call external LLMs by default.
