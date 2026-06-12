use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use knowledge_agent_core::vault::confirmation::{
    CreateReplaceNoteConfirmation, create_replace_note_confirmation,
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
async fn local_settings_can_be_saved_and_loaded() {
    let vault = tempfile::tempdir().expect("tempdir");
    let app = build_router(AppState::new_with_fake_ask_runner(
        vault.path().to_path_buf(),
        "fake llm answer",
    ));

    let save_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/settings/local")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"llm":{"provider":"deepseek","deepseek_api_key":"secret","deepseek_model":"deepseek-chat"},"web_search":{"enabled":true,"provider":"duckduckgo"}}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(save_response.status(), StatusCode::OK);

    let load_response = app
        .oneshot(
            Request::builder()
                .uri("/api/settings/local")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(load_response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(load_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["llm"]["deepseek_model"], "deepseek-chat");
    assert_eq!(json["web_search"]["enabled"], true);
    assert_eq!(json["effective"]["deepseek_api_key_configured"], true);
    assert_eq!(json["effective"]["deepseek_api_key_source"], "local");
}

#[tokio::test]
async fn confirmations_can_be_listed_applied_and_rejected() {
    let vault = tempfile::tempdir().expect("tempdir");
    let note_path = vault.path().join("note.md");
    std::fs::write(&note_path, "# Old\n").unwrap();
    let apply_item = create_replace_note_confirmation(
        vault.path(),
        CreateReplaceNoteConfirmation {
            path: "note.md".to_string(),
            reason: Some("test apply".to_string()),
            proposed_content: "# New\n".to_string(),
        },
    )
    .unwrap();
    let reject_item = create_replace_note_confirmation(
        vault.path(),
        CreateReplaceNoteConfirmation {
            path: "note.md".to_string(),
            reason: Some("test reject".to_string()),
            proposed_content: "# Rejected\n".to_string(),
        },
    )
    .unwrap();
    let app = build_router(AppState::new_with_fake_ask_runner(
        vault.path().to_path_buf(),
        "fake llm answer",
    ));

    let list_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/confirmations")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list_response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(list_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["items"].as_array().unwrap().len(), 2);

    let apply_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/confirmations/{}/apply", apply_item.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(apply_response.status(), StatusCode::OK);
    assert_eq!(std::fs::read_to_string(&note_path).unwrap(), "# New\n");

    let reject_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/confirmations/{}/reject", reject_item.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reject_response.status(), StatusCode::OK);
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
async fn ask_sessions_can_be_listed_created_and_loaded() {
    let vault = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../knowledge-agent-core/tests/fixtures/basic-vault");
    let app = build_router(AppState::new_with_fake_ask_runner(vault, "fake llm answer"));

    let list_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/ask/sessions")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list_response.status(), StatusCode::OK);

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/ask/sessions")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"research"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::OK);

    let messages_response = app
        .oneshot(
            Request::builder()
                .uri("/api/ask/sessions/research/messages")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(messages_response.status(), StatusCode::OK);
}

#[tokio::test]
async fn ask_sessions_can_be_renamed_and_deleted() {
    let vault = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../knowledge-agent-core/tests/fixtures/basic-vault");
    let app = build_router(AppState::new_with_fake_ask_runner(vault, "fake llm answer"));

    let rename_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/ask/sessions/research")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"renamed"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(rename_response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(rename_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], "renamed");

    let delete_response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/ask/sessions/renamed")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete_response.status(), StatusCode::NO_CONTENT);
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
