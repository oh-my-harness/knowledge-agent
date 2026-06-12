use std::{
    path::{Component, Path, PathBuf},
    sync::Arc,
};

use futures::future::BoxFuture;
use knowledge_agent_core::vault::{
    graph::build_link_graph,
    policy::{VaultWriteOperation, VaultWritePolicy, WriteDecision},
    scanner::scan_vault,
};
use llm_harness::prelude::{ContentBlock, Tool, ToolContext, ToolError, ToolResult};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::io::AsyncWriteExt;

const MAX_NOTE_CHARS: usize = 24_000;
const MAX_SEARCH_RESULTS: usize = 20;
const MAX_SNIPPET_CHARS: usize = 240;

pub fn vault_read_tools(vault_root: impl Into<PathBuf>) -> Vec<Arc<dyn Tool>> {
    let vault_root = Arc::new(vault_root.into());
    vec![
        Arc::new(ListNotesTool::new(vault_root.clone())),
        Arc::new(ReadNoteTool::new(vault_root.clone())),
        Arc::new(SearchNotesTool::new(vault_root.clone())),
        Arc::new(NeighborNotesTool::new(vault_root)),
    ]
}

pub fn vault_edit_tools(vault_root: impl Into<PathBuf>) -> Vec<Arc<dyn Tool>> {
    let vault_root = Arc::new(vault_root.into());
    vec![
        Arc::new(CreateNoteTool::new(vault_root.clone())),
        Arc::new(AppendIndexEntryTool::new(vault_root.clone())),
        Arc::new(ProposeNoteUpdateTool::new(vault_root)),
    ]
}

pub fn vault_agent_tools(vault_root: impl Into<PathBuf>) -> Vec<Arc<dyn Tool>> {
    let vault_root = vault_root.into();
    let mut tools = vault_read_tools(vault_root.clone());
    tools.extend(vault_edit_tools(vault_root));
    tools
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ListNotesResult {
    notes: Vec<NoteSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct NoteSummary {
    path: String,
    title: Option<String>,
    note_type: Option<String>,
    tags: Vec<String>,
}

struct ListNotesTool {
    vault_root: Arc<PathBuf>,
    schema: Value,
}

impl ListNotesTool {
    fn new(vault_root: Arc<PathBuf>) -> Self {
        Self {
            vault_root,
            schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        }
    }
}

impl Tool for ListNotesTool {
    fn name(&self) -> &str {
        "vault_list_notes"
    }

    fn label(&self) -> &str {
        "列出知识库笔记"
    }

    fn description(&self) -> &str {
        "列出当前 Obsidian 知识库中的 Markdown 笔记路径、标题、类型和标签。"
    }

    fn parameters_schema(&self) -> &Value {
        &self.schema
    }

    fn execute<'a>(
        &'a self,
        _args: Value,
        _ctx: &'a ToolContext,
    ) -> BoxFuture<'a, Result<ToolResult, ToolError>> {
        Box::pin(async move {
            let scan = scan_vault(&self.vault_root).map_err(to_tool_error)?;
            let notes = scan
                .notes
                .into_iter()
                .map(|note| NoteSummary {
                    path: note.relative_path,
                    title: note.title,
                    note_type: note.note_type,
                    tags: note.tags,
                })
                .collect();

            ok_json(ListNotesResult { notes })
        })
    }
}

#[derive(Debug, Deserialize)]
struct ReadNoteArgs {
    path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ReadNoteResult {
    path: String,
    content: String,
    truncated: bool,
}

struct ReadNoteTool {
    vault_root: Arc<PathBuf>,
    schema: Value,
}

impl ReadNoteTool {
    fn new(vault_root: Arc<PathBuf>) -> Self {
        Self {
            vault_root,
            schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "相对知识库根目录的 Markdown 文件路径，例如 docs/concepts/agent-harness.md"
                    }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
        }
    }
}

impl Tool for ReadNoteTool {
    fn name(&self) -> &str {
        "vault_read_note"
    }

    fn label(&self) -> &str {
        "读取笔记"
    }

    fn description(&self) -> &str {
        "读取当前 Obsidian 知识库内指定 Markdown 笔记的正文内容。只能读取知识库内的 .md 文件。"
    }

    fn parameters_schema(&self) -> &Value {
        &self.schema
    }

    fn execute<'a>(
        &'a self,
        args: Value,
        _ctx: &'a ToolContext,
    ) -> BoxFuture<'a, Result<ToolResult, ToolError>> {
        Box::pin(async move {
            let args: ReadNoteArgs = serde_json::from_value(args)
                .map_err(|err| ToolError::InvalidArguments(err.to_string()))?;
            let path = resolve_markdown_path(&self.vault_root, &args.path)?;
            let content = tokio::fs::read_to_string(&path)
                .await
                .map_err(|err| ToolError::Execution(err.to_string()))?;
            let (content, truncated) = truncate_chars(content, MAX_NOTE_CHARS);

            ok_json(ReadNoteResult {
                path: normalize_relative_path(&args.path),
                content,
                truncated,
            })
        })
    }
}

#[derive(Debug, Deserialize)]
struct SearchNotesArgs {
    query: String,
    limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct SearchNotesResult {
    query: String,
    matches: Vec<SearchMatch>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct SearchMatch {
    path: String,
    line: usize,
    snippet: String,
}

struct SearchNotesTool {
    vault_root: Arc<PathBuf>,
    schema: Value,
}

impl SearchNotesTool {
    fn new(vault_root: Arc<PathBuf>) -> Self {
        Self {
            vault_root,
            schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "要在 Markdown 笔记中查找的文本"
                    },
                    "limit": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 20,
                        "description": "最多返回多少条匹配，默认 10，最大 20"
                    }
                },
                "required": ["query"],
                "additionalProperties": false
            }),
        }
    }
}

impl Tool for SearchNotesTool {
    fn name(&self) -> &str {
        "vault_search_notes"
    }

    fn label(&self) -> &str {
        "搜索笔记"
    }

    fn description(&self) -> &str {
        "按纯文本在当前 Obsidian 知识库的 Markdown 笔记中搜索，返回路径、行号和片段。"
    }

    fn parameters_schema(&self) -> &Value {
        &self.schema
    }

    fn execute<'a>(
        &'a self,
        args: Value,
        _ctx: &'a ToolContext,
    ) -> BoxFuture<'a, Result<ToolResult, ToolError>> {
        Box::pin(async move {
            let args: SearchNotesArgs = serde_json::from_value(args)
                .map_err(|err| ToolError::InvalidArguments(err.to_string()))?;
            let query = args.query.trim().to_string();
            if query.is_empty() {
                return Err(ToolError::InvalidArguments(
                    "query cannot be empty".to_string(),
                ));
            }

            let limit = args.limit.unwrap_or(10).clamp(1, MAX_SEARCH_RESULTS);
            let query_lower = query.to_lowercase();
            let scan = scan_vault(&self.vault_root).map_err(to_tool_error)?;
            let mut matches = Vec::new();

            for note in scan.notes {
                let path = resolve_markdown_path(&self.vault_root, &note.relative_path)?;
                let content = tokio::fs::read_to_string(&path)
                    .await
                    .map_err(|err| ToolError::Execution(err.to_string()))?;
                for (index, line) in content.lines().enumerate() {
                    if line.to_lowercase().contains(&query_lower) {
                        matches.push(SearchMatch {
                            path: note.relative_path.clone(),
                            line: index + 1,
                            snippet: truncate_chars(line.trim().to_string(), MAX_SNIPPET_CHARS).0,
                        });
                        if matches.len() >= limit {
                            return ok_json(SearchNotesResult { query, matches });
                        }
                    }
                }
            }

            ok_json(SearchNotesResult { query, matches })
        })
    }
}

#[derive(Debug, Deserialize)]
struct NeighborNotesArgs {
    path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct NeighborNotesResult {
    path: String,
    outgoing_links: Vec<String>,
    backlinks: Vec<String>,
}

struct NeighborNotesTool {
    vault_root: Arc<PathBuf>,
    schema: Value,
}

#[derive(Debug, Deserialize)]
struct CreateNoteArgs {
    path: String,
    content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CreateNoteResult {
    path: String,
    written: bool,
}

struct CreateNoteTool {
    vault_root: Arc<PathBuf>,
    schema: Value,
}

impl CreateNoteTool {
    fn new(vault_root: Arc<PathBuf>) -> Self {
        Self {
            vault_root,
            schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative Markdown path for the new note. Must end with .md and must not already exist."
                    },
                    "content": {
                        "type": "string",
                        "description": "Full Markdown content to write into the new note."
                    }
                },
                "required": ["path", "content"],
                "additionalProperties": false
            }),
        }
    }
}

impl Tool for CreateNoteTool {
    fn name(&self) -> &str {
        "vault_create_note"
    }

    fn label(&self) -> &str {
        "Create note"
    }

    fn description(&self) -> &str {
        "Create a new Markdown note inside the current Obsidian vault. It never overwrites existing files."
    }

    fn parameters_schema(&self) -> &Value {
        &self.schema
    }

    fn execute<'a>(
        &'a self,
        args: Value,
        _ctx: &'a ToolContext,
    ) -> BoxFuture<'a, Result<ToolResult, ToolError>> {
        Box::pin(async move {
            let args: CreateNoteArgs = serde_json::from_value(args)
                .map_err(|err| ToolError::InvalidArguments(err.to_string()))?;
            let normalized = normalize_relative_path(&args.path);
            let path = resolve_new_markdown_path(&self.vault_root, &normalized)?;
            if let Some(parent) = path.parent() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|err| ToolError::Execution(err.to_string()))?;
            }
            tokio::fs::write(&path, args.content)
                .await
                .map_err(|err| ToolError::Execution(err.to_string()))?;

            ok_json(CreateNoteResult {
                path: normalized,
                written: true,
            })
        })
    }
}

#[derive(Debug, Deserialize)]
struct AppendIndexEntryArgs {
    index_path: String,
    target_path: String,
    title: String,
    summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct AppendIndexEntryResult {
    index_path: String,
    target_path: String,
    written: bool,
    decision: String,
}

struct AppendIndexEntryTool {
    vault_root: Arc<PathBuf>,
    schema: Value,
}

impl AppendIndexEntryTool {
    fn new(vault_root: Arc<PathBuf>) -> Self {
        Self {
            vault_root,
            schema: json!({
                "type": "object",
                "properties": {
                    "index_path": {
                        "type": "string",
                        "description": "Relative path of the Markdown index file to update."
                    },
                    "target_path": {
                        "type": "string",
                        "description": "Relative path of the note that should be linked from the index."
                    },
                    "title": {
                        "type": "string",
                        "description": "Index entry heading."
                    },
                    "summary": {
                        "type": "string",
                        "description": "Optional one-line summary."
                    }
                },
                "required": ["index_path", "target_path", "title"],
                "additionalProperties": false
            }),
        }
    }
}

impl Tool for AppendIndexEntryTool {
    fn name(&self) -> &str {
        "vault_append_index_entry"
    }

    fn label(&self) -> &str {
        "Append index entry"
    }

    fn description(&self) -> &str {
        "Append a Markdown index entry. This is a low-risk automatic write according to the vault write policy."
    }

    fn parameters_schema(&self) -> &Value {
        &self.schema
    }

    fn execute<'a>(
        &'a self,
        args: Value,
        _ctx: &'a ToolContext,
    ) -> BoxFuture<'a, Result<ToolResult, ToolError>> {
        Box::pin(async move {
            let args: AppendIndexEntryArgs = serde_json::from_value(args)
                .map_err(|err| ToolError::InvalidArguments(err.to_string()))?;
            let index_path = normalize_relative_path(&args.index_path);
            let target_path = normalize_relative_path(&args.target_path);
            let decision = VaultWritePolicy.decide(&VaultWriteOperation::AddIndexEntry {
                index_path: index_path.clone(),
                target_path: target_path.clone(),
            });
            if decision != WriteDecision::AllowAutomatic {
                return Err(ToolError::Execution(
                    "write policy requires confirmation for this index update".to_string(),
                ));
            }

            let full_index_path = resolve_markdown_path(&self.vault_root, &index_path)?;
            let mut entry = format!(
                "\n## {}\n\n- link: [[{}]]\n",
                args.title.trim(),
                target_path
            );
            if let Some(summary) = args.summary.filter(|value| !value.trim().is_empty()) {
                entry.push_str(&format!("- summary: {}\n", summary.trim()));
            }
            tokio::fs::OpenOptions::new()
                .append(true)
                .open(&full_index_path)
                .await
                .map_err(|err| ToolError::Execution(err.to_string()))?
                .write_all(entry.as_bytes())
                .await
                .map_err(|err| ToolError::Execution(err.to_string()))?;

            ok_json(AppendIndexEntryResult {
                index_path,
                target_path,
                written: true,
                decision: "allow_automatic".to_string(),
            })
        })
    }
}

#[derive(Debug, Deserialize)]
struct ProposeNoteUpdateArgs {
    path: String,
    replacement_content: String,
    reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ProposeNoteUpdateResult {
    path: String,
    written: bool,
    requires_confirmation: bool,
    reason: Option<String>,
    replacement_content: String,
}

struct ProposeNoteUpdateTool {
    vault_root: Arc<PathBuf>,
    schema: Value,
}

impl ProposeNoteUpdateTool {
    fn new(vault_root: Arc<PathBuf>) -> Self {
        Self {
            vault_root,
            schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path of the existing Markdown note."
                    },
                    "replacement_content": {
                        "type": "string",
                        "description": "Full replacement Markdown content proposed for the note."
                    },
                    "reason": {
                        "type": "string",
                        "description": "Short reason for the proposed edit."
                    }
                },
                "required": ["path", "replacement_content"],
                "additionalProperties": false
            }),
        }
    }
}

impl Tool for ProposeNoteUpdateTool {
    fn name(&self) -> &str {
        "vault_propose_note_update"
    }

    fn label(&self) -> &str {
        "Propose note update"
    }

    fn description(&self) -> &str {
        "Prepare a replacement for an existing note without writing it. Existing note body edits require user confirmation."
    }

    fn parameters_schema(&self) -> &Value {
        &self.schema
    }

    fn execute<'a>(
        &'a self,
        args: Value,
        _ctx: &'a ToolContext,
    ) -> BoxFuture<'a, Result<ToolResult, ToolError>> {
        Box::pin(async move {
            let args: ProposeNoteUpdateArgs = serde_json::from_value(args)
                .map_err(|err| ToolError::InvalidArguments(err.to_string()))?;
            let normalized = normalize_relative_path(&args.path);
            let _path = resolve_markdown_path(&self.vault_root, &normalized)?;
            let decision = VaultWritePolicy.decide(&VaultWriteOperation::ModifyBodyMeaning {
                path: normalized.clone(),
            });

            ok_json(ProposeNoteUpdateResult {
                path: normalized,
                written: false,
                requires_confirmation: decision == WriteDecision::RequireConfirmation,
                reason: args.reason,
                replacement_content: args.replacement_content,
            })
        })
    }
}

impl NeighborNotesTool {
    fn new(vault_root: Arc<PathBuf>) -> Self {
        Self {
            vault_root,
            schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "相对知识库根目录的 Markdown 文件路径"
                    }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
        }
    }
}

impl Tool for NeighborNotesTool {
    fn name(&self) -> &str {
        "vault_neighbor_notes"
    }

    fn label(&self) -> &str {
        "查看邻近节点"
    }

    fn description(&self) -> &str {
        "查看某篇笔记的出链和反链，用于沿 Obsidian 链接图展开临近知识节点。"
    }

    fn parameters_schema(&self) -> &Value {
        &self.schema
    }

    fn execute<'a>(
        &'a self,
        args: Value,
        _ctx: &'a ToolContext,
    ) -> BoxFuture<'a, Result<ToolResult, ToolError>> {
        Box::pin(async move {
            let args: NeighborNotesArgs = serde_json::from_value(args)
                .map_err(|err| ToolError::InvalidArguments(err.to_string()))?;
            let normalized = normalize_relative_path(&args.path);
            let _path = resolve_markdown_path(&self.vault_root, &normalized)?;
            let scan = scan_vault(&self.vault_root).map_err(to_tool_error)?;
            let graph = build_link_graph(&scan);

            let note = scan
                .notes
                .iter()
                .find(|note| note.relative_path == normalized)
                .ok_or_else(|| ToolError::Execution(format!("note not found: {normalized}")))?;
            let outgoing_links = note
                .links
                .iter()
                .map(|link| link.target.clone())
                .collect::<Vec<_>>();

            let mut backlinks = graph
                .backlinks_to(&normalized)
                .into_iter()
                .map(str::to_string)
                .collect::<Vec<_>>();
            for identity in note_identities(note) {
                backlinks.extend(
                    graph
                        .backlinks_to(&identity)
                        .into_iter()
                        .map(str::to_string),
                );
            }
            backlinks.sort();
            backlinks.dedup();

            ok_json(NeighborNotesResult {
                path: normalized,
                outgoing_links,
                backlinks,
            })
        })
    }
}

fn ok_json(value: impl Serialize) -> Result<ToolResult, ToolError> {
    let details =
        serde_json::to_value(&value).map_err(|err| ToolError::Execution(err.to_string()))?;
    let text = serde_json::to_string_pretty(&details)
        .map_err(|err| ToolError::Execution(err.to_string()))?;

    Ok(ToolResult {
        content: vec![ContentBlock::Text { text }],
        details,
        terminate: false,
    })
}

fn resolve_markdown_path(vault_root: &Path, relative_path: &str) -> Result<PathBuf, ToolError> {
    let normalized = normalize_relative_path(relative_path);
    if !normalized.ends_with(".md") {
        return Err(ToolError::InvalidArguments(
            "path must point to a Markdown .md file".to_string(),
        ));
    }

    let path = Path::new(&normalized);
    if path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_)))
    {
        return Err(ToolError::InvalidArguments(
            "path must stay inside the vault".to_string(),
        ));
    }

    let full_path = vault_root.join(path);
    if !full_path.exists() {
        return Err(ToolError::Execution(format!(
            "note not found: {normalized}"
        )));
    }

    Ok(full_path)
}

fn resolve_new_markdown_path(vault_root: &Path, relative_path: &str) -> Result<PathBuf, ToolError> {
    let normalized = normalize_relative_path(relative_path);
    if !normalized.ends_with(".md") {
        return Err(ToolError::InvalidArguments(
            "path must point to a Markdown .md file".to_string(),
        ));
    }

    let path = Path::new(&normalized);
    if path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_)))
    {
        return Err(ToolError::InvalidArguments(
            "path must stay inside the vault".to_string(),
        ));
    }

    let full_path = vault_root.join(path);
    if full_path.exists() {
        return Err(ToolError::Execution(format!(
            "note already exists: {normalized}"
        )));
    }

    Ok(full_path)
}

fn normalize_relative_path(path: &str) -> String {
    path.trim()
        .replace('\\', "/")
        .trim_start_matches('/')
        .to_string()
}

fn truncate_chars(value: String, max_chars: usize) -> (String, bool) {
    if value.chars().count() <= max_chars {
        return (value, false);
    }

    (value.chars().take(max_chars).collect(), true)
}

fn note_identities(note: &knowledge_agent_core::vault::scanner::ScannedNote) -> Vec<String> {
    let mut identities = vec![note.relative_path.clone()];
    if let Some(title) = &note.title {
        identities.push(title.clone());
    }
    if let Some(stem) = Path::new(&note.relative_path)
        .file_stem()
        .and_then(|stem| stem.to_str())
    {
        identities.push(stem.to_string());
    }
    identities.sort();
    identities.dedup();
    identities
}

fn to_tool_error(err: anyhow::Error) -> ToolError {
    ToolError::Execution(err.to_string())
}
