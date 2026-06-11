# Knowledge Agent 设计

## 状态

已确认，可进入实现规划。

## 目标

构建一个运行在 Obsidian vault 内的本地研究助手和知识库维护工具：

```bash
knowledge-agent serve .
```

系统帮助用户研究主题、生成 Obsidian 兼容的 Markdown 研究报告、对本地 vault 提问，并维护知识库健康。vault 是唯一事实来源。所有长期知识都保存为普通 Markdown、frontmatter、wikilink、tag、模板和人类可读的索引文档。

## 非目标

- 第一版不使用向量数据库。
- 不要求安装 Obsidian 插件。
- 不在未经用户确认时自动写入语义性知识修改。
- 第一版不为每个网页来源生成独立 source note。
- 不做托管式或多用户服务。

## 产品形态

第一版有两个核心工作区：

1. 研究项目
   - 搜索网页。
   - 阅读和总结相关来源。
   - 优先检查本地 vault。
   - 生成 Markdown 研究报告。
   - 建议概念卡片，由用户确认后写入。
   - 提出索引更新。

2. 维护收件箱
   - 手动扫描 vault。
   - 检测结构问题、断链、过期索引、重复知识、冲突知识和可能老化的内容。
   - 自动应用范围很窄的低风险修复。
   - 将语义修改以 diff 形式送入 review inbox。

Web UI 使用“草稿中心”的工作台：研究报告草稿是主区域，助手聊天、搜索结果、本地引用和概念建议位于侧边。

## 运行模型

应用采用 Rust 后端和 Web 前端。

用户在 vault 根目录运行：

```bash
knowledge-agent serve .
```

后端提供本地 HTTP API 和 Web UI。默认把当前工作目录视为 vault，也允许显式传入路径。

个人运行状态不进入 Git：

```text
.knowledge-agent/
  local.toml
  cache/
  sessions/
  logs/
```

共享的 vault 维护规则可以提交：

```text
.knowledge-agent.toml
```

该文件描述 vault 约定，例如目录结构、模板路径、必填 frontmatter 字段、命名规则和索引维护规则。个人 provider 设置、API key、端口、sessions、日志和缓存必须保存在 `.knowledge-agent/` 下。

## Vault Profile

默认 vault profile 采用 Obsidian 兼容的 Markdown 结构，风格参考 `oh-my-harness/harness-knowledge`：

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

研究报告写入：

```text
docs/research/<project-slug>/report.md
```

用户确认后的概念卡片写入：

```text
docs/concepts/<concept-slug>.md
```

默认不写入独立 source note。网页来源只在研究报告中引用。

## Markdown 契约

agent 生成的 Markdown 必须能被 Obsidian 正常阅读，也要适合 Git review。

必备属性：

- YAML frontmatter。
- 稳定的标题。
- `type` 字段，例如 `research`、`concept` 或 `index`。
- 必要时添加 tags。
- `created` 和 `updated` 日期。
- 使用 wikilink 表达内部引用。
- 使用普通 Markdown 记录网页引用。

示例：

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

研究报告必须包含简洁证据区块：本地参考笔记、网页来源、搜索范围和局限性。完整 agent 日志默认留在 `.knowledge-agent/sessions/`，除非用户主动导出。

## Markdown 索引

索引是 Markdown 文档，不是 JSON，也不是数据库：

```text
docs/_index.md
docs/research/_index.md
docs/concepts/_index.md
```

它们既是人类可读的导航页，也是 agent 可读的路由文档。agent 先利用索引决定打开哪些笔记，再通过 wikilink 和 backlink 展开上下文。

索引维护策略：

- agent 可以提出索引修改 diff。
- agent 可以自动补充低风险的缺失条目，但仅限 agent 管理的索引区块。
- 语义分类、重组或删除必须经过用户确认。

## 检索模型

第一版不使用 embedding 或向量数据库。

检索使用：

- 根目录 `README.md`。
- `docs/_index.md` 和各层 `_index.md`。
- Frontmatter。
- 标题和 heading。
- Tags。
- Wikilinks。
- Backlinks。
- 文件路径和更新时间。
- 索引文档中的简短 summary。

知识库问答流程：

1. 读取根文档和索引文档。
2. 选择可能相关的索引路径。
3. 打开候选 Markdown 笔记。
4. 沿 wikilink 和 backlink 展开一到两跳邻近节点。
5. 让 LLM 基于选中的上下文回答。
6. 引用本地笔记路径和标题。
7. 如果本地知识不足，可选择搜索网页，并明确区分网页来源和 vault 来源。

## 研究项目流程

1. 用户创建研究项目，输入研究问题、期望深度和输出风格。
2. agent 先通过 Markdown 索引和链接展开检查本地 vault。
3. agent 说明本地已有的相关知识。
4. 如有需要，agent 调用可插拔网页搜索 provider。
5. agent 阅读和筛选网页结果。
6. agent 生成 Markdown 报告草稿。
7. agent 建议概念卡片，包含标题、摘要、tags、路径和与现有笔记的链接关系。
8. 用户确认哪些概念卡片需要写入。
9. agent 写入报告和已确认的概念卡片。
10. agent 提出或应用允许范围内的索引更新。
11. 后续追问默认保存在 `.knowledge-agent/sessions/`。

默认行为偏向用户确认，尤其是会长期影响知识网络的内容。

## 维护服务

第一版维护服务只由用户手动触发。未来可以提供等价 CLI 命令：

```bash
knowledge-agent maintain .
```

扫描会生成 Maintenance Inbox，并按风险分组：

```text
P0 Broken knowledge
P1 Conflicts
P2 Index updates
P3 Suggested cleanups
```

初始检查范围：

- 缺失或格式错误的 frontmatter。
- 过期或不完整的 `_index.md`。
- 断开的 wikilink。
- 孤立笔记。
- 可能重复的概念卡片。
- 可能冲突的定义或结论。
- 依赖外部事实且可能需要复核的笔记。

低风险修复只有匹配白名单时才能自动应用。语义修改必须 review。

## 写入策略

所有文件写入都必须经过统一的 `VaultWritePolicy`。

允许自动写入：

- 补充确定性的缺失 frontmatter 字段。
- 在已经发生写入时更新 `updated`。
- 将新文件加入 agent 管理的索引区块。
- 修复简单格式问题。
- 在非语义 metadata 区块中标记断链或孤立状态。

必须确认的写入：

- 修改正文含义。
- 合并笔记。
- 删除笔记。
- 重命名、移动或改标题。
- 修改概念定义。
- 解决冲突断言。
- 根据网页搜索更新事实结论。

每个需要 review 的写入都展示：

- 问题类型。
- 涉及文件。
- 证据。
- 建议 diff。
- 风险等级。
- 操作：接受、编辑后接受、忽略或讨论。

任何 tool 都不能绕过写入策略直接编辑 vault 文件。

## LLM Harness 集成

使用 `llm-harness-core` 作为 agent runtime 层：

- Agent loop。
- Messages。
- Sessions。
- Tool calls。
- Streaming responses。
- 必要时使用 compaction 和 skills。

应用自己负责：

- Vault 扫描。
- Markdown 解析和写入。
- Search providers。
- Fetch providers。
- Diff 生成。
- 权限和写入策略。
- HTTP API。
- Web UI 状态。
- Settings 和 provider registry。

暴露给 harness 的工具包括：

- 读取笔记。
- 搜索 vault 索引。
- 展开 links 和 backlinks。
- 搜索网页。
- 抓取网页。
- 起草报告。
- 建议概念卡片。
- 提出 diff。
- 应用已批准的 diff。
- 运行维护检查。

## 后端模块

建议 Rust workspace：

```text
crates/
  knowledge-agent-cli/
  knowledge-agent-server/
  knowledge-agent-core/
  knowledge-agent-harness/
web/
```

核心模块布局：

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

主导航：

- Projects
- Ask Vault
- Maintenance
- Vault
- Settings

Projects 显示草稿中心的研究工作台。

Ask Vault 优先使用本地 Markdown 回答，可选网页补充。

Maintenance 显示扫描结果、分组问题和 diff review。

Vault 显示索引文档、最近笔记、断链和孤立笔记。

Settings 将个人 provider 配置写入 `.knowledge-agent/local.toml`。

## HTTP API 草案

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

长时间 agent 任务应使用 streaming endpoint。

## 测试策略

尽量使用 fixture vault，而不是只 mock 字符串。

测试范围：

- Frontmatter 解析和写入。
- Wikilink 解析。
- Backlink graph 构建。
- Markdown index 解析。
- 检索路径选择。
- 写入策略强制执行。
- Diff 生成。
- 维护检查。
- 研究报告渲染。
- 概念卡片建议序列化。
- API contract tests。

在许可和本地设置允许的情况下，使用 `harness-knowledge` 作为参考风格的 vault profile 和 fixture 来源。

## 实现规划前的待定项

- 选择 Rust web framework。
- 选择前端技术栈。
- 选择第一版默认网页搜索 provider。
- 明确 `llm-harness-core` adapter 边界。
- 明确 Markdown index grammar。
- 决定第一版是否包含 CLI-only 维护命令。
