use std::{path::PathBuf, sync::Arc};

#[derive(Debug, Clone)]
pub struct AppState {
    pub vault_root: Arc<PathBuf>,
}

impl AppState {
    pub fn new(vault_root: PathBuf) -> Self {
        Self {
            vault_root: Arc::new(vault_root),
        }
    }
}
