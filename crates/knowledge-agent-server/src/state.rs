use std::{path::PathBuf, sync::Arc};

use knowledge_agent_harness::{AskRunner, DeepSeekAskRunner, FakeAskRunner, UnavailableAskRunner};

#[derive(Clone)]
pub struct AppState {
    pub vault_root: Arc<PathBuf>,
    pub ask_runner: Arc<dyn AskRunner>,
}

impl AppState {
    pub fn new(vault_root: PathBuf) -> Self {
        let ask_runner: Arc<dyn AskRunner> =
            match DeepSeekAskRunner::from_env_with_vault(&vault_root) {
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
