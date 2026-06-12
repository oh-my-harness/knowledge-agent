use crate::state::AppState;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::sse::{Event, KeepAlive, Sse},
    routing::{delete, get, patch, post},
};
use futures::{Stream, stream};
use knowledge_agent_core::{
    maintenance::checks::run_maintenance_scan,
    settings::{LocalSettings, load_local_settings, save_local_settings},
    vault::{
        confirmation::{apply_confirmation, list_confirmations, reject_confirmation},
        scanner::scan_vault,
    },
};
use knowledge_agent_harness as harness;
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, path::PathBuf, sync::Arc, time::Duration};
use tokio::sync::broadcast;
use tower_http::services::{ServeDir, ServeFile};

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
struct AskEventsQuery {
    session_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreateSessionRequest {
    name: String,
}

#[derive(Debug, Deserialize)]
struct RenameSessionRequest {
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

#[derive(Debug, Serialize)]
struct AskActivityEvent {
    kind: &'static str,
    label: String,
    detail: Option<String>,
}

#[derive(Debug, Serialize)]
struct LocalSettingsResponse {
    #[serde(flatten)]
    settings: LocalSettings,
    effective: EffectiveLocalSettings,
}

#[derive(Debug, Serialize)]
struct EffectiveLocalSettings {
    deepseek_api_key_configured: bool,
    deepseek_api_key_source: Option<&'static str>,
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/vault/index", get(vault_index))
        .route("/api/maintenance/scan", post(maintenance_scan))
        .route("/api/confirmations", get(confirmations))
        .route(
            "/api/confirmations/{confirmation_id}/apply",
            post(apply_confirmation_route),
        )
        .route(
            "/api/confirmations/{confirmation_id}/reject",
            post(reject_confirmation_route),
        )
        .route("/api/settings/local", get(local_settings))
        .route("/api/settings/local", post(save_settings))
        .route("/api/ask/sessions", get(list_ask_sessions))
        .route("/api/ask/sessions", post(create_ask_session))
        .route("/api/ask/sessions/{session_id}", patch(rename_ask_session))
        .route("/api/ask/sessions/{session_id}", delete(delete_ask_session))
        .route(
            "/api/ask/sessions/{session_id}/messages",
            get(ask_session_messages),
        )
        .route("/api/ask/events", get(ask_events))
        .route("/api/ask", post(ask))
        .with_state(state)
}

pub fn build_router_with_static(state: AppState, web_dir: PathBuf) -> Router {
    let service =
        ServeDir::new(&web_dir).not_found_service(ServeFile::new(web_dir.join("index.html")));
    build_router(state).fallback_service(service)
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

async fn confirmations(State(state): State<AppState>) -> ApiResult<impl Serialize> {
    list_confirmations(state.vault_root.as_ref())
        .map(Json)
        .map_err(internal_error)
}

async fn apply_confirmation_route(
    State(state): State<AppState>,
    Path(confirmation_id): Path<String>,
) -> ApiResult<impl Serialize> {
    apply_confirmation(state.vault_root.as_ref(), &confirmation_id)
        .map(Json)
        .map_err(internal_error)
}

async fn reject_confirmation_route(
    State(state): State<AppState>,
    Path(confirmation_id): Path<String>,
) -> ApiResult<impl Serialize> {
    reject_confirmation(state.vault_root.as_ref(), &confirmation_id)
        .map(Json)
        .map_err(internal_error)
}

async fn local_settings(State(state): State<AppState>) -> ApiResult<LocalSettingsResponse> {
    load_local_settings(&state.vault_root)
        .map(settings_response)
        .map(Json)
        .map_err(internal_error)
}

async fn save_settings(
    State(state): State<AppState>,
    Json(settings): Json<LocalSettings>,
) -> ApiResult<LocalSettingsResponse> {
    save_local_settings(&state.vault_root, &settings).map_err(internal_error)?;
    state.reload_ask_runner();
    Ok(Json(settings_response(settings)))
}

fn settings_response(settings: LocalSettings) -> LocalSettingsResponse {
    let local_key_configured = settings
        .llm
        .deepseek_api_key
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty());
    let env_key_configured =
        std::env::var("DEEPSEEK_API_KEY").is_ok_and(|value| !value.trim().is_empty());
    let deepseek_api_key_source = if local_key_configured {
        Some("local")
    } else if env_key_configured {
        Some("environment")
    } else {
        None
    };

    LocalSettingsResponse {
        settings,
        effective: EffectiveLocalSettings {
            deepseek_api_key_configured: local_key_configured || env_key_configured,
            deepseek_api_key_source,
        },
    }
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
                .ask_runner()
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
        .ask_runner()
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
        .ask_runner()
        .create_session(request.name)
        .await
        .map(Json)
        .map_err(ask_error)
}

async fn rename_ask_session(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Json(request): Json<RenameSessionRequest>,
) -> ApiResult<impl Serialize> {
    if request.name.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "session name cannot be empty".to_string(),
        ));
    }

    state
        .ask_runner()
        .rename_session(session_id, request.name)
        .await
        .map(Json)
        .map_err(ask_error)
}

async fn delete_ask_session(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    state
        .ask_runner()
        .delete_session(session_id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(ask_error)
}

async fn ask_session_messages(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> ApiResult<impl Serialize> {
    state
        .ask_runner()
        .session_messages(session_id)
        .await
        .map(Json)
        .map_err(ask_error)
}

async fn ask_events(
    State(state): State<AppState>,
    Query(query): Query<AskEventsQuery>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, (StatusCode, String)> {
    let session_id = query.session_id.unwrap_or_else(|| "default".to_string());
    let receiver = state
        .ask_runner()
        .subscribe_events(session_id)
        .await
        .map_err(ask_error)?;
    let stream = stream::unfold(receiver, |mut receiver| async move {
        loop {
            match receiver.recv().await {
                Ok(event) => {
                    return Some((Ok(to_sse_event(&event)), receiver));
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    });

    Ok(Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    ))
}

fn to_sse_event(event: &Arc<harness::AgentHarnessEvent>) -> Event {
    let payload = to_activity_event(event.as_ref());
    Event::default()
        .event("agent")
        .json_data(payload)
        .expect("activity event serializes")
}

fn to_activity_event(event: &harness::AgentHarnessEvent) -> AskActivityEvent {
    match event {
        harness::AgentHarnessEvent::Agent(agent_event) => to_agent_activity_event(agent_event),
        harness::AgentHarnessEvent::ToolCallStart { tool_name, .. } => AskActivityEvent {
            kind: "tool_call_start",
            label: format!("准备使用工具：{}", tool_label(tool_name)),
            detail: Some(tool_name.clone()),
        },
        harness::AgentHarnessEvent::ToolCallEnd {
            tool_name, result, ..
        } => AskActivityEvent {
            kind: "tool_execution_end",
            label: if result.is_error {
                "工具执行失败".to_string()
            } else {
                "工具执行完成".to_string()
            },
            detail: Some(tool_name.clone()),
        },
        harness::AgentHarnessEvent::Settled => AskActivityEvent {
            kind: "agent_end",
            label: "完成".to_string(),
            detail: None,
        },
        harness::AgentHarnessEvent::SavePoint { .. } => AskActivityEvent {
            kind: "save_point",
            label: "正在保存会话".to_string(),
            detail: None,
        },
        harness::AgentHarnessEvent::PhaseChange { .. } => AskActivityEvent {
            kind: "running",
            label: "正在处理".to_string(),
            detail: None,
        },
        _ => AskActivityEvent {
            kind: "running",
            label: "正在处理".to_string(),
            detail: None,
        },
    }
}

fn to_agent_activity_event(event: &harness::AgentEvent) -> AskActivityEvent {
    match event {
        harness::AgentEvent::AgentStart { .. } => AskActivityEvent {
            kind: "agent_start",
            label: "开始处理".to_string(),
            detail: None,
        },
        harness::AgentEvent::ThinkingDelta { .. } => AskActivityEvent {
            kind: "thinking",
            label: "正在思考".to_string(),
            detail: None,
        },
        harness::AgentEvent::ToolCallStart { name, .. } => AskActivityEvent {
            kind: "tool_call_start",
            label: format!("准备使用工具：{}", tool_label(name)),
            detail: Some(name.clone()),
        },
        harness::AgentEvent::ToolExecutionStart { tool_name, .. } => AskActivityEvent {
            kind: "tool_execution_start",
            label: format!("正在{}", tool_action(tool_name)),
            detail: Some(tool_name.clone()),
        },
        harness::AgentEvent::ToolExecutionEnd { result, .. } => AskActivityEvent {
            kind: "tool_execution_end",
            label: if result.is_ok() {
                "工具执行完成".to_string()
            } else {
                "工具执行失败".to_string()
            },
            detail: result.as_ref().err().map(|err| err.to_string()),
        },
        harness::AgentEvent::TextDelta { .. } | harness::AgentEvent::MessageUpdate { .. } => {
            AskActivityEvent {
                kind: "answering",
                label: "正在生成回答".to_string(),
                detail: None,
            }
        }
        harness::AgentEvent::AgentEnd { .. } => AskActivityEvent {
            kind: "agent_end",
            label: "完成".to_string(),
            detail: None,
        },
        harness::AgentEvent::Error(err) => AskActivityEvent {
            kind: "error",
            label: "处理失败".to_string(),
            detail: Some(err.to_string()),
        },
        _ => AskActivityEvent {
            kind: "running",
            label: "正在处理".to_string(),
            detail: None,
        },
    }
}

fn tool_label(name: &str) -> &str {
    match name {
        "vault_list_notes" => "列出笔记",
        "vault_read_note" => "读取笔记",
        "vault_search_notes" => "搜索笔记",
        "vault_neighbor_notes" => "查看相邻节点",
        "web_search" => "搜索网页",
        "vault_create_note" => "新建笔记",
        "vault_append_index_entry" => "维护索引",
        "vault_propose_note_update" => "提出笔记修改",
        _ => name,
    }
}

fn tool_action(name: &str) -> &str {
    match name {
        "vault_list_notes" => "列出知识库笔记",
        "vault_read_note" => "读取笔记",
        "vault_search_notes" => "搜索知识库",
        "vault_neighbor_notes" => "查看链接邻近节点",
        "web_search" => "搜索网页",
        "vault_create_note" => "新建笔记",
        "vault_append_index_entry" => "维护索引",
        "vault_propose_note_update" => "生成笔记修改提案",
        _ => "执行工具",
    }
}

fn internal_error(err: anyhow::Error) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

fn ask_error(err: harness::AskError) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
