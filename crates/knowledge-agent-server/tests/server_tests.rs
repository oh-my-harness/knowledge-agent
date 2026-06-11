use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use knowledge_agent_server::{AppState, build_router};
use std::path::Path;
use tower::ServiceExt;

#[tokio::test]
async fn health_returns_ok() {
    let vault = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../knowledge-agent-core/tests/fixtures/basic-vault");
    let app = build_router(AppState::new(vault));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn maintenance_scan_returns_json() {
    let vault = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../knowledge-agent-core/tests/fixtures/basic-vault");
    let app = build_router(AppState::new(vault));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/maintenance/scan")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn maintenance_scan_errors_for_missing_vault() {
    let missing_vault = Path::new(env!("CARGO_MANIFEST_DIR")).join("missing-vault");
    let app = build_router(AppState::new(missing_vault));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/maintenance/scan")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn ask_returns_runner_answer() {
    let vault = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../knowledge-agent-core/tests/fixtures/basic-vault");
    let app = build_router(AppState::new_with_fake_ask_runner(vault, "fake llm answer"));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/ask")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"message":"什么是 Agent Harness？","mode":"vault"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["answer"], "fake llm answer");
}

#[tokio::test]
async fn ask_rejects_empty_message() {
    let vault = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../knowledge-agent-core/tests/fixtures/basic-vault");
    let app = build_router(AppState::new(vault));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/ask")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"message":"   ","mode":"vault"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn ask_reports_missing_llm_configuration() {
    unsafe {
        std::env::remove_var("DEEPSEEK_API_KEY");
    }

    let vault = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../knowledge-agent-core/tests/fixtures/basic-vault");
    let app = build_router(AppState::new(vault));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/ask")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"message":"hello","mode":"vault"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let text = String::from_utf8(body.to_vec()).unwrap();
    assert!(text.contains("DEEPSEEK_API_KEY"));
}
