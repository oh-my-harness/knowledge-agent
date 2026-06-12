use std::{
    path::{Component, Path, PathBuf},
    sync::Arc,
};

use futures::future::BoxFuture;
use knowledge_agent_core::vault::{
    assets::{PdfAsset, list_pdf_assets},
    confirmation::{CreateReplaceNoteConfirmation, create_replace_note_confirmation},
    graph::build_link_graph,
    policy::{VaultWriteOperation, VaultWritePolicy, WriteDecision},
    scanner::scan_vault,
};
use llm_harness::prelude::{ContentBlock, Tool, ToolContext, ToolError, ToolResult};
use reqwest::Url;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::io::AsyncWriteExt;

const MAX_NOTE_CHARS: usize = 24_000;
const MAX_SOURCE_CHARS: usize = 48_000;
const MAX_SEARCH_RESULTS: usize = 20;
const MAX_SNIPPET_CHARS: usize = 240;
const MAX_WEB_SEARCH_RESULTS: usize = 8;
const WEB_SEARCH_TIMEOUT_SECS: u64 = 12;

pub fn vault_read_tools(vault_root: impl Into<PathBuf>) -> Vec<Arc<dyn Tool>> {
    let vault_root = Arc::new(vault_root.into());
    vec![
        Arc::new(ListNotesTool::new(vault_root.clone())),
        Arc::new(ReadNoteTool::new(vault_root.clone())),
        Arc::new(SearchNotesTool::new(vault_root.clone())),
        Arc::new(NeighborNotesTool::new(vault_root.clone())),
        Arc::new(FindRelatedNotesTool::new(vault_root.clone())),
        Arc::new(ListPdfAssetsTool::new(vault_root.clone())),
        Arc::new(ReadPdfTextTool::new(vault_root)),
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

pub fn web_search_tools() -> Vec<Arc<dyn Tool>> {
    vec![Arc::new(DuckDuckGoSearchTool::new())]
}

pub fn web_fetch_tools() -> Vec<Arc<dyn Tool>> {
    vec![Arc::new(WebFetchPageTool::new())]
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
struct WebSearchArgs {
    query: String,
    limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct WebSearchResult {
    query: String,
    results: Vec<WebSearchItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct WebSearchItem {
    title: String,
    url: String,
    snippet: String,
}

struct DuckDuckGoSearchTool {
    schema: Value,
    client: reqwest::Client,
}

impl DuckDuckGoSearchTool {
    fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(WEB_SEARCH_TIMEOUT_SECS))
            .user_agent("knowledge-agent/0.1 local research assistant")
            .build()
            .expect("reqwest client configuration is valid");
        Self {
            client,
            schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "要搜索的网页关键词。应包含足够具体的主题、实体或时间范围。"
                    },
                    "limit": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 8,
                        "description": "最多返回多少条结果，默认 5，最大 8"
                    }
                },
                "required": ["query"],
                "additionalProperties": false
            }),
        }
    }
}

impl Tool for DuckDuckGoSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn label(&self) -> &str {
        "搜索网页"
    }

    fn description(&self) -> &str {
        "使用 DuckDuckGo 搜索公开网页，返回标题、链接和摘要。用于研究开放网络上的最新资料；如果需要可靠结论，应交叉比较多个结果。"
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
            let args: WebSearchArgs = serde_json::from_value(args)
                .map_err(|err| ToolError::InvalidArguments(err.to_string()))?;
            let query = args.query.trim().to_string();
            if query.is_empty() {
                return Err(ToolError::InvalidArguments(
                    "query cannot be empty".to_string(),
                ));
            }

            let limit = args.limit.unwrap_or(5).clamp(1, MAX_WEB_SEARCH_RESULTS);
            let mut url = Url::parse("https://duckduckgo.com/html/")
                .map_err(|err| ToolError::Execution(err.to_string()))?;
            url.query_pairs_mut().append_pair("q", &query);

            let html = self
                .client
                .get(url)
                .send()
                .await
                .map_err(|err| ToolError::Execution(err.to_string()))?
                .error_for_status()
                .map_err(|err| ToolError::Execution(err.to_string()))?
                .text()
                .await
                .map_err(|err| ToolError::Execution(err.to_string()))?;
            let results = parse_duckduckgo_html(&html, limit);

            ok_json(WebSearchResult { query, results })
        })
    }
}

#[derive(Debug, Deserialize)]
struct WebFetchPageArgs {
    url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct WebFetchPageResult {
    url: String,
    title: Option<String>,
    description: Option<String>,
    text: String,
    truncated: bool,
}

struct WebFetchPageTool {
    schema: Value,
    client: reqwest::Client,
}

impl WebFetchPageTool {
    fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(WEB_SEARCH_TIMEOUT_SECS))
            .user_agent("knowledge-agent/0.1 local research assistant")
            .build()
            .expect("reqwest client configuration is valid");
        Self {
            client,
            schema: json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "要读取和总结的公开网页 URL，必须是 http 或 https。"
                    }
                },
                "required": ["url"],
                "additionalProperties": false
            }),
        }
    }
}

impl Tool for WebFetchPageTool {
    fn name(&self) -> &str {
        "web_fetch_page"
    }

    fn label(&self) -> &str {
        "读取网页"
    }

    fn description(&self) -> &str {
        "读取指定网页并提取标题、描述和正文文本。用于用户给定链接后的阅读、总结和生成知识库资料卡。"
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
            let args: WebFetchPageArgs = serde_json::from_value(args)
                .map_err(|err| ToolError::InvalidArguments(err.to_string()))?;
            let url = Url::parse(args.url.trim())
                .map_err(|err| ToolError::InvalidArguments(err.to_string()))?;
            if !matches!(url.scheme(), "http" | "https") {
                return Err(ToolError::InvalidArguments(
                    "url must use http or https".to_string(),
                ));
            }

            let html = self
                .client
                .get(url.clone())
                .send()
                .await
                .map_err(|err| ToolError::Execution(err.to_string()))?
                .error_for_status()
                .map_err(|err| ToolError::Execution(err.to_string()))?
                .text()
                .await
                .map_err(|err| ToolError::Execution(err.to_string()))?;
            let page = extract_page_text(url.as_str(), &html);

            ok_json(page)
        })
    }
}

#[derive(Debug, Deserialize)]
struct FindRelatedNotesArgs {
    text: String,
    limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct FindRelatedNotesResult {
    matches: Vec<RelatedNote>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct RelatedNote {
    path: String,
    title: Option<String>,
    tags: Vec<String>,
    score: usize,
    reason: String,
}

struct FindRelatedNotesTool {
    vault_root: Arc<PathBuf>,
    schema: Value,
}

impl FindRelatedNotesTool {
    fn new(vault_root: Arc<PathBuf>) -> Self {
        Self {
            vault_root,
            schema: json!({
                "type": "object",
                "properties": {
                    "text": {
                        "type": "string",
                        "description": "从网页或 PDF 提取出的标题、摘要、关键词或正文片段。工具会据此在现有 Markdown 笔记中寻找相关知识。"
                    },
                    "limit": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 20,
                        "description": "最多返回多少篇相关笔记，默认 8，最大 20。"
                    }
                },
                "required": ["text"],
                "additionalProperties": false
            }),
        }
    }
}

impl Tool for FindRelatedNotesTool {
    fn name(&self) -> &str {
        "vault_find_related_notes"
    }

    fn label(&self) -> &str {
        "查找相关笔记"
    }

    fn description(&self) -> &str {
        "根据资料文本在当前 Obsidian 知识库中查找相关 Markdown 笔记，用于生成资料卡时建立 wikilink 关联。"
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
            let args: FindRelatedNotesArgs = serde_json::from_value(args)
                .map_err(|err| ToolError::InvalidArguments(err.to_string()))?;
            let text = normalize_whitespace(&args.text);
            if text.is_empty() {
                return Err(ToolError::InvalidArguments(
                    "text cannot be empty".to_string(),
                ));
            }

            let limit = args.limit.unwrap_or(8).clamp(1, MAX_SEARCH_RESULTS);
            let tokens = meaningful_tokens(&text);
            let scan = scan_vault(&self.vault_root).map_err(to_tool_error)?;
            let mut matches = Vec::new();

            for note in scan.notes {
                let path = resolve_markdown_path(&self.vault_root, &note.relative_path)?;
                let content = tokio::fs::read_to_string(&path)
                    .await
                    .map_err(|err| ToolError::Execution(err.to_string()))?;
                let haystack = format!(
                    "{} {} {} {}",
                    note.relative_path,
                    note.title.clone().unwrap_or_default(),
                    note.tags.join(" "),
                    content
                )
                .to_lowercase();
                let mut matched_tokens = Vec::new();
                let mut score = 0;
                for token in &tokens {
                    if haystack.contains(token) {
                        matched_tokens.push(token.clone());
                        score += if note
                            .title
                            .as_deref()
                            .is_some_and(|title| title.to_lowercase().contains(token))
                        {
                            4
                        } else if note
                            .tags
                            .iter()
                            .any(|tag| tag.to_lowercase().contains(token))
                        {
                            3
                        } else if note.relative_path.to_lowercase().contains(token) {
                            2
                        } else {
                            1
                        };
                    }
                }

                if score > 0 {
                    matched_tokens.sort();
                    matched_tokens.dedup();
                    matches.push(RelatedNote {
                        path: note.relative_path,
                        title: note.title,
                        tags: note.tags,
                        score,
                        reason: format!(
                            "matched keywords: {}",
                            matched_tokens
                                .into_iter()
                                .take(8)
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                    });
                }
            }

            matches.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.path.cmp(&b.path)));
            matches.truncate(limit);

            ok_json(FindRelatedNotesResult { matches })
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ListPdfAssetsResult {
    pdfs: Vec<PdfAsset>,
}

struct ListPdfAssetsTool {
    vault_root: Arc<PathBuf>,
    schema: Value,
}

impl ListPdfAssetsTool {
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

impl Tool for ListPdfAssetsTool {
    fn name(&self) -> &str {
        "vault_list_pdf_assets"
    }

    fn label(&self) -> &str {
        "列出 PDF"
    }

    fn description(&self) -> &str {
        "列出当前 Obsidian 知识库中的 PDF 资源文件，优先用于发现 assets/papers 和 assets/references 下的原始资料。"
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
            let pdfs = list_pdf_assets(&self.vault_root).map_err(to_tool_error)?;
            ok_json(ListPdfAssetsResult { pdfs })
        })
    }
}

#[derive(Debug, Deserialize)]
struct ReadPdfTextArgs {
    path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ReadPdfTextResult {
    path: String,
    text: String,
    truncated: bool,
}

struct ReadPdfTextTool {
    vault_root: Arc<PathBuf>,
    schema: Value,
}

impl ReadPdfTextTool {
    fn new(vault_root: Arc<PathBuf>) -> Self {
        Self {
            vault_root,
            schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "相对知识库根目录的 PDF 文件路径，例如 assets/papers/rag/example.pdf"
                    }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
        }
    }
}

impl Tool for ReadPdfTextTool {
    fn name(&self) -> &str {
        "vault_read_pdf_text"
    }

    fn label(&self) -> &str {
        "读取 PDF 文本"
    }

    fn description(&self) -> &str {
        "提取知识库内文本型 PDF 的文字内容，用于总结原始 PDF 资料并生成 Markdown 资料卡。扫描版 PDF 可能无法提取有效文字。"
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
            let args: ReadPdfTextArgs = serde_json::from_value(args)
                .map_err(|err| ToolError::InvalidArguments(err.to_string()))?;
            let normalized = normalize_relative_path(&args.path);
            let path = resolve_pdf_path(&self.vault_root, &normalized)?;
            let extracted = tokio::task::spawn_blocking(move || pdf_extract::extract_text(path))
                .await
                .map_err(|err| ToolError::Execution(err.to_string()))?
                .map_err(|err| ToolError::Execution(err.to_string()))?;
            let text = normalize_whitespace(&extracted);
            let (text, truncated) = truncate_chars(text, MAX_SOURCE_CHARS);

            ok_json(ReadPdfTextResult {
                path: normalized,
                text,
                truncated,
            })
        })
    }
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
    confirmation_id: String,
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
            let item = create_replace_note_confirmation(
                self.vault_root.as_ref(),
                CreateReplaceNoteConfirmation {
                    path: normalized.clone(),
                    reason: args.reason.clone(),
                    proposed_content: args.replacement_content.clone(),
                },
            )
            .map_err(to_tool_error)?;

            ok_json(ProposeNoteUpdateResult {
                confirmation_id: item.id,
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

fn extract_page_text(url: &str, html: &str) -> WebFetchPageResult {
    let document = Html::parse_document(html);
    let title = document
        .select(&selector("title"))
        .next()
        .map(|node| normalize_whitespace(&node.text().collect::<Vec<_>>().join(" ")))
        .filter(|value| !value.is_empty());
    let description = document
        .select(&selector(r#"meta[name="description"]"#))
        .next()
        .and_then(|node| node.value().attr("content"))
        .map(normalize_whitespace)
        .filter(|value| !value.is_empty());

    let content_selector = selector("article, main, body");
    let mut chunks = Vec::new();
    for node in document.select(&content_selector) {
        let text = normalize_whitespace(&node.text().collect::<Vec<_>>().join(" "));
        if text.chars().count() > 80 {
            chunks.push(text);
        }
    }

    let text = chunks
        .into_iter()
        .max_by_key(|chunk| chunk.chars().count())
        .unwrap_or_else(|| {
            normalize_whitespace(&document.root_element().text().collect::<Vec<_>>().join(" "))
        });
    let (text, truncated) = truncate_chars(text, MAX_SOURCE_CHARS);

    WebFetchPageResult {
        url: url.to_string(),
        title,
        description,
        text,
        truncated,
    }
}

fn parse_duckduckgo_html(html: &str, limit: usize) -> Vec<WebSearchItem> {
    let document = Html::parse_document(html);
    let result_selector = selector(".result");
    let title_selector = selector(".result__a");
    let snippet_selector = selector(".result__snippet");
    let fallback_snippet_selector = selector(".result__body");
    let mut results = Vec::new();

    for result in document.select(&result_selector) {
        let Some(title_link) = result.select(&title_selector).next() else {
            continue;
        };
        let title = normalize_whitespace(&title_link.text().collect::<Vec<_>>().join(" "));
        let Some(href) = title_link.value().attr("href") else {
            continue;
        };
        let url = clean_duckduckgo_url(href);
        if title.is_empty() || url.is_empty() {
            continue;
        }

        let snippet = result
            .select(&snippet_selector)
            .next()
            .or_else(|| result.select(&fallback_snippet_selector).next())
            .map(|node| normalize_whitespace(&node.text().collect::<Vec<_>>().join(" ")))
            .unwrap_or_default();
        let snippet = truncate_chars(snippet, MAX_SNIPPET_CHARS).0;

        results.push(WebSearchItem {
            title,
            url,
            snippet,
        });
        if results.len() >= limit {
            break;
        }
    }

    results
}

fn clean_duckduckgo_url(href: &str) -> String {
    if let Ok(url) = Url::parse(href) {
        if let Some(uddg) = url.query_pairs().find_map(|(key, value)| {
            if key == "uddg" {
                Some(value.into_owned())
            } else {
                None
            }
        }) {
            return uddg;
        }
        return url.to_string();
    }

    let Ok(base) = Url::parse("https://duckduckgo.com") else {
        return href.to_string();
    };
    match base.join(href) {
        Ok(url) => {
            if let Some(uddg) = url.query_pairs().find_map(|(key, value)| {
                if key == "uddg" {
                    Some(value.into_owned())
                } else {
                    None
                }
            }) {
                uddg
            } else {
                url.to_string()
            }
        }
        Err(_) => href.to_string(),
    }
}

fn normalize_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn selector(value: &str) -> Selector {
    Selector::parse(value).expect("static CSS selector is valid")
}

fn meaningful_tokens(text: &str) -> Vec<String> {
    let mut tokens = text
        .split(|ch: char| !ch.is_alphanumeric())
        .map(str::trim)
        .filter(|token| token.chars().count() >= 3)
        .map(str::to_lowercase)
        .filter(|token| !COMMON_TOKENS.contains(&token.as_str()))
        .take(256)
        .collect::<Vec<_>>();
    tokens.sort();
    tokens.dedup();
    tokens
}

const COMMON_TOKENS: &[&str] = &[
    "the", "and", "for", "with", "from", "this", "that", "you", "are", "was", "were", "can",
    "will", "your", "about", "into", "using", "use", "used", "not", "but", "all", "one", "two",
    "more", "when", "what", "which", "their", "there", "they", "them", "these", "those", "一个",
    "以及", "可以", "通过", "进行", "当前", "相关", "知识", "内容", "这个", "需要", "如果",
];

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

fn resolve_pdf_path(vault_root: &Path, relative_path: &str) -> Result<PathBuf, ToolError> {
    let normalized = normalize_relative_path(relative_path);
    if !normalized.to_lowercase().ends_with(".pdf") {
        return Err(ToolError::InvalidArguments(
            "path must point to a PDF .pdf file".to_string(),
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
        return Err(ToolError::Execution(format!("pdf not found: {normalized}")));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_duckduckgo_html_results() {
        let html = r#"
          <div class="result">
            <a class="result__a" href="/l/?uddg=https%3A%2F%2Fexample.com%2Farticle%3Fx%3D1"> Example Article </a>
            <a class="result__snippet"> A short result summary with extra spacing. </a>
          </div>
          <div class="result">
            <a class="result__a" href="https://example.org/direct">Second Result</a>
            <div class="result__snippet">Another summary</div>
          </div>
        "#;

        let results = parse_duckduckgo_html(html, 5);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].title, "Example Article");
        assert_eq!(results[0].url, "https://example.com/article?x=1");
        assert_eq!(
            results[0].snippet,
            "A short result summary with extra spacing."
        );
        assert_eq!(results[1].url, "https://example.org/direct");
    }

    #[test]
    fn limits_duckduckgo_html_results() {
        let html = r#"
          <div class="result"><a class="result__a" href="https://one.example">One</a></div>
          <div class="result"><a class="result__a" href="https://two.example">Two</a></div>
        "#;

        let results = parse_duckduckgo_html(html, 1);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "One");
    }
}
