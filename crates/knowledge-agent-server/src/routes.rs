use crate::state::AppState;
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use knowledge_agent_core::{maintenance::checks::run_maintenance_scan, vault::scanner::scan_vault};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/vault/index", get(vault_index))
        .route("/api/maintenance/scan", post(maintenance_scan))
        .with_state(state)
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

type ApiResult<T> = Result<Json<T>, (StatusCode, String)>;

async fn vault_index(State(state): State<AppState>) -> ApiResult<impl Serialize> {
    scan_vault(&state.vault_root)
        .map(Json)
        .map_err(internal_error)
}

async fn maintenance_scan(State(state): State<AppState>) -> ApiResult<impl Serialize> {
    run_maintenance_scan(&state.vault_root)
        .map(Json)
        .map_err(internal_error)
}

fn internal_error(err: anyhow::Error) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
