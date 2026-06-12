use crate::state::AppState;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use knowledge_agent_core::{
    maintenance::checks::run_maintenance_scan,
    settings::{LocalSettings, load_local_settings, save_local_settings},
    vault::scanner::scan_vault,
};
use knowledge_agent_harness as harness;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Debug, Deserialize)]
struct AskRequest {
    message: String,
    session_id: Option<String>,
    mode: AskMode,
}

#[derive(Debug, Deserialize)]
struct CreateSessionRequest {
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum AskMode {
    Vault,
}

#[derive(Debug, Serialize)]
struct AskResponse {
    answer: String,
    sources: Vec<AskSource>,
    requires_followup: bool,
}

#[derive(Debug, Serialize)]
struct AskSource {
    title: String,
    path: String,
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/vault/index", get(vault_index))
        .route("/api/maintenance/scan", post(maintenance_scan))
        .route("/api/settings/local", get(local_settings))
        .route("/api/settings/local", post(save_settings))
        .route("/api/ask/sessions", get(list_ask_sessions))
        .route("/api/ask/sessions", post(create_ask_session))
        .route(
            "/api/ask/sessions/{session_id}/messages",
            get(ask_session_messages),
        )
        .route("/api/ask", post(ask))
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

async fn local_settings(State(state): State<AppState>) -> ApiResult<LocalSettings> {
    load_local_settings(&state.vault_root)
        .map(Json)
        .map_err(internal_error)
}

async fn save_settings(
    State(state): State<AppState>,
    Json(settings): Json<LocalSettings>,
) -> ApiResult<LocalSettings> {
    save_local_settings(&state.vault_root, &settings).map_err(internal_error)?;
    Ok(Json(settings))
}

async fn ask(
    State(state): State<AppState>,
    Json(request): Json<AskRequest>,
) -> ApiResult<AskResponse> {
    if request.message.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "message cannot be empty".to_string(),
        ));
    }

    let answer = match request.mode {
        AskMode::Vault => {
            state
                .ask_runner
                .ask(harness::AskRequest {
                    message: request.message,
                    session_id: request.session_id,
                })
                .await
                .map_err(ask_error)?
                .answer
        }
    };

    Ok(Json(AskResponse {
        answer,
        sources: Vec::new(),
        requires_followup: false,
    }))
}

async fn list_ask_sessions(State(state): State<AppState>) -> ApiResult<impl Serialize> {
    state
        .ask_runner
        .list_sessions()
        .await
        .map(Json)
        .map_err(ask_error)
}

async fn create_ask_session(
    State(state): State<AppState>,
    Json(request): Json<CreateSessionRequest>,
) -> ApiResult<impl Serialize> {
    if request.name.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "session name cannot be empty".to_string(),
        ));
    }

    state
        .ask_runner
        .create_session(request.name)
        .await
        .map(Json)
        .map_err(ask_error)
}

async fn ask_session_messages(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> ApiResult<impl Serialize> {
    state
        .ask_runner
        .session_messages(session_id)
        .await
        .map(Json)
        .map_err(ask_error)
}

fn internal_error(err: anyhow::Error) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

fn ask_error(err: harness::AskError) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
