use knowledge_agent_harness::{
    AskError, AskRequest, AskRunner, DeepSeekAskRunner, FakeAskRunner, UnavailableAskRunner,
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
