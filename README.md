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
- `GET /api/confirmations`
- `POST /api/confirmations/{id}/apply`
- `POST /api/confirmations/{id}/reject`

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

需要用户确认的编辑提案保存到：

```text
.knowledge-agent/confirmations/
```

该目录同样是个人运行状态，不应提交到远端知识库。维护页会展示这些待确认修改，用户可以确认应用或拒绝。

Web 聊天界面支持多个会话：

- 会话列表、创建和切换通过 `/api/ask/sessions` 系列接口管理。
- 每个会话对应一个 `llm-harness-core` JSONL session。
- 输入框中按 Enter 直接发送，按 Shift+Enter 换行。

当前 agent 已通过 `llm-harness-core` 的 Tool 机制接入一组只读知识库工具：

- `vault_list_notes`：列出 vault 内的 Markdown 笔记。
- `vault_read_note`：读取指定 Markdown 笔记内容。
- `vault_search_notes`：按纯文本搜索笔记内容。
- `vault_neighbor_notes`：查看指定笔记的出链和反链。

当前也接入了第一版编辑工具：

- `vault_create_note`：在 vault 内创建新的 Markdown 笔记，不会覆盖已有文件。
- `vault_append_index_entry`：按写入策略自动追加低风险 index 条目。
- `vault_propose_note_update`：为既有笔记生成替换内容提案，写入本地确认队列，但不直接覆盖原文件。

编辑工具遵守安全边界：创建新文件和低风险 index 追加可以自动执行；修改既有正文必须先进入确认队列，由用户在 Web UI 中确认后才会写入；删除、移动或重命名笔记仍未开放自动执行。
