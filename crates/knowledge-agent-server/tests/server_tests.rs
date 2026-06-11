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
