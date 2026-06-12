use std::{path::PathBuf, sync::Arc};

use knowledge_agent_core::settings::load_local_settings;
use knowledge_agent_harness::{AskRunner, DeepSeekAskRunner, FakeAskRunner, UnavailableAskRunner};

#[derive(Clone)]
pub struct AppState {
    pub vault_root: Arc<PathBuf>,
    pub ask_runner: Arc<dyn AskRunner>,
}

impl AppState {
    pub fn new(vault_root: PathBuf) -> Self {
        let sessions_root = vault_root.join(".knowledge-agent").join("sessions");
        let local_settings = load_local_settings(&vault_root).unwrap_or_default();
        let ask_runner: Arc<dyn AskRunner> = match DeepSeekAskRunner::from_env_with_options(
            |name| {
                std::env::var(name).ok().or_else(|| match name {
                    "DEEPSEEK_API_KEY" => local_settings.llm.deepseek_api_key.clone(),
                    "DEEPSEEK_MODEL" => Some(local_settings.llm.deepseek_model.clone()),
                    _ => None,
                })
            },
            Some(sessions_root),
            "default".to_string(),
            Some(vault_root.clone()),
        ) {
            Ok(runner) => Arc::new(runner),
            Err(err) => Arc::new(UnavailableAskRunner::new(err)),
        };

        Self {
            vault_root: Arc::new(vault_root),
            ask_runner,
        }
    }

    pub fn new_with_ask_runner(vault_root: PathBuf, ask_runner: Arc<dyn AskRunner>) -> Self {
        Self {
            vault_root: Arc::new(vault_root),
            ask_runner,
        }
    }

    pub fn new_with_fake_ask_runner(vault_root: PathBuf, answer: impl Into<String>) -> Self {
        Self::new_with_ask_runner(vault_root, Arc::new(FakeAskRunner::new(answer)))
    }
}
