# 聊天通路基础设计

## 状态

已确认，可以进入实现计划。

## 目标

为 Knowledge Agent 增加第一版“提问”通路，让 Web UI 可以把用户问题发送到 Rust 后端，并显示结构化助手回复。

本阶段只搭建稳定的 API、前端页面、类型和测试基础。它是后续接入 Markdown index 检索、wikilink/backlink 展开、网页搜索和 `llm-harness-core` agent runtime 的入口。

## 范围

本阶段实现：

- 后端新增 `POST /api/ask`。
- 前端新增“提问”导航页。
- 前端支持输入问题、发送、显示用户消息和助手回复。
- API client 增加 `askVault()`。
- 测试覆盖 Rust API contract、Web API client 和页面交互。

本阶段不实现：

- 不调用 `llm-harness-core`。
- 不调用模型 provider。
- 不搜索网页。
- 不读取或写入 vault 内容作为回答上下文。
- 不做 streaming response。
- 不保存 session 到 `.knowledge-agent/sessions/`。

## HTTP API

请求：

```http
POST /api/ask
Content-Type: application/json
```

```json
{
  "message": "什么是 Agent Harness？",
  "mode": "vault"
}
```

响应：

```json
{
  "answer": "已收到你的问题。后续这里会接入知识库检索和 LLM Harness。",
  "sources": [],
  "requires_followup": false
}
```

`mode` 第一版只接受 `vault`。后续可以扩展为 `research` 或 `web`，但本阶段不提前实现。

## 后端设计

在 `knowledge-agent-server` 中增加 ask route 和请求/响应类型。handler 只做输入校验和占位回复：

- `message` 去掉首尾空白后不能为空。
- 空消息返回 `400 Bad Request`。
- 非空消息返回固定占位回答。

占位回答要明确说明当前是通路层，后续会接入知识库检索和 LLM Harness，避免用户误以为已经有真实问答能力。

## 前端设计

在 `App.tsx` 导航中增加“提问”页面。页面布局保持工作台风格：

- 顶部标题为“提问”。
- 主区域显示消息列表。
- 底部是输入框和发送按钮。
- 发送时禁用按钮并显示等待状态。
- 请求成功后追加助手回答。
- 请求失败时显示错误文本，但保留用户输入历史。

当前不做多会话、不做 markdown 渲染、不做 source 展开，只显示普通文本。

## 数据类型

前端新增：

- `AskRequest`
- `AskResponse`
- `AskSource`

`AskSource` 第一版只保留：

- `title`
- `path`

虽然本阶段 `sources` 为空，但提前定义最小来源结构，方便下一阶段接入 vault 检索。

## 测试策略

Rust：

- `POST /api/ask` 对非空问题返回 `200` 和 JSON。
- 空问题返回 `400`。

Web：

- API client 测试 `askVault()` 使用 `POST /api/ask`，并序列化 message/mode。
- 页面测试用户输入问题、点击发送后显示用户消息和助手回答。
- 页面测试请求失败时显示错误。

## 后续衔接

下一阶段可以在不改前端页面入口的情况下替换后端实现：

1. 使用 Markdown index 初选候选笔记。
2. 沿 wikilink/backlink 展开邻近节点。
3. 把上下文和用户问题交给 `llm-harness-core`。
4. 在 `sources` 中返回本地笔记引用。
5. 最后再引入 streaming 和 session persistence。

## 自检

- 本设计聚焦单一通路，不包含真实 LLM 或检索逻辑。
- API 形状能承接后续知识库问答。
- 前端行为可测试，且不会误写 vault。
- 空输入有明确错误处理。
