use std::{
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

use knowledge_agent_core::settings::{LocalSettings, load_local_settings};
use knowledge_agent_harness::{AskRunner, DeepSeekAskRunner, FakeAskRunner, UnavailableAskRunner};

#[derive(Clone)]
pub struct AppState {
    pub vault_root: Arc<PathBuf>,
    ask_runner: Arc<RwLock<Arc<dyn AskRunner>>>,
}

impl AppState {
    pub fn new(vault_root: PathBuf) -> Self {
        let ask_runner = Self::build_ask_runner(&vault_root);
        Self {
            vault_root: Arc::new(vault_root),
            ask_runner: Arc::new(RwLock::new(ask_runner)),
        }
    }

    pub fn ask_runner(&self) -> Arc<dyn AskRunner> {
        self.ask_runner
            .read()
            .expect("ask runner lock poisoned")
            .clone()
    }

    pub fn reload_ask_runner(&self) {
        let runner = Self::build_ask_runner(self.vault_root.as_ref());
        *self.ask_runner.write().expect("ask runner lock poisoned") = runner;
    }

    fn build_ask_runner(vault_root: &Path) -> Arc<dyn AskRunner> {
        let sessions_root = vault_root.join(".knowledge-agent").join("sessions");
        let local_settings = load_local_settings(vault_root).unwrap_or_default();
        Self::build_ask_runner_with_settings(vault_root, local_settings, Some(sessions_root))
    }

    fn build_ask_runner_with_settings(
        vault_root: &Path,
        local_settings: LocalSettings,
        sessions_root: Option<PathBuf>,
    ) -> Arc<dyn AskRunner> {
        let web_search_enabled =
            local_settings.web_search.enabled && local_settings.web_search.provider == "duckduckgo";
        match DeepSeekAskRunner::from_env_with_options(
            |name| {
                let local_value = match name {
                    "DEEPSEEK_API_KEY" => local_settings.llm.deepseek_api_key.clone(),
                    "DEEPSEEK_MODEL" => Some(local_settings.llm.deepseek_model.clone()),
                    _ => None,
                }
                .filter(|value| !value.trim().is_empty());
                local_value.or_else(|| std::env::var(name).ok())
            },
            sessions_root,
            "default".to_string(),
            Some(vault_root.to_path_buf()),
        ) {
            Ok(runner) => Arc::new(runner.with_web_search(web_search_enabled)),
            Err(err) => Arc::new(UnavailableAskRunner::new(err)),
        }
    }

    pub fn new_with_ask_runner(vault_root: PathBuf, ask_runner: Arc<dyn AskRunner>) -> Self {
        Self {
            vault_root: Arc::new(vault_root),
            ask_runner: Arc::new(RwLock::new(ask_runner)),
        }
    }

    pub fn new_with_fake_ask_runner(vault_root: PathBuf, answer: impl Into<String>) -> Self {
        Self::new_with_ask_runner(vault_root, Arc::new(FakeAskRunner::new(answer)))
    }
}
