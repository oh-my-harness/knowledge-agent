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

如果没有设置 `DEEPSEEK_API_KEY`，`/api/ask` 会返回明确错误。API key 属于个人配置，不应提交到 Git；后续会迁移到 `.knowledge-agent/local.toml`。

聊天会话由 `llm-harness-core` 的 session 机制保存到：

```text
.knowledge-agent/sessions/
```

该目录是个人运行状态，已经被 `.gitignore` 忽略，不应提交到远端知识库。
