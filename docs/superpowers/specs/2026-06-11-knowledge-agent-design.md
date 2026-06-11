# Knowledge Agent Design

## Status

Approved for planning.

## Goal

Build a local research assistant and knowledge maintenance tool that runs inside an Obsidian vault:

```bash
knowledge-agent serve .
```

The system helps a user research topics, write Obsidian-compatible Markdown reports, ask questions of the local vault, and maintain the health of the knowledge base. The vault remains the source of truth. All durable knowledge is stored as ordinary Markdown, frontmatter, wikilinks, tags, templates, and human-readable index documents.

## Non-Goals

- Do not use a vector database for the first version.
- Do not require Obsidian plugins.
- Do not auto-write semantic knowledge changes without user confirmation.
- Do not generate standalone source notes for every web page in the first version.
- Do not build a hosted or multi-user service.

## Product Shape

The first version has two core work areas:

1. Research Projects
   - Search the web.
   - Read and summarize relevant sources.
   - Check the local vault first.
   - Produce a Markdown research report.
   - Suggest concept cards for user approval.
   - Propose index updates.

2. Maintenance Inbox
   - Manually scan the vault.
   - Detect structure issues, broken links, stale indexes, duplicates, conflicts, and aging knowledge.
   - Automatically apply narrow low-risk fixes.
   - Send semantic changes to a review inbox with diffs.

The Web UI uses a draft-centered workbench: the research report draft is primary, with assistant chat, search results, local references, and concept suggestions beside it.

## Runtime Model

The app is developed as a Rust backend with a Web frontend.

The user runs it from a vault root:

```bash
knowledge-agent serve .
```

The backend serves a local HTTP API and Web UI. It treats the current working directory as the vault unless a path is explicitly supplied.

Personal runtime state is ignored by Git:

```text
.knowledge-agent/
  local.toml
  cache/
  sessions/
  logs/
```

Shared vault rules can be committed:

```text
.knowledge-agent.toml
```

This file describes vault conventions such as directories, templates, required frontmatter fields, naming rules, and index maintenance rules. Personal provider settings, API keys, ports, sessions, logs, and caches must stay under `.knowledge-agent/`.

## Vault Profile

The default vault profile follows an Obsidian-compatible Markdown structure inspired by `oh-my-harness/harness-knowledge`:

```text
vault/
  README.md
  .obsidian/
  .knowledge-agent.toml
  .knowledge-agent/
  docs/
    _index.md
    research/
      _index.md
      <project-slug>/
        report.md
    concepts/
      _index.md
      <concept-slug>.md
  templates/
    research-report.md
    concept-card.md
  assets/
```

Reports are written to:

```text
docs/research/<project-slug>/report.md
```

Confirmed concept cards are written to:

```text
docs/concepts/<concept-slug>.md
```

Source notes are not written by default. Web sources are cited inside the research report.

## Markdown Contract

Agent-generated Markdown must be readable in Obsidian and useful in Git review.

Required properties:

- YAML frontmatter.
- Stable title.
- `type` field such as `research`, `concept`, or `index`.
- Tags where useful.
- `created` and `updated` dates.
- Wikilinks for internal references.
- Plain Markdown citation sections for web sources.

Example:

```md
---
title: Example Topic
type: research
tags: [ai, agent]
created: 2026-06-11
updated: 2026-06-11
status: draft
---
```

Research reports must include a compact evidence section with local notes consulted, web sources cited, search scope, and limitations. Full agent logs stay in `.knowledge-agent/sessions/` unless the user exports them.

## Markdown Indexes

Indexes are Markdown documents, not JSON or a database:

```text
docs/_index.md
docs/research/_index.md
docs/concepts/_index.md
```

They are both human-readable navigation pages and agent-readable routing documents. The agent uses them to decide which notes to open before expanding through wikilinks and backlinks.

Index maintenance policy:

- Agent may propose index changes as diffs.
- Low-risk missing entries inside agent-managed sections may be automatically added.
- Semantic categorization, reorganization, or deletion requires user confirmation.

## Retrieval Model

The first version does not use embeddings or a vector database.

Retrieval uses:

- Root `README.md`.
- `docs/_index.md` and nested `_index.md` files.
- Frontmatter.
- Titles and headings.
- Tags.
- Wikilinks.
- Backlinks.
- File paths and updated dates.
- Short summaries present in index documents.

For vault question answering:

1. Read root and index documents.
2. Select likely index paths.
3. Open candidate Markdown notes.
4. Expand one or two hops through wikilinks and backlinks.
5. Ask the LLM to answer using the selected context.
6. Cite local note paths and titles.
7. If local knowledge is insufficient, optionally search the web and clearly separate web-derived material from vault-derived material.

## Research Project Flow

1. User creates a research project with a question, desired depth, and output style.
2. Agent checks the local vault first through Markdown indexes and link expansion.
3. Agent explains what relevant local knowledge already exists.
4. If needed, agent calls a pluggable web search provider.
5. Agent reads and filters web results.
6. Agent produces a Markdown report draft.
7. Agent suggests concept cards with title, summary, tags, path, and links to existing notes.
8. User confirms which concept cards to write.
9. Agent writes the report and confirmed concept cards.
10. Agent proposes or applies allowed index updates.
11. Follow-up chat stays in `.knowledge-agent/sessions/` by default.

The default behavior favors user confirmation for content that will shape the long-term knowledge network.

## Maintenance Service

Maintenance is manually triggered from the UI in the first version. A future CLI command can mirror it:

```bash
knowledge-agent maintain .
```

The scan creates a Maintenance Inbox with risk groups:

```text
P0 Broken knowledge
P1 Conflicts
P2 Index updates
P3 Suggested cleanups
```

Initial checks:

- Missing or malformed frontmatter.
- Stale or incomplete `_index.md` files.
- Broken wikilinks.
- Orphan notes.
- Possible duplicate concept cards.
- Possible conflicting definitions or conclusions.
- Notes that depend on external facts and may need review.

Low-risk fixes can be applied automatically only when they match a whitelist. Semantic changes require review.

## Write Policy

All file writes go through a shared `VaultWritePolicy`.

Allowed automatic writes:

- Add missing deterministic frontmatter fields.
- Update `updated` where a write is already happening.
- Add a new file to an agent-managed index section.
- Fix simple formatting issues.
- Mark broken links or orphan status in a non-semantic metadata section.

Writes requiring confirmation:

- Change body meaning.
- Merge notes.
- Delete notes.
- Rename, move, or retitle notes.
- Change concept definitions.
- Resolve conflicting claims.
- Update factual conclusions from web search.

Every reviewed write presents:

- Issue type.
- Files involved.
- Evidence.
- Suggested diff.
- Risk level.
- Actions: accept, edit then accept, ignore, or discuss.

No tool may bypass the write policy to edit vault files directly.

## LLM Harness Integration

Use `llm-harness-core` as the agent runtime layer:

- Agent loop.
- Messages.
- Sessions.
- Tool calls.
- Streaming responses.
- Compaction and skills where useful.

The application owns:

- Vault scanning.
- Markdown parsing and writing.
- Search providers.
- Fetch providers.
- Diff generation.
- Permissions and write policy.
- HTTP API.
- Web UI state.
- Settings and provider registry.

Tools exposed to the harness should include:

- Read note.
- Search vault indexes.
- Expand links and backlinks.
- Search web.
- Fetch web page.
- Draft report.
- Suggest concept cards.
- Propose diff.
- Apply approved diff.
- Run maintenance check.

## Backend Modules

Suggested Rust workspace:

```text
crates/
  knowledge-agent-cli/
  knowledge-agent-server/
  knowledge-agent-core/
  knowledge-agent-harness/
web/
```

Core module layout:

```text
vault/
  scanner.rs
  frontmatter.rs
  wikilinks.rs
  index_docs.rs
  graph.rs
  writer.rs
  policy.rs

research/
  project.rs
  report.rs
  concept_suggestions.rs

maintenance/
  checks/
  inbox.rs
  fixes.rs

providers/
  llm.rs
  search.rs
  fetch.rs

harness/
  agent.rs
  tools.rs
  sessions.rs
```

## Web UI

Main navigation:

- Projects
- Ask Vault
- Maintenance
- Vault
- Settings

Projects shows the draft-centered research workbench.

Ask Vault answers questions using local Markdown first, with optional web supplement.

Maintenance shows scan results, grouped issues, and diff review.

Vault shows index documents, recent notes, broken links, and orphan notes.

Settings writes personal provider configuration to `.knowledge-agent/local.toml`.

## HTTP API Sketch

```text
POST /api/research/projects
POST /api/research/{id}/run
POST /api/research/{id}/write
POST /api/ask
POST /api/maintenance/scan
POST /api/diffs/{id}/apply
GET  /api/vault/index
GET  /api/settings/local
PUT  /api/settings/local
```

Streaming endpoints should be used for long agent runs.

## Testing Strategy

Use fixture vaults rather than mocked strings wherever possible.

Test areas:

- Frontmatter parser and writer.
- Wikilink parser.
- Backlink graph construction.
- Markdown index parsing.
- Retrieval path selection.
- Write policy enforcement.
- Diff generation.
- Maintenance checks.
- Research report rendering.
- Concept card suggestion serialization.
- API contract tests.

Use `harness-knowledge` as a reference-style vault profile and fixture source where licensing and local setup allow.

## Open Decisions For Implementation Planning

- Which Rust web framework to use.
- Which frontend stack to use.
- Which default web search provider to implement first.
- Exact `llm-harness-core` adapter boundaries.
- Exact Markdown index grammar.
- Whether the first version includes a CLI-only maintenance command.
