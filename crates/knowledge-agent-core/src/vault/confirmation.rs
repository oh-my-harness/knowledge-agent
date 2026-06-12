use std::{
    fs,
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfirmationQueue {
    pub items: Vec<ConfirmationItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfirmationItem {
    pub id: String,
    pub kind: ConfirmationKind,
    pub path: String,
    pub reason: Option<String>,
    pub original_content: String,
    pub proposed_content: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfirmationKind {
    ReplaceNote,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateReplaceNoteConfirmation {
    pub path: String,
    pub reason: Option<String>,
    pub proposed_content: String,
}

pub fn create_replace_note_confirmation(
    vault_root: impl AsRef<Path>,
    proposal: CreateReplaceNoteConfirmation,
) -> Result<ConfirmationItem> {
    let vault_root = vault_root.as_ref();
    let path = normalize_relative_path(&proposal.path);
    let full_path = resolve_existing_markdown_path(vault_root, &path)?;
    let original_content = fs::read_to_string(&full_path)
        .with_context(|| format!("failed to read existing note: {path}"))?;
    let created_at = unix_timestamp_nanos().to_string();
    let id = format!("{created_at}-{}", sanitize_id_segment(&path));

    let item = ConfirmationItem {
        id,
        kind: ConfirmationKind::ReplaceNote,
        path,
        reason: proposal.reason,
        original_content,
        proposed_content: proposal.proposed_content,
        created_at,
    };

    fs::create_dir_all(queue_dir(vault_root)).context("failed to create confirmation queue")?;
    let item_path = queue_item_path(vault_root, &item.id);
    let json = serde_json::to_string_pretty(&item).context("failed to serialize confirmation")?;
    fs::write(&item_path, json)
        .with_context(|| format!("failed to write confirmation item: {}", item_path.display()))?;

    Ok(item)
}

pub fn list_confirmations(vault_root: impl AsRef<Path>) -> Result<ConfirmationQueue> {
    let vault_root = vault_root.as_ref();
    let dir = queue_dir(vault_root);
    if !dir.exists() {
        return Ok(ConfirmationQueue { items: Vec::new() });
    }

    let mut items = Vec::new();
    for entry in fs::read_dir(&dir).context("failed to read confirmation queue")? {
        let entry = entry.context("failed to read confirmation queue entry")?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read confirmation: {}", path.display()))?;
        let item: ConfirmationItem = serde_json::from_str(&content)
            .with_context(|| format!("failed to parse confirmation: {}", path.display()))?;
        items.push(item);
    }

    items.sort_by(|left, right| left.created_at.cmp(&right.created_at));
    Ok(ConfirmationQueue { items })
}

pub fn apply_confirmation(vault_root: impl AsRef<Path>, id: &str) -> Result<ConfirmationItem> {
    let vault_root = vault_root.as_ref();
    let item = load_confirmation(vault_root, id)?;
    match item.kind {
        ConfirmationKind::ReplaceNote => {
            let full_path = resolve_existing_markdown_path(vault_root, &item.path)?;
            fs::write(&full_path, &item.proposed_content)
                .with_context(|| format!("failed to write note: {}", item.path))?;
        }
    }
    remove_confirmation(vault_root, id)?;
    Ok(item)
}

pub fn reject_confirmation(vault_root: impl AsRef<Path>, id: &str) -> Result<ConfirmationItem> {
    let vault_root = vault_root.as_ref();
    let item = load_confirmation(vault_root, id)?;
    remove_confirmation(vault_root, id)?;
    Ok(item)
}

fn load_confirmation(vault_root: &Path, id: &str) -> Result<ConfirmationItem> {
    validate_confirmation_id(id)?;
    let path = queue_item_path(vault_root, id);
    let content =
        fs::read_to_string(&path).with_context(|| format!("confirmation not found: {id}"))?;
    serde_json::from_str(&content).with_context(|| format!("failed to parse confirmation: {id}"))
}

fn remove_confirmation(vault_root: &Path, id: &str) -> Result<()> {
    validate_confirmation_id(id)?;
    let path = queue_item_path(vault_root, id);
    fs::remove_file(&path).with_context(|| format!("failed to remove confirmation: {id}"))
}

fn queue_dir(vault_root: &Path) -> PathBuf {
    vault_root.join(".knowledge-agent").join("confirmations")
}

fn queue_item_path(vault_root: &Path, id: &str) -> PathBuf {
    queue_dir(vault_root).join(format!("{id}.json"))
}

fn resolve_existing_markdown_path(vault_root: &Path, relative_path: &str) -> Result<PathBuf> {
    let normalized = normalize_relative_path(relative_path);
    if !normalized.ends_with(".md") {
        bail!("path must point to a Markdown .md file");
    }

    let path = Path::new(&normalized);
    if path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_)))
    {
        bail!("path must stay inside the vault");
    }

    let full_path = vault_root.join(path);
    if !full_path.exists() {
        bail!("note not found: {normalized}");
    }

    Ok(full_path)
}

fn normalize_relative_path(path: &str) -> String {
    path.trim()
        .replace('\\', "/")
        .trim_start_matches('/')
        .to_string()
}

fn validate_confirmation_id(id: &str) -> Result<()> {
    if id.trim().is_empty()
        || id.contains('/')
        || id.contains('\\')
        || id.contains("..")
        || id.contains(':')
    {
        bail!("invalid confirmation id");
    }
    Ok(())
}

fn sanitize_id_segment(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn unix_timestamp_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}
