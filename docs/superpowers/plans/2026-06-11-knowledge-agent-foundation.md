# Knowledge Agent Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 构建第一阶段可运行基础：`knowledge-agent serve .` 能启动本地服务，扫描 Obsidian vault，解析 Markdown/frontmatter/wikilink/index，构建链接图，执行手动维护扫描，并通过统一写入策略保护 vault。

**Architecture:** 使用 Rust workspace 分成 CLI、server、core 三层。`knowledge-agent-core` 只负责 vault 语义、索引、图谱、diff、维护检查和写入策略；`knowledge-agent-server` 暴露 axum HTTP API；`knowledge-agent-cli` 解析命令并启动服务。第一阶段不接真实 LLM、不做 Web UI、不做网页搜索，但所有接口为后续 Research 与 Harness 集成留出稳定边界。

**Tech Stack:** Rust 1.95、Cargo workspace、clap、axum、tokio、serde、serde_json、toml、walkdir、gray_matter、pulldown-cmark、tempfile。

---

## Scope

本计划只实现 foundation。它不实现完整 Research agent、`llm-harness-core` adapter、React Web UI、网页搜索 provider，也不生成研究报告。它产出的软件必须能：

- 从 vault 根目录运行 `cargo run -p knowledge-agent-cli -- serve . --port 0`。
- 通过 HTTP 返回 vault 概览。
- 扫描 Markdown 文件和 `_index.md`。
- 解析 frontmatter、标题、tags、wikilinks。
- 构建 links/backlinks 图。
- 手动运行维护扫描。
- 对低风险写入使用 `VaultWritePolicy`。

## File Structure

```text
Cargo.toml
crates/
  knowledge-agent-cli/
    Cargo.toml
    src/main.rs
  knowledge-agent-server/
    Cargo.toml
    src/lib.rs
    src/routes.rs
    src/state.rs
  knowledge-agent-core/
    Cargo.toml
    src/lib.rs
    src/vault/mod.rs
    src/vault/frontmatter.rs
    src/vault/wikilinks.rs
    src/vault/scanner.rs
    src/vault/index_docs.rs
    src/vault/graph.rs
    src/vault/policy.rs
    src/maintenance/mod.rs
    src/maintenance/checks.rs
    src/maintenance/inbox.rs
    src/settings.rs
    tests/fixtures/basic-vault/
      README.md
      .knowledge-agent.toml
      docs/_index.md
      docs/concepts/_index.md
      docs/concepts/agent-harness.md
      docs/research/_index.md
```

Responsibilities:

- `knowledge-agent-cli`: CLI argument parsing and process startup only.
- `knowledge-agent-server`: HTTP routing, app state, JSON response types.
- `knowledge-agent-core::vault`: all filesystem and Markdown knowledge behavior.
- `knowledge-agent-core::maintenance`: scan results and low-risk fix proposals.
- `knowledge-agent-core::settings`: shared `.knowledge-agent.toml` loading.

---

### Task 1: Create Rust Workspace Skeleton

**Files:**
- Create: `Cargo.toml`
- Create: `crates/knowledge-agent-core/Cargo.toml`
- Create: `crates/knowledge-agent-core/src/lib.rs`
- Create: `crates/knowledge-agent-cli/Cargo.toml`
- Create: `crates/knowledge-agent-cli/src/main.rs`
- Create: `crates/knowledge-agent-server/Cargo.toml`
- Create: `crates/knowledge-agent-server/src/lib.rs`

- [ ] **Step 1: Add root workspace manifest**

Create `Cargo.toml`:

```toml
[workspace]
members = [
  "crates/knowledge-agent-cli",
  "crates/knowledge-agent-core",
  "crates/knowledge-agent-server",
]
resolver = "2"

[workspace.package]
edition = "2024"
license = "MIT"
version = "0.1.0"

[workspace.dependencies]
anyhow = "1"
axum = "0.8"
clap = { version = "4", features = ["derive"] }
gray_matter = "0.2"
pulldown-cmark = "0.13"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tempfile = "3"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "net"] }
toml = "0.9"
walkdir = "2"
```

- [ ] **Step 2: Add core crate**

Create `crates/knowledge-agent-core/Cargo.toml`:

```toml
[package]
name = "knowledge-agent-core"
edition.workspace = true
license.workspace = true
version.workspace = true

[dependencies]
anyhow.workspace = true
gray_matter.workspace = true
pulldown-cmark.workspace = true
serde.workspace = true
toml.workspace = true
walkdir.workspace = true

[dev-dependencies]
tempfile.workspace = true
```

Create `crates/knowledge-agent-core/src/lib.rs`:

```rust
pub mod maintenance;
pub mod settings;
pub mod vault;
```

- [ ] **Step 3: Add server crate**

Create `crates/knowledge-agent-server/Cargo.toml`:

```toml
[package]
name = "knowledge-agent-server"
edition.workspace = true
license.workspace = true
version.workspace = true

[dependencies]
anyhow.workspace = true
axum.workspace = true
knowledge-agent-core = { path = "../knowledge-agent-core" }
serde.workspace = true
serde_json.workspace = true
tokio.workspace = true
```

Create `crates/knowledge-agent-server/src/lib.rs`:

```rust
pub mod routes;
pub mod state;
```

- [ ] **Step 4: Add CLI crate**

Create `crates/knowledge-agent-cli/Cargo.toml`:

```toml
[package]
name = "knowledge-agent-cli"
edition.workspace = true
license.workspace = true
version.workspace = true

[[bin]]
name = "knowledge-agent"
path = "src/main.rs"

[dependencies]
anyhow.workspace = true
clap.workspace = true
knowledge-agent-server = { path = "../knowledge-agent-server" }
tokio.workspace = true
```

Create `crates/knowledge-agent-cli/src/main.rs`:

```rust
use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "knowledge-agent")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Serve {
        vault: PathBuf,
        #[arg(long, default_value_t = 3030)]
        port: u16,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Serve { vault, port } => {
            knowledge_agent_server::serve(vault, port).await?;
        }
    }
    Ok(())
}
```

- [ ] **Step 5: Run workspace check**

Run:

```bash
cargo check
```

Expected: FAIL because `routes`, `state`, `maintenance`, `settings`, `vault`, and `serve` are not implemented yet.

- [ ] **Step 6: Commit skeleton**

```bash
git add Cargo.toml crates
git commit -m "chore: scaffold rust workspace"
```

---

### Task 2: Add Settings Loader And Fixture Vault

**Files:**
- Create: `crates/knowledge-agent-core/src/settings.rs`
- Create: `crates/knowledge-agent-core/tests/fixtures/basic-vault/README.md`
- Create: `crates/knowledge-agent-core/tests/fixtures/basic-vault/.knowledge-agent.toml`
- Create: `crates/knowledge-agent-core/tests/settings_tests.rs`

- [ ] **Step 1: Write failing settings tests**

Create `crates/knowledge-agent-core/tests/settings_tests.rs`:

```rust
use knowledge_agent_core::settings::{VaultSettings, load_vault_settings};
use std::path::Path;

#[test]
fn loads_shared_vault_settings() {
    let vault = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/basic-vault");

    let settings = load_vault_settings(&vault).expect("settings should load");

    assert_eq!(settings.docs_dir, "docs");
    assert_eq!(settings.research_dir, "docs/research");
    assert_eq!(settings.concepts_dir, "docs/concepts");
    assert_eq!(settings.required_frontmatter, vec!["title", "type", "created", "updated"]);
}

#[test]
fn defaults_when_shared_settings_missing() {
    let temp = tempfile::tempdir().expect("tempdir");

    let settings = load_vault_settings(temp.path()).expect("defaults should load");

    assert_eq!(settings, VaultSettings::default());
}
```

- [ ] **Step 2: Add fixture vault files**

Create `crates/knowledge-agent-core/tests/fixtures/basic-vault/README.md`:

```md
# Basic Vault

This fixture is an Obsidian-compatible vault for tests.
```

Create `crates/knowledge-agent-core/tests/fixtures/basic-vault/.knowledge-agent.toml`:

```toml
docs_dir = "docs"
research_dir = "docs/research"
concepts_dir = "docs/concepts"
index_file_name = "_index.md"
required_frontmatter = ["title", "type", "created", "updated"]
```

- [ ] **Step 3: Run test to verify it fails**

Run:

```bash
cargo test -p knowledge-agent-core settings_tests -- --nocapture
```

Expected: FAIL with unresolved import `knowledge_agent_core::settings`.

- [ ] **Step 4: Implement settings loader**

Create `crates/knowledge-agent-core/src/settings.rs`:

```rust
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VaultSettings {
    pub docs_dir: String,
    pub research_dir: String,
    pub concepts_dir: String,
    pub index_file_name: String,
    pub required_frontmatter: Vec<String>,
}

impl Default for VaultSettings {
    fn default() -> Self {
        Self {
            docs_dir: "docs".to_string(),
            research_dir: "docs/research".to_string(),
            concepts_dir: "docs/concepts".to_string(),
            index_file_name: "_index.md".to_string(),
            required_frontmatter: vec![
                "title".to_string(),
                "type".to_string(),
                "created".to_string(),
                "updated".to_string(),
            ],
        }
    }
}

pub fn load_vault_settings(vault_root: &Path) -> Result<VaultSettings> {
    let path = vault_root.join(".knowledge-agent.toml");
    if !path.exists() {
        return Ok(VaultSettings::default());
    }

    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let settings = toml::from_str::<VaultSettings>(&raw)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(settings)
}
```

- [ ] **Step 5: Run test to verify it passes**

Run:

```bash
cargo test -p knowledge-agent-core settings_tests -- --nocapture
```

Expected: PASS, 2 tests.

- [ ] **Step 6: Commit settings loader**

```bash
git add crates/knowledge-agent-core
git commit -m "feat: load vault settings"
```

---

### Task 3: Parse Frontmatter And Wikilinks

**Files:**
- Create: `crates/knowledge-agent-core/src/vault/mod.rs`
- Create: `crates/knowledge-agent-core/src/vault/frontmatter.rs`
- Create: `crates/knowledge-agent-core/src/vault/wikilinks.rs`
- Create: `crates/knowledge-agent-core/tests/markdown_parse_tests.rs`

- [ ] **Step 1: Write failing parser tests**

Create `crates/knowledge-agent-core/tests/markdown_parse_tests.rs`:

```rust
use knowledge_agent_core::vault::{
    frontmatter::parse_markdown_note,
    wikilinks::{WikiLink, extract_wikilinks},
};

#[test]
fn parses_yaml_frontmatter_and_body() {
    let raw = r#"---
title: Agent Harness
type: concept
tags: [agent, runtime]
created: 2026-06-11
updated: 2026-06-11
---

# Agent Harness

Links to [[LLM Harness|runtime harness]].
"#;

    let note = parse_markdown_note(raw).expect("note parses");

    assert_eq!(note.title.as_deref(), Some("Agent Harness"));
    assert_eq!(note.note_type.as_deref(), Some("concept"));
    assert_eq!(note.tags, vec!["agent", "runtime"]);
    assert!(note.body.contains("# Agent Harness"));
}

#[test]
fn extracts_wikilinks_with_aliases() {
    let links = extract_wikilinks("See [[Agent Harness]] and [[LLM Harness|runtime harness]].");

    assert_eq!(
        links,
        vec![
            WikiLink { target: "Agent Harness".to_string(), alias: None },
            WikiLink { target: "LLM Harness".to_string(), alias: Some("runtime harness".to_string()) },
        ]
    );
}
```

- [ ] **Step 2: Run parser tests to verify they fail**

Run:

```bash
cargo test -p knowledge-agent-core markdown_parse_tests -- --nocapture
```

Expected: FAIL with unresolved `vault::frontmatter` and `vault::wikilinks`.

- [ ] **Step 3: Add vault module**

Create `crates/knowledge-agent-core/src/vault/mod.rs`:

```rust
pub mod frontmatter;
pub mod graph;
pub mod index_docs;
pub mod policy;
pub mod scanner;
pub mod wikilinks;
```

- [ ] **Step 4: Implement frontmatter parser**

Create `crates/knowledge-agent-core/src/vault/frontmatter.rs`:

```rust
use anyhow::Result;
use gray_matter::{Matter, engine::YAML};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedNote {
    pub title: Option<String>,
    pub note_type: Option<String>,
    pub tags: Vec<String>,
    pub body: String,
}

#[derive(Debug, Default, Deserialize)]
struct RawFrontmatter {
    title: Option<String>,
    #[serde(rename = "type")]
    note_type: Option<String>,
    tags: Option<Vec<String>>,
}

pub fn parse_markdown_note(raw: &str) -> Result<ParsedNote> {
    let matter = Matter::<YAML>::new();
    let parsed = matter.parse(raw);
    let data = parsed
        .data
        .and_then(|value| value.deserialize::<RawFrontmatter>().ok())
        .unwrap_or_default();

    Ok(ParsedNote {
        title: data.title,
        note_type: data.note_type,
        tags: data.tags.unwrap_or_default(),
        body: parsed.content,
    })
}
```

- [ ] **Step 5: Implement wikilink parser**

Create `crates/knowledge-agent-core/src/vault/wikilinks.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WikiLink {
    pub target: String,
    pub alias: Option<String>,
}

pub fn extract_wikilinks(markdown: &str) -> Vec<WikiLink> {
    let mut links = Vec::new();
    let mut rest = markdown;

    while let Some(start) = rest.find("[[") {
        let after_start = &rest[start + 2..];
        let Some(end) = after_start.find("]]") else {
            break;
        };
        let inner = &after_start[..end];
        let mut parts = inner.splitn(2, '|');
        let target = parts.next().unwrap_or("").trim();
        let alias = parts.next().map(str::trim).filter(|s| !s.is_empty());

        if !target.is_empty() {
            links.push(WikiLink {
                target: target.to_string(),
                alias: alias.map(str::to_string),
            });
        }

        rest = &after_start[end + 2..];
    }

    links
}
```

- [ ] **Step 6: Run parser tests to verify they pass**

Run:

```bash
cargo test -p knowledge-agent-core markdown_parse_tests -- --nocapture
```

Expected: PASS, 2 tests.

- [ ] **Step 7: Commit parser modules**

```bash
git add crates/knowledge-agent-core/src/vault crates/knowledge-agent-core/tests/markdown_parse_tests.rs
git commit -m "feat: parse markdown frontmatter and wikilinks"
```

---

### Task 4: Scan Vault Markdown Files

**Files:**
- Create: `crates/knowledge-agent-core/src/vault/scanner.rs`
- Modify: `crates/knowledge-agent-core/tests/fixtures/basic-vault/docs/concepts/agent-harness.md`
- Create: `crates/knowledge-agent-core/tests/vault_scanner_tests.rs`

- [ ] **Step 1: Add fixture note**

Create `crates/knowledge-agent-core/tests/fixtures/basic-vault/docs/concepts/agent-harness.md`:

```md
---
title: Agent Harness
type: concept
tags: [agent, runtime]
created: 2026-06-11
updated: 2026-06-11
---

# Agent Harness

An agent harness coordinates messages, tools, and sessions.

See [[LLM Harness]].
```

- [ ] **Step 2: Write failing scanner test**

Create `crates/knowledge-agent-core/tests/vault_scanner_tests.rs`:

```rust
use knowledge_agent_core::vault::scanner::scan_vault;
use std::path::Path;

#[test]
fn scans_markdown_notes_relative_to_vault_root() {
    let vault = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/basic-vault");

    let scan = scan_vault(&vault).expect("vault scans");

    let note = scan
        .notes
        .iter()
        .find(|note| note.relative_path == "docs/concepts/agent-harness.md")
        .expect("concept note exists");

    assert_eq!(note.title.as_deref(), Some("Agent Harness"));
    assert_eq!(note.tags, vec!["agent", "runtime"]);
    assert_eq!(note.links[0].target, "LLM Harness");
}
```

- [ ] **Step 3: Run scanner test to verify it fails**

Run:

```bash
cargo test -p knowledge-agent-core vault_scanner_tests -- --nocapture
```

Expected: FAIL with unresolved `scan_vault`.

- [ ] **Step 4: Implement scanner**

Create `crates/knowledge-agent-core/src/vault/scanner.rs`:

```rust
use crate::vault::{
    frontmatter::parse_markdown_note,
    wikilinks::{WikiLink, extract_wikilinks},
};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VaultScan {
    pub root: PathBuf,
    pub notes: Vec<ScannedNote>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScannedNote {
    pub relative_path: String,
    pub title: Option<String>,
    pub note_type: Option<String>,
    pub tags: Vec<String>,
    pub links: Vec<WikiLink>,
}

pub fn scan_vault(vault_root: &Path) -> Result<VaultScan> {
    let mut notes = Vec::new();

    for entry in WalkDir::new(vault_root)
        .into_iter()
        .filter_entry(|entry| entry.file_name() != ".git" && entry.file_name() != ".knowledge-agent")
    {
        let entry = entry?;
        if !entry.file_type().is_file() || entry.path().extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }

        let raw = std::fs::read_to_string(entry.path())
            .with_context(|| format!("failed to read {}", entry.path().display()))?;
        let parsed = parse_markdown_note(&raw)?;
        let relative_path = entry
            .path()
            .strip_prefix(vault_root)?
            .to_string_lossy()
            .replace('\\', "/");

        notes.push(ScannedNote {
            relative_path,
            title: parsed.title,
            note_type: parsed.note_type,
            tags: parsed.tags,
            links: extract_wikilinks(&raw),
        });
    }

    notes.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

    Ok(VaultScan {
        root: vault_root.to_path_buf(),
        notes,
    })
}
```

- [ ] **Step 5: Run scanner test to verify it passes**

Run:

```bash
cargo test -p knowledge-agent-core vault_scanner_tests -- --nocapture
```

Expected: PASS, 1 test.

- [ ] **Step 6: Commit scanner**

```bash
git add crates/knowledge-agent-core/src/vault/scanner.rs crates/knowledge-agent-core/tests
git commit -m "feat: scan vault markdown notes"
```

---

### Task 5: Parse Markdown Index Documents And Build Link Graph

**Files:**
- Create: `crates/knowledge-agent-core/src/vault/index_docs.rs`
- Create: `crates/knowledge-agent-core/src/vault/graph.rs`
- Create: `crates/knowledge-agent-core/tests/fixtures/basic-vault/docs/_index.md`
- Create: `crates/knowledge-agent-core/tests/fixtures/basic-vault/docs/concepts/_index.md`
- Create: `crates/knowledge-agent-core/tests/index_graph_tests.rs`

- [ ] **Step 1: Add fixture index documents**

Create `crates/knowledge-agent-core/tests/fixtures/basic-vault/docs/_index.md`:

```md
---
title: Docs Index
type: index
scope: docs
created: 2026-06-11
updated: 2026-06-11
---

# Docs Index

## Concepts

- [[agent-harness]]
```

Create `crates/knowledge-agent-core/tests/fixtures/basic-vault/docs/concepts/_index.md`:

```md
---
title: Concepts Index
type: index
scope: concepts
created: 2026-06-11
updated: 2026-06-11
---

# Concepts

## Agent Harness

- [[agent-harness]]
- tags: #agent #runtime
- summary: Agent runtime, tools, and sessions.
```

- [ ] **Step 2: Write failing index and graph tests**

Create `crates/knowledge-agent-core/tests/index_graph_tests.rs`:

```rust
use knowledge_agent_core::vault::{
    graph::build_link_graph,
    index_docs::parse_index_document,
    scanner::scan_vault,
};
use std::path::Path;

#[test]
fn parses_index_links_and_summaries() {
    let raw = include_str!("fixtures/basic-vault/docs/concepts/_index.md");

    let index = parse_index_document("docs/concepts/_index.md", raw).expect("index parses");

    assert_eq!(index.path, "docs/concepts/_index.md");
    assert_eq!(index.entries.len(), 1);
    assert_eq!(index.entries[0].title, "Agent Harness");
    assert_eq!(index.entries[0].links[0].target, "agent-harness");
    assert_eq!(index.entries[0].summary.as_deref(), Some("Agent runtime, tools, and sessions."));
}

#[test]
fn builds_backlinks_from_scan() {
    let vault = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/basic-vault");
    let scan = scan_vault(&vault).expect("vault scans");

    let graph = build_link_graph(&scan);

    let backlinks = graph.backlinks_to("LLM Harness");
    assert_eq!(backlinks, vec!["docs/concepts/agent-harness.md"]);
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run:

```bash
cargo test -p knowledge-agent-core index_graph_tests -- --nocapture
```

Expected: FAIL with unresolved `parse_index_document` and `build_link_graph`.

- [ ] **Step 4: Implement index parser**

Create `crates/knowledge-agent-core/src/vault/index_docs.rs`:

```rust
use crate::vault::wikilinks::{WikiLink, extract_wikilinks};
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexDocument {
    pub path: String,
    pub entries: Vec<IndexEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexEntry {
    pub title: String,
    pub links: Vec<WikiLink>,
    pub summary: Option<String>,
}

pub fn parse_index_document(path: &str, raw: &str) -> Result<IndexDocument> {
    let mut entries = Vec::new();
    let mut current: Option<IndexEntry> = None;

    for line in raw.lines() {
        if let Some(title) = line.strip_prefix("## ") {
            if let Some(entry) = current.take() {
                entries.push(entry);
            }
            current = Some(IndexEntry {
                title: title.trim().to_string(),
                links: Vec::new(),
                summary: None,
            });
            continue;
        }

        if let Some(entry) = current.as_mut() {
            entry.links.extend(extract_wikilinks(line));
            if let Some(summary) = line.trim().strip_prefix("- summary:") {
                entry.summary = Some(summary.trim().to_string());
            }
        }
    }

    if let Some(entry) = current {
        entries.push(entry);
    }

    Ok(IndexDocument {
        path: path.to_string(),
        entries,
    })
}
```

- [ ] **Step 5: Implement graph builder**

Create `crates/knowledge-agent-core/src/vault/graph.rs`:

```rust
use crate::vault::scanner::VaultScan;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkGraph {
    backlinks: BTreeMap<String, Vec<String>>,
}

impl LinkGraph {
    pub fn backlinks_to(&self, target: &str) -> Vec<&str> {
        self.backlinks
            .get(target)
            .map(|paths| paths.iter().map(String::as_str).collect())
            .unwrap_or_default()
    }
}

pub fn build_link_graph(scan: &VaultScan) -> LinkGraph {
    let mut backlinks: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for note in &scan.notes {
        for link in &note.links {
            backlinks
                .entry(link.target.clone())
                .or_default()
                .push(note.relative_path.clone());
        }
    }

    for paths in backlinks.values_mut() {
        paths.sort();
        paths.dedup();
    }

    LinkGraph { backlinks }
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run:

```bash
cargo test -p knowledge-agent-core index_graph_tests -- --nocapture
```

Expected: PASS, 2 tests.

- [ ] **Step 7: Commit index and graph**

```bash
git add crates/knowledge-agent-core/src/vault crates/knowledge-agent-core/tests
git commit -m "feat: parse markdown indexes and backlinks"
```

---

### Task 6: Add Write Policy And Maintenance Checks

**Files:**
- Create: `crates/knowledge-agent-core/src/vault/policy.rs`
- Create: `crates/knowledge-agent-core/src/maintenance/mod.rs`
- Create: `crates/knowledge-agent-core/src/maintenance/inbox.rs`
- Create: `crates/knowledge-agent-core/src/maintenance/checks.rs`
- Create: `crates/knowledge-agent-core/tests/maintenance_tests.rs`

- [ ] **Step 1: Write failing maintenance tests**

Create `crates/knowledge-agent-core/tests/maintenance_tests.rs`:

```rust
use knowledge_agent_core::{
    maintenance::checks::run_maintenance_scan,
    vault::policy::{VaultWriteOperation, VaultWritePolicy, WriteDecision},
};
use std::path::Path;

#[test]
fn reports_broken_wikilinks() {
    let vault = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/basic-vault");

    let inbox = run_maintenance_scan(&vault).expect("scan succeeds");

    assert!(inbox.items.iter().any(|item| {
        item.kind == "broken_wikilink"
            && item.file == "docs/concepts/agent-harness.md"
            && item.evidence.contains("LLM Harness")
    }));
}

#[test]
fn write_policy_allows_only_low_risk_automatic_writes() {
    let policy = VaultWritePolicy::default();

    assert_eq!(
        policy.decide(&VaultWriteOperation::AddIndexEntry {
            index_path: "docs/concepts/_index.md".to_string(),
            target_path: "docs/concepts/agent-harness.md".to_string(),
        }),
        WriteDecision::AllowAutomatic
    );

    assert_eq!(
        policy.decide(&VaultWriteOperation::ModifyBodyMeaning {
            path: "docs/concepts/agent-harness.md".to_string(),
        }),
        WriteDecision::RequireConfirmation
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test -p knowledge-agent-core maintenance_tests -- --nocapture
```

Expected: FAIL with unresolved maintenance and policy modules.

- [ ] **Step 3: Implement write policy**

Create `crates/knowledge-agent-core/src/vault/policy.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VaultWriteOperation {
    AddIndexEntry { index_path: String, target_path: String },
    UpdateFrontmatterField { path: String, field: String },
    MarkNonSemanticMetadata { path: String, field: String },
    ModifyBodyMeaning { path: String },
    DeleteNote { path: String },
    MoveNote { from: String, to: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteDecision {
    AllowAutomatic,
    RequireConfirmation,
}

#[derive(Debug, Default)]
pub struct VaultWritePolicy;

impl VaultWritePolicy {
    pub fn decide(&self, operation: &VaultWriteOperation) -> WriteDecision {
        match operation {
            VaultWriteOperation::AddIndexEntry { .. }
            | VaultWriteOperation::UpdateFrontmatterField { .. }
            | VaultWriteOperation::MarkNonSemanticMetadata { .. } => WriteDecision::AllowAutomatic,
            VaultWriteOperation::ModifyBodyMeaning { .. }
            | VaultWriteOperation::DeleteNote { .. }
            | VaultWriteOperation::MoveNote { .. } => WriteDecision::RequireConfirmation,
        }
    }
}
```

- [ ] **Step 4: Implement maintenance inbox types**

Create `crates/knowledge-agent-core/src/maintenance/mod.rs`:

```rust
pub mod checks;
pub mod inbox;
```

Create `crates/knowledge-agent-core/src/maintenance/inbox.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaintenanceInbox {
    pub items: Vec<MaintenanceItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaintenanceItem {
    pub priority: String,
    pub kind: String,
    pub file: String,
    pub evidence: String,
    pub requires_confirmation: bool,
}
```

- [ ] **Step 5: Implement broken wikilink check**

Create `crates/knowledge-agent-core/src/maintenance/checks.rs`:

```rust
use crate::{
    maintenance::inbox::{MaintenanceInbox, MaintenanceItem},
    vault::scanner::scan_vault,
};
use anyhow::Result;
use std::{collections::BTreeSet, path::Path};

pub fn run_maintenance_scan(vault_root: &Path) -> Result<MaintenanceInbox> {
    let scan = scan_vault(vault_root)?;
    let known_titles = scan
        .notes
        .iter()
        .filter_map(|note| note.title.clone())
        .collect::<BTreeSet<_>>();
    let known_stems = scan
        .notes
        .iter()
        .filter_map(|note| {
            note.relative_path
                .rsplit('/')
                .next()
                .and_then(|name| name.strip_suffix(".md"))
                .map(str::to_string)
        })
        .collect::<BTreeSet<_>>();

    let mut items = Vec::new();

    for note in &scan.notes {
        for link in &note.links {
            if !known_titles.contains(&link.target) && !known_stems.contains(&link.target) {
                items.push(MaintenanceItem {
                    priority: "P0".to_string(),
                    kind: "broken_wikilink".to_string(),
                    file: note.relative_path.clone(),
                    evidence: format!("Missing target [[{}]]", link.target),
                    requires_confirmation: false,
                });
            }
        }
    }

    Ok(MaintenanceInbox { items })
}
```

- [ ] **Step 6: Run maintenance tests to verify they pass**

Run:

```bash
cargo test -p knowledge-agent-core maintenance_tests -- --nocapture
```

Expected: PASS, 2 tests.

- [ ] **Step 7: Commit maintenance foundation**

```bash
git add crates/knowledge-agent-core/src crates/knowledge-agent-core/tests/maintenance_tests.rs
git commit -m "feat: add maintenance scan and write policy"
```

---

### Task 7: Expose Local HTTP API

**Files:**
- Create: `crates/knowledge-agent-server/src/state.rs`
- Create: `crates/knowledge-agent-server/src/routes.rs`
- Modify: `crates/knowledge-agent-server/src/lib.rs`
- Create: `crates/knowledge-agent-server/tests/server_tests.rs`

- [ ] **Step 1: Write failing server tests**

Create `crates/knowledge-agent-server/tests/server_tests.rs`:

```rust
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use knowledge_agent_server::{AppState, build_router};
use std::path::Path;
use tower::ServiceExt;

#[tokio::test]
async fn health_returns_ok() {
    let vault = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../knowledge-agent-core/tests/fixtures/basic-vault");
    let app = build_router(AppState::new(vault));

    let response = app
        .oneshot(Request::builder().uri("/api/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn maintenance_scan_returns_json() {
    let vault = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../knowledge-agent-core/tests/fixtures/basic-vault");
    let app = build_router(AppState::new(vault));

    let response = app
        .oneshot(Request::builder().method("POST").uri("/api/maintenance/scan").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
```

- [ ] **Step 2: Add server test dependencies**

Modify `crates/knowledge-agent-server/Cargo.toml`:

```toml
[dev-dependencies]
tower = { version = "0.5", features = ["util"] }
```

- [ ] **Step 3: Run server tests to verify they fail**

Run:

```bash
cargo test -p knowledge-agent-server -- --nocapture
```

Expected: FAIL with unresolved `AppState` and `build_router`.

- [ ] **Step 4: Implement app state**

Create `crates/knowledge-agent-server/src/state.rs`:

```rust
use std::{path::PathBuf, sync::Arc};

#[derive(Debug, Clone)]
pub struct AppState {
    pub vault_root: Arc<PathBuf>,
}

impl AppState {
    pub fn new(vault_root: PathBuf) -> Self {
        Self {
            vault_root: Arc::new(vault_root),
        }
    }
}
```

- [ ] **Step 5: Implement routes**

Create `crates/knowledge-agent-server/src/routes.rs`:

```rust
use crate::state::AppState;
use axum::{Json, Router, extract::State, routing::{get, post}};
use knowledge_agent_core::{maintenance::checks::run_maintenance_scan, vault::scanner::scan_vault};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/vault/index", get(vault_index))
        .route("/api/maintenance/scan", post(maintenance_scan))
        .with_state(state)
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

async fn vault_index(State(state): State<AppState>) -> Result<Json<impl Serialize>, String> {
    scan_vault(&state.vault_root).map(Json).map_err(|err| err.to_string())
}

async fn maintenance_scan(State(state): State<AppState>) -> Result<Json<impl Serialize>, String> {
    run_maintenance_scan(&state.vault_root)
        .map(Json)
        .map_err(|err| err.to_string())
}
```

- [ ] **Step 6: Export server functions and implement serve**

Modify `crates/knowledge-agent-server/src/lib.rs`:

```rust
pub mod routes;
pub mod state;

use anyhow::Result;
use std::{net::SocketAddr, path::PathBuf};
use tokio::net::TcpListener;

pub use routes::build_router;
pub use state::AppState;

pub async fn serve(vault_root: PathBuf, port: u16) -> Result<()> {
    let app = build_router(AppState::new(vault_root));
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = TcpListener::bind(addr).await?;
    let local_addr = listener.local_addr()?;
    println!("knowledge-agent listening on http://{}", local_addr);
    axum::serve(listener, app).await?;
    Ok(())
}
```

- [ ] **Step 7: Run server tests to verify they pass**

Run:

```bash
cargo test -p knowledge-agent-server -- --nocapture
```

Expected: PASS, 2 tests.

- [ ] **Step 8: Commit server API**

```bash
git add crates/knowledge-agent-server
git commit -m "feat: expose vault and maintenance api"
```

---

### Task 8: Verify CLI End To End

**Files:**
- Modify: `README.md`
- Modify: `.gitignore`

- [ ] **Step 1: Add local runtime ignores**

Modify `.gitignore` so it contains:

```gitignore
.superpowers/
.knowledge-agent/
target/
node_modules/
dist/
```

- [ ] **Step 2: Add project README**

Create or modify `README.md`:

```md
# Knowledge Agent

Knowledge Agent is a local research assistant and Obsidian vault maintenance tool.

## Foundation Command

Run from an Obsidian vault root:

```bash
knowledge-agent serve .
```

During development, run against the fixture vault:

```bash
cargo run -p knowledge-agent-cli -- serve crates/knowledge-agent-core/tests/fixtures/basic-vault --port 3030
```

Available foundation endpoints:

- `GET /api/health`
- `GET /api/vault/index`
- `POST /api/maintenance/scan`
```

- [ ] **Step 3: Run all Rust tests**

Run:

```bash
cargo test
```

Expected: PASS for all workspace tests.

- [ ] **Step 4: Run CLI server smoke test**

Run:

```bash
cargo run -p knowledge-agent-cli -- serve crates/knowledge-agent-core/tests/fixtures/basic-vault --port 3030
```

Expected stdout:

```text
knowledge-agent listening on http://127.0.0.1:3030
```

In another terminal, run:

```bash
curl http://127.0.0.1:3030/api/health
curl -X POST http://127.0.0.1:3030/api/maintenance/scan
```

Expected health response:

```json
{"status":"ok"}
```

Expected maintenance response contains:

```json
"kind":"broken_wikilink"
```

- [ ] **Step 5: Format and lint**

Run:

```bash
cargo fmt
cargo clippy --workspace --all-targets -- -D warnings
```

Expected: both commands pass.

- [ ] **Step 6: Commit foundation docs and verification**

```bash
git add .gitignore README.md
git commit -m "docs: document foundation server"
```

---

## Self-Review

Spec coverage:

- `knowledge-agent serve .`: Task 1 and Task 7.
- Vault settings split: Task 2 and Task 8.
- Markdown/frontmatter/wikilinks: Task 3.
- Vault scan and Markdown facts: Task 4.
- Markdown index and backlinks: Task 5.
- Manual maintenance scan: Task 6 and Task 7.
- Write policy safety boundary: Task 6.
- HTTP API foundation: Task 7.
- Local runtime ignores: Task 8.

Known exclusions for later plans:

- Research project generation.
- Concept card preview/write workflow.
- Ask Vault LLM answering.
- `llm-harness-core` adapter.
- Web frontend.
- Web search provider.
