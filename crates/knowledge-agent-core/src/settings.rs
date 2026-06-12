use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

const LOCAL_STATE_GITIGNORE_ENTRY: &str = ".knowledge-agent/";

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

    let raw =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let settings = toml::from_str::<VaultSettings>(&raw)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(settings)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitVaultReport {
    pub vault_settings_path: PathBuf,
    pub local_state_dir: PathBuf,
    pub gitignore_path: PathBuf,
    pub created_vault_settings: bool,
    pub created_local_state_dir: bool,
    pub updated_gitignore: bool,
}

pub fn init_vault(vault_root: &Path) -> Result<InitVaultReport> {
    fs::create_dir_all(vault_root)
        .with_context(|| format!("failed to create {}", vault_root.display()))?;

    let vault_settings_path = vault_root.join(".knowledge-agent.toml");
    let created_vault_settings = if vault_settings_path.exists() {
        false
    } else {
        let raw = toml::to_string_pretty(&VaultSettings::default())
            .context("failed to serialize default vault settings")?;
        fs::write(&vault_settings_path, raw)
            .with_context(|| format!("failed to write {}", vault_settings_path.display()))?;
        true
    };

    let local_state_dir = vault_root.join(".knowledge-agent");
    let created_local_state_dir = if local_state_dir.exists() {
        false
    } else {
        fs::create_dir_all(&local_state_dir)
            .with_context(|| format!("failed to create {}", local_state_dir.display()))?;
        true
    };

    let gitignore_path = vault_root.join(".gitignore");
    let updated_gitignore = ensure_gitignore_entry(&gitignore_path, LOCAL_STATE_GITIGNORE_ENTRY)?;

    Ok(InitVaultReport {
        vault_settings_path,
        local_state_dir,
        gitignore_path,
        created_vault_settings,
        created_local_state_dir,
        updated_gitignore,
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalSettings {
    pub llm: LocalLlmSettings,
    pub web_search: LocalWebSearchSettings,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalLlmSettings {
    pub provider: String,
    pub deepseek_api_key: Option<String>,
    pub deepseek_model: String,
}

impl Default for LocalLlmSettings {
    fn default() -> Self {
        Self {
            provider: "deepseek".to_string(),
            deepseek_api_key: None,
            deepseek_model: "deepseek-v4-flash".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalWebSearchSettings {
    pub enabled: bool,
    pub provider: String,
}

impl Default for LocalWebSearchSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: "duckduckgo".to_string(),
        }
    }
}

pub fn load_local_settings(vault_root: &Path) -> Result<LocalSettings> {
    let path = local_settings_path(vault_root);
    if !path.exists() {
        return Ok(LocalSettings::default());
    }

    let raw =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let settings = toml::from_str::<LocalSettings>(&raw)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(settings)
}

pub fn save_local_settings(vault_root: &Path, settings: &LocalSettings) -> Result<()> {
    let path = local_settings_path(vault_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let raw = toml::to_string_pretty(settings).context("failed to serialize local settings")?;
    fs::write(&path, raw).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

fn local_settings_path(vault_root: &Path) -> std::path::PathBuf {
    vault_root.join(".knowledge-agent").join("local.toml")
}

fn ensure_gitignore_entry(path: &Path, entry: &str) -> Result<bool> {
    let existing = if path.exists() {
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?
    } else {
        String::new()
    };

    if existing.lines().any(|line| line.trim() == entry) {
        return Ok(false);
    }

    let mut next = existing;
    if !next.is_empty() && !next.ends_with('\n') {
        next.push('\n');
    }
    next.push_str(entry);
    next.push('\n');

    fs::write(path, next).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(true)
}
