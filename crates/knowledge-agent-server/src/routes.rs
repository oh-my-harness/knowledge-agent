use crate::state::AppState;
use axum::{
    Json, Router,
    extract::State,
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

async fn vault_index(State(state): State<AppState>) -> Result<Json<impl Serialize>, String> {
    scan_vault(&state.vault_root)
        .map(Json)
        .map_err(|err| err.to_string())
}

async fn maintenance_scan(State(state): State<AppState>) -> Result<Json<impl Serialize>, String> {
    run_maintenance_scan(&state.vault_root)
        .map(Json)
        .map_err(|err| err.to_string())
}
