use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VaultSettings {
    pub docs_dir: String,
    pub research_dir: String,
    pub concepts_dir: String,
    pub index_file_name: String,
    pub required_frontmatter: Vec<String>,
}

impl Default for VaultSettings {
    fn default() -> Self {
        Self {
            docs_dir: "docs".to_string(),
            research_dir: "docs/research".to_string(),
            concepts_dir: "docs/concepts".to_string(),
            index_file_name: "_index.md".to_string(),
            required_frontmatter: vec![
                "title".to_string(),
                "type".to_string(),
                "created".to_string(),
                "updated".to_string(),
            ],
        }
    }
}

pub fn load_vault_settings(vault_root: &Path) -> Result<VaultSettings> {
    let path = vault_root.join(".knowledge-agent.toml");
    if !path.exists() {
        return Ok(VaultSettings::default());
    }

    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let settings = toml::from_str::<VaultSettings>(&raw)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(settings)
}
