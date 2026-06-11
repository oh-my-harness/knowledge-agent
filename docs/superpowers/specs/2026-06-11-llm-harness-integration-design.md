# LLM Harness 接入设计

## 状态

已确认，可以进入实现计划。

## 目标

把 `/api/ask` 从固定占位回复升级为通过 `oh-my-harness/llm-harness-core` 调用真实 LLM 的单轮回答。

本阶段只做最小 LLM 通路，不接入知识库检索、工具调用、网页搜索、streaming 或 session persistence。

## 边界

`llm-harness-core` 是 Agent SDK，不是 Knowledge Agent 的产品层。它负责 Agent、message、event、loop、session 等框架能力；Knowledge Agent 仍负责：

- 本地配置读取。
- HTTP API。
- 产品 system prompt。
- 错误展示。
- 后续 vault/search tools 的接入。

provider 调用尽量使用 `llm-harness-core` 示例中的 `llm_adapter`，本项目不自行实现模型 provider。

## 接入方式

新增 crate：

```text
crates/knowledge-agent-harness/
```

职责：

- 读取 `DEEPSEEK_API_KEY`。
- 读取可选 `DEEPSEEK_MODEL`，默认 `deepseek-v4-flash`。
- 创建 `llm_adapter::deepseek::client(api_key)`。
- 创建 `llm_harness::prelude::Agent`。
- 设置 Knowledge Agent system prompt。
- 执行单轮 prompt。
- 从 assistant messages 中提取 `ContentBlock::Text`。

## `/api/ask` 行为

请求结构保持不变：

```json
{
  "message": "什么是 Agent Harness？",
  "mode": "vault"
}
```

成功响应结构保持不变：

```json
{
  "answer": "...",
  "sources": [],
  "requires_followup": false
}
```

错误行为：

- 空消息继续返回 `400 Bad Request`。
- 未配置 `DEEPSEEK_API_KEY` 返回 `500`，错误文本包含 `DEEPSEEK_API_KEY`，方便用户定位。
- LLM 调用失败返回 `500`，错误文本来自 harness adapter。

## System Prompt

第一版 system prompt 必须诚实说明能力边界：

- 这是本地 Obsidian 知识库研究助手。
- 当前回答尚未接入 vault 检索。
- 不要声称已经阅读用户知识库。
- 如果需要本地知识库上下文，应明确说明下一阶段会接入。
- 回答使用中文，保持简洁。

## 测试策略

真实 LLM 调用不进入默认自动化测试，避免依赖外部网络和 API key。

测试分两层：

1. `knowledge-agent-harness` 提供可注入 runner trait。
   - 单元测试使用 fake runner，验证 server 能把 message 传给 runner。
   - 单元测试验证没有 API key 时返回明确错误。
2. `knowledge-agent-server` 使用 `AppState` 注入 ask service。
   - contract test 不调用真实 LLM。
   - 现有 `/api/ask` 测试改为断言 fake runner 的回答。

手动烟测：

```powershell
$env:DEEPSEEK_API_KEY="sk-..."
$env:DEEPSEEK_MODEL="deepseek-v4-flash"
cargo run -p knowledge-agent-cli -- serve crates/knowledge-agent-core/tests/fixtures/basic-vault --port 3030
```

然后在 Web UI 提问，确认返回不再是固定占位回复。

## 非目标

本阶段不做：

- OpenAI provider。
- provider registry。
- `.knowledge-agent/local.toml` 写入。
- 多轮 session。
- streaming。
- tool calls。
- vault 检索。
- 网页搜索。
- sources 填充。

## 后续衔接

下一阶段可以在不改前端 API 的情况下：

1. 在 `/api/ask` 前先做 Markdown index 检索。
2. 把选中的本地上下文写入 prompt。
3. 将本地笔记引用填入 `sources`。
4. 再把 vault/search tools 暴露给 `Agent`。
5. 最后引入 streaming 和 session persistence。

## 自检

- 只接入 LLM 单轮调用，范围明确。
- 不把 API key 写入 Git。
- 不伪装已有知识库检索能力。
- 测试不依赖外部 LLM。
