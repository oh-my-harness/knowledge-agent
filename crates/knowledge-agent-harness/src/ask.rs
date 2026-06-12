use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use async_trait::async_trait;
use llm_adapter::deepseek;
use llm_harness::prelude::{
    AgentHarness, AgentHarnessOptions, AgentMessage, ContentBlock, ExecutionEnv, JsonlSessionRepo,
    Session, SessionRepo, UnsupportedEnv,
};
use llm_harness::session::{CreateSessionOptions, ListSessionOptions};
use llm_harness_loop::LlmClient;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::Mutex;

const DEFAULT_DEEPSEEK_MODEL: &str = "deepseek-v4-flash";
const SYSTEM_PROMPT: &str = "你是 Knowledge Agent，一个本地 Obsidian 知识库研究助手。当前版本尚未接入 vault 检索，所以不要声称已经阅读用户的本地知识库。请用中文简洁回答；如果问题需要本地知识库上下文，请说明需要后续接入检索后才能准确回答。";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AskRequest {
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct AskResponse {
    pub answer: String,
}

#[derive(Debug, Clone, Error)]
pub enum AskError {
    #[error("missing DEEPSEEK_API_KEY")]
    MissingApiKey,
    #[error("llm returned no assistant text")]
    EmptyAnswer,
    #[error("session error: {0}")]
    Session(String),
    #[error("llm harness error: {0}")]
    Harness(String),
}

#[async_trait]
pub trait AskRunner: Send + Sync {
    async fn ask(&self, request: AskRequest) -> Result<AskResponse, AskError>;
}

#[derive(Debug)]
pub struct FakeAskRunner {
    answer: String,
}

impl FakeAskRunner {
    pub fn new(answer: impl Into<String>) -> Self {
        Self {
            answer: answer.into(),
        }
    }
}

#[async_trait]
impl AskRunner for FakeAskRunner {
    async fn ask(&self, _request: AskRequest) -> Result<AskResponse, AskError> {
        Ok(AskResponse {
            answer: self.answer.clone(),
        })
    }
}

pub struct UnavailableAskRunner {
    error: AskError,
}

impl UnavailableAskRunner {
    pub fn new(error: AskError) -> Self {
        Self { error }
    }
}

#[async_trait]
impl AskRunner for UnavailableAskRunner {
    async fn ask(&self, _request: AskRequest) -> Result<AskResponse, AskError> {
        Err(self.error.clone())
    }
}

pub struct DeepSeekAskRunner {
    api_key: String,
    model: String,
    sessions_root: Option<PathBuf>,
    session_name: String,
    harness: Mutex<Option<HarnessAskRunner>>,
}

impl DeepSeekAskRunner {
    pub fn from_env() -> Result<Self, AskError> {
        Self::from_env_with(|name| std::env::var(name).ok())
    }

    pub fn from_env_with_sessions_root(
        sessions_root: impl Into<PathBuf>,
    ) -> Result<Self, AskError> {
        Self::from_env_with_options(
            |name| std::env::var(name).ok(),
            Some(sessions_root.into()),
            "default".to_string(),
        )
    }

    pub fn from_env_with(get_var: impl Fn(&str) -> Option<String>) -> Result<Self, AskError> {
        Self::from_env_with_options(get_var, None, "default".to_string())
    }

    pub fn from_env_with_options(
        get_var: impl Fn(&str) -> Option<String>,
        sessions_root: Option<PathBuf>,
        session_name: String,
    ) -> Result<Self, AskError> {
        let api_key = get_var("DEEPSEEK_API_KEY")
            .filter(|value| !value.trim().is_empty())
            .ok_or(AskError::MissingApiKey)?;
        let model = get_var("DEEPSEEK_MODEL")
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_DEEPSEEK_MODEL.to_string());

        Ok(Self {
            api_key,
            model,
            sessions_root,
            session_name,
            harness: Mutex::new(None),
        })
    }
}

#[async_trait]
impl AskRunner for DeepSeekAskRunner {
    async fn ask(&self, request: AskRequest) -> Result<AskResponse, AskError> {
        let mut harness = self.harness.lock().await;
        if harness.is_none() {
            let client = Arc::new(deepseek::client(self.api_key.clone())) as Arc<dyn LlmClient>;
            *harness = Some(if let Some(sessions_root) = &self.sessions_root {
                HarnessAskRunner::new_jsonl(
                    client,
                    self.model.clone(),
                    sessions_root,
                    self.session_name.clone(),
                )
                .await?
            } else {
                HarnessAskRunner::new_in_memory(client, self.model.clone()).await
            });
        }

        harness
            .as_ref()
            .expect("harness initialized before ask")
            .ask(request)
            .await
    }
}

pub struct HarnessAskRunner {
    harness: Mutex<AgentHarness>,
}

impl HarnessAskRunner {
    pub async fn new_in_memory(client: Arc<dyn LlmClient>, model: String) -> Self {
        Self::new_in_memory_with_env(client, Arc::new(UnsupportedEnv::new()), model).await
    }

    pub async fn new_jsonl(
        client: Arc<dyn LlmClient>,
        model: String,
        sessions_root: impl AsRef<Path>,
        session_name: String,
    ) -> Result<Self, AskError> {
        Self::new_jsonl_with_env(
            client,
            Arc::new(UnsupportedEnv::new()),
            model,
            sessions_root,
            session_name,
        )
        .await
    }

    pub async fn new_in_memory_with_env(
        client: Arc<dyn LlmClient>,
        env: Arc<dyn ExecutionEnv>,
        model: String,
    ) -> Self {
        let mut options = AgentHarnessOptions::new(model);
        options.system_prompt = Some(SYSTEM_PROMPT.to_string());
        let harness = AgentHarness::new_in_memory(client, env, options).await;

        Self {
            harness: Mutex::new(harness),
        }
    }

    pub async fn new_jsonl_with_env(
        client: Arc<dyn LlmClient>,
        env: Arc<dyn ExecutionEnv>,
        model: String,
        sessions_root: impl AsRef<Path>,
        session_name: String,
    ) -> Result<Self, AskError> {
        tokio::fs::create_dir_all(sessions_root.as_ref())
            .await
            .map_err(|err| AskError::Session(err.to_string()))?;

        let repo = JsonlSessionRepo::new(sessions_root.as_ref());
        let existing = repo
            .list(ListSessionOptions {
                name_contains: Some(session_name.clone()),
                ..ListSessionOptions::default()
            })
            .await
            .map_err(|err| AskError::Session(err.to_string()))?
            .into_iter()
            .find(|metadata| metadata.name.as_deref() == Some(session_name.as_str()));

        let storage = if let Some(metadata) = existing {
            repo.open(&metadata.id)
                .await
                .map_err(|err| AskError::Session(err.to_string()))?
        } else {
            repo.create(CreateSessionOptions {
                name: Some(session_name),
                initial_model: Some(model.clone()),
                ..CreateSessionOptions::default()
            })
            .await
            .map_err(|err| AskError::Session(err.to_string()))?
        };

        let mut options = AgentHarnessOptions::new(model);
        options.system_prompt = Some(SYSTEM_PROMPT.to_string());
        let harness = AgentHarness::with_session(client, env, Session::new(storage), options);

        Ok(Self {
            harness: Mutex::new(harness),
        })
    }

    pub async fn context_messages(&self) -> Result<Vec<AgentMessage>, AskError> {
        let harness = self.harness.lock().await;
        let context = harness
            .build_context()
            .await
            .map_err(|err| AskError::Harness(err.to_string()))?;
        Ok(context.messages)
    }
}

#[async_trait]
impl AskRunner for HarnessAskRunner {
    async fn ask(&self, request: AskRequest) -> Result<AskResponse, AskError> {
        let harness = self.harness.lock().await;
        let message_start = harness
            .build_context()
            .await
            .map_err(|err| AskError::Harness(err.to_string()))?
            .messages
            .len();

        harness
            .prompt(request.message)
            .await
            .map_err(|err| AskError::Harness(err.to_string()))?;

        let context = harness
            .build_context()
            .await
            .map_err(|err| AskError::Harness(err.to_string()))?;
        let answer = assistant_text(&context.messages[message_start..]);
        if answer.trim().is_empty() {
            return Err(AskError::EmptyAnswer);
        }

        Ok(AskResponse { answer })
    }
}

fn assistant_text(messages: &[AgentMessage]) -> String {
    let mut output = String::new();

    for message in messages {
        let AgentMessage::Assistant(assistant) = message else {
            continue;
        };

        for block in &assistant.content {
            if let ContentBlock::Text { text } = block {
                output.push_str(text);
            }
        }
    }

    output
}
