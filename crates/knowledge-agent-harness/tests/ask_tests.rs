use std::sync::Arc;

use knowledge_agent_harness::{
    AskError, AskRequest, AskRunner, DeepSeekAskRunner, FakeAskRunner, HarnessAskRunner,
    UnavailableAskRunner,
};
use llm_harness_loop::{
    LlmClient,
    test_utils::{MockLlmClient, MockResponse},
};

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
