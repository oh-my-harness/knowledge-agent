use std::sync::Arc;

use knowledge_agent_harness::{
    AskError, AskRequest, AskRunner, DeepSeekAskRunner, FakeAskRunner, HarnessAskRunner,
    UnavailableAskRunner, vault_read_tools,
};
use llm_harness::prelude::{AgentMessage, ContentBlock};
use llm_harness_loop::{
    LlmClient,
    test_utils::{MockLlmClient, MockResponse},
};
use tempfile::TempDir;

#[tokio::test]
async fn fake_runner_returns_configured_answer() {
    let runner = FakeAskRunner::new("fake answer");

    let response = runner
        .ask(AskRequest {
            message: "hello".to_string(),
        })
        .await
        .unwrap();

    assert_eq!(response.answer, "fake answer");
}

#[tokio::test]
async fn deepseek_runner_reports_missing_api_key() {
    let result = DeepSeekAskRunner::from_env_with(|name| {
        if name == "DEEPSEEK_MODEL" {
            Some("deepseek-v4-flash".to_string())
        } else {
            None
        }
    });

    assert!(matches!(result, Err(AskError::MissingApiKey)));
}

#[tokio::test]
async fn unavailable_runner_returns_its_error() {
    let runner = UnavailableAskRunner::new(AskError::MissingApiKey);

    let result = runner
        .ask(AskRequest {
            message: "hello".to_string(),
        })
        .await;

    assert!(matches!(result, Err(AskError::MissingApiKey)));
}

#[tokio::test]
async fn harness_runner_keeps_context_between_turns() {
    let client = Arc::new(MockLlmClient::new(vec![
        MockResponse::text("first answer"),
        MockResponse::text("second answer"),
    ])) as Arc<dyn LlmClient>;
    let runner = HarnessAskRunner::new_in_memory(client, "test-model".to_string()).await;

    runner
        .ask(AskRequest {
            message: "first question".to_string(),
        })
        .await
        .unwrap();
    let second = runner
        .ask(AskRequest {
            message: "second question".to_string(),
        })
        .await
        .unwrap();

    assert_eq!(second.answer, "second answer");
    let messages = runner.context_messages().await.unwrap();
    assert!(
        messages.len() >= 4,
        "expected user/assistant messages from two turns, got {}",
        messages.len()
    );
}

#[tokio::test]
async fn harness_runner_reopens_jsonl_session() {
    let tmp = TempDir::new().unwrap();
    let first_client = Arc::new(MockLlmClient::new(vec![MockResponse::text("first answer")]))
        as Arc<dyn LlmClient>;
    let first_runner = HarnessAskRunner::new_jsonl(
        first_client,
        "test-model".to_string(),
        tmp.path(),
        "default".to_string(),
        Vec::new(),
    )
    .await
    .unwrap();

    first_runner
        .ask(AskRequest {
            message: "first question".to_string(),
        })
        .await
        .unwrap();
    drop(first_runner);

    let second_client = Arc::new(MockLlmClient::new(vec![MockResponse::text(
        "second answer",
    )])) as Arc<dyn LlmClient>;
    let second_runner = HarnessAskRunner::new_jsonl(
        second_client,
        "test-model".to_string(),
        tmp.path(),
        "default".to_string(),
        Vec::new(),
    )
    .await
    .unwrap();
    let second = second_runner
        .ask(AskRequest {
            message: "second question".to_string(),
        })
        .await
        .unwrap();

    assert_eq!(second.answer, "second answer");
    let messages = second_runner.context_messages().await.unwrap();
    assert!(
        messages.len() >= 4,
        "expected persisted messages from two turns, got {}",
        messages.len()
    );
}

#[tokio::test]
async fn harness_runner_executes_vault_read_tool() {
    let vault_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../knowledge-agent-core/tests/fixtures/basic-vault");
    let client = Arc::new(MockLlmClient::new(vec![
        MockResponse::tool_use(
            "tool-1",
            "vault_read_note",
            r#"{"path":"docs/concepts/agent-harness.md"}"#,
        ),
        MockResponse::text("我已经读取了 agent harness 笔记。"),
    ])) as Arc<dyn LlmClient>;
    let runner = HarnessAskRunner::new_in_memory_with_tools(
        client,
        "test-model".to_string(),
        vault_read_tools(vault_root),
    )
    .await;

    let response = runner
        .ask(AskRequest {
            message: "读取 agent harness 笔记".to_string(),
        })
        .await
        .unwrap();

    assert_eq!(response.answer, "我已经读取了 agent harness 笔记。");
    let messages = runner.context_messages().await.unwrap();
    let tool_text = messages.iter().find_map(|message| {
        let AgentMessage::ToolResult(result) = message else {
            return None;
        };
        result.content.iter().find_map(|block| {
            if let ContentBlock::Text { text } = block {
                Some(text)
            } else {
                None
            }
        })
    });
    assert!(
        tool_text.is_some_and(|text| text.contains("agent-harness.md")),
        "expected vault_read_note tool result in context"
    );
}
