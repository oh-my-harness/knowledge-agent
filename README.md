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
