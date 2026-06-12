# Knowledge Agent

Knowledge Agent 是一个本地运行的研究助手和 Obsidian vault 维护工具。

## 使用方式

在 Obsidian vault 根目录运行：

```powershell
knowledge-agent init .
knowledge-agent serve .
```

`init` 会创建共享配置 `.knowledge-agent.toml`，创建本机运行状态目录 `.knowledge-agent/`，并把 `.knowledge-agent/` 写入 vault 的 `.gitignore`。

如果当前目录或可执行文件所在目录存在 `web/dist/index.html`，服务会自动加载 Web UI。也可以显式指定前端静态文件目录：

```powershell
knowledge-agent serve . --web-dir .\web\dist
```

启动后访问：

```text
http://127.0.0.1:3030
```

开发时可以使用 fixture vault 运行：

```powershell
cargo run -p knowledge-agent-cli -- serve crates/knowledge-agent-core/tests/fixtures/basic-vault --port 3030
```

主要接口：

- `GET /api/health`
- `GET /api/vault/index`
- `GET /api/vault/pdfs`
- `POST /api/maintenance/scan`
- `GET /api/confirmations`
- `POST /api/confirmations/{id}/apply`
- `POST /api/confirmations/{id}/reject`
- `GET /api/ask/sessions`
- `POST /api/ask`

## Vault 文件约定

- `.knowledge-agent.toml` 是共享 vault 配置，可以随知识库提交。
- `.knowledge-agent/` 是本机运行状态和个人配置，应该被 Git 忽略。
- `.knowledge-agent/local.toml` 保存本机 LLM 和网页搜索配置。
- `.knowledge-agent/sessions/` 保存聊天会话。
- `.knowledge-agent/confirmations/` 保存待确认编辑提案。

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

## 打包

Windows 下可以使用：

```powershell
.\scripts\package.ps1
```

脚本会执行：

- `npm --prefix web install`
- `npm --prefix web run build`
- `cargo build --release -p knowledge-agent-cli`

产物输出到：

```text
dist/knowledge-agent/
dist/knowledge-agent.zip
```

运行打包产物：

```powershell
cd dist\knowledge-agent
.\knowledge-agent.exe init <你的 Obsidian vault 路径>
.\knowledge-agent.exe serve <你的 Obsidian vault 路径>
```

验证打包产物：

```powershell
.\scripts\verify-package.ps1
```

该脚本会创建临时 vault、执行 `init`、启动打包后的服务，并检查 Web 首页和 `/api/health`。

## 用户级安装

如果希望像 `git` 一样在任意目录直接运行 `knowledge-agent`，可以先打包，再安装到当前 Windows 用户：

```powershell
.\scripts\package.ps1
.\scripts\install-user.ps1
```

默认安装位置：

```text
%LOCALAPPDATA%\KnowledgeAgent
```

安装脚本会把该目录加入当前用户的 `PATH`，不需要管理员权限。安装完成后打开一个新的 PowerShell，然后运行：

```powershell
knowledge-agent --help
knowledge-agent init <你的 Obsidian vault 路径>
knowledge-agent serve <你的 Obsidian vault 路径>
```

如果只想测试复制安装目录，不修改 `PATH`：

```powershell
.\scripts\install-user.ps1 -InstallDir "$env:TEMP\knowledge-agent-install-test" -NoPath
```

卸载当前用户安装：

```powershell
.\scripts\uninstall-user.ps1
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

该文件保存本机 LLM provider、DeepSeek API key、模型名和网页搜索配置。保存后会自动重载后端 runner，并用于后续请求。

配置优先级：

1. `.knowledge-agent/local.toml`
2. 环境变量 `DEEPSEEK_API_KEY` / `DEEPSEEK_MODEL`
3. 默认模型名 `deepseek-v4-flash`

设置页不会展示环境变量中的真实 API Key，但会显示当前有效 Key 是否来自环境变量。若在设置页填写 API Key，保存后会写入本地配置，并立即优先用于后续请求。

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
- `vault_find_related_notes`：根据网页或 PDF 提取文本查找相关笔记，用于生成资料卡时建立 wikilink。
- `vault_list_pdf_assets`：列出 vault 内的 PDF 资源。
- `vault_read_pdf_text`：提取文本型 PDF 的文字内容，用于总结和资料卡生成。
- `web_search`：启用网页搜索后使用 DuckDuckGo 搜索公开网页，返回标题、链接和摘要。
- `web_fetch_page`：读取用户给定网页 URL，提取标题、描述和正文文本。

Web UI 提供“资料摄入”页面：

- 网页链接：提交 URL 后，agent 会读取网页、总结内容、查找相关笔记，并生成 Obsidian 资料卡建议。
- 本地 PDF：可输入或选择 vault 内 PDF 路径，agent 会提取 PDF 文本、总结内容、查找相关笔记，并生成资料卡建议。
- 资料卡写入遵守编辑安全边界：新建资料卡可自动执行，修改既有笔记正文必须进入确认队列。

当前也接入了第一版编辑工具：

- `vault_create_note`：在 vault 内创建新的 Markdown 笔记，不会覆盖已有文件。
- `vault_append_index_entry`：按写入策略自动追加低风险 index 条目。
- `vault_propose_note_update`：为既有笔记生成替换内容提案，写入本地确认队列，但不直接覆盖原文件。

编辑工具遵守安全边界：创建新文件和低风险 index 追加可以自动执行；修改既有正文必须先进入确认队列，由用户在 Web UI 中确认后才会写入；删除、移动或重命名笔记仍未开放自动执行。
