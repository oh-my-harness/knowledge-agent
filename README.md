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
