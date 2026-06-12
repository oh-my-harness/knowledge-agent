# Knowledge Agent

Knowledge Agent 是一个本地运行的研究助手和 Obsidian vault 维护工具。

## 基础命令

在 Obsidian vault 根目录运行：

```bash
knowledge-agent serve .
```

开发时可以使用 fixture vault 运行：

```bash
cargo run -p knowledge-agent-cli -- serve crates/knowledge-agent-core/tests/fixtures/basic-vault --port 3030
```

当前 foundation 阶段提供的接口：

- `GET /api/health`
- `GET /api/vault/index`
- `POST /api/maintenance/scan`

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

## LLM 配置

当前第一版 LLM 接入使用 `llm-harness-core` 的 DeepSeek 示例路径。启动后端前设置：

```powershell
$env:DEEPSEEK_API_KEY="sk-..."
$env:DEEPSEEK_MODEL="deepseek-v4-flash"
cargo run -p knowledge-agent-cli -- serve . --port 3030
```

`DEEPSEEK_MODEL` 可省略，默认使用 `deepseek-v4-flash`。

如果没有设置 `DEEPSEEK_API_KEY`，`/api/ask` 会返回明确错误。API key 属于个人配置，不应提交到 Git。

Web 设置页会读写：

```text
.knowledge-agent/local.toml
```

该文件保存本机 LLM provider、DeepSeek API key、模型名和网页搜索配置。环境变量仍可覆盖对应配置；保存后的 LLM 配置会在服务重启后用于新 runner。

聊天会话由 `llm-harness-core` 的 session 机制保存到：

```text
.knowledge-agent/sessions/
```

该目录是个人运行状态，已经被 `.gitignore` 忽略，不应提交到远端知识库。

Web 聊天界面支持多个会话：

- 会话列表、创建和切换通过 `/api/ask/sessions` 系列接口管理。
- 每个会话对应一个 `llm-harness-core` JSONL session。
- 输入框中按 Enter 直接发送，按 Shift+Enter 换行。

当前 agent 已通过 `llm-harness-core` 的 Tool 机制接入一组只读知识库工具：

- `vault_list_notes`：列出 vault 内的 Markdown 笔记。
- `vault_read_note`：读取指定 Markdown 笔记内容。
- `vault_search_notes`：按纯文本搜索笔记内容。
- `vault_neighbor_notes`：查看指定笔记的出链和反链。

这些工具只读取当前 vault 内的 `.md` 文件；编辑类工具会在后续按写入安全策略单独接入。
