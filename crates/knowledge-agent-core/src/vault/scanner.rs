use crate::vault::{
    frontmatter::parse_markdown_note,
    wikilinks::{WikiLink, extract_wikilinks},
};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VaultScan {
    pub root: PathBuf,
    pub notes: Vec<ScannedNote>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScannedNote {
    pub relative_path: String,
    pub title: Option<String>,
    pub note_type: Option<String>,
    pub tags: Vec<String>,
    pub links: Vec<WikiLink>,
}

pub fn scan_vault(vault_root: &Path) -> Result<VaultScan> {
    let mut notes = Vec::new();

    for entry in WalkDir::new(vault_root).into_iter().filter_entry(|entry| {
        entry.file_name() != ".git" && entry.file_name() != ".knowledge-agent"
    }) {
        let entry = entry?;
        if !entry.file_type().is_file()
            || entry.path().extension().and_then(|s| s.to_str()) != Some("md")
        {
            continue;
        }

        let raw = std::fs::read_to_string(entry.path())
            .with_context(|| format!("failed to read {}", entry.path().display()))?;
        let parsed = parse_markdown_note(&raw)?;
        let relative_path = entry
            .path()
            .strip_prefix(vault_root)?
            .to_string_lossy()
            .replace('\\', "/");

        notes.push(ScannedNote {
            relative_path,
            title: parsed.title,
            note_type: parsed.note_type,
            tags: parsed.tags,
            links: extract_wikilinks(&raw),
        });
    }

    notes.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

    Ok(VaultScan {
        root: vault_root.to_path_buf(),
        notes,
    })
}
