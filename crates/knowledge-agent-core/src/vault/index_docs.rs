use crate::vault::wikilinks::{WikiLink, extract_wikilinks};
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexDocument {
    pub path: String,
    pub entries: Vec<IndexEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexEntry {
    pub title: String,
    pub links: Vec<WikiLink>,
    pub summary: Option<String>,
}

pub fn parse_index_document(path: &str, raw: &str) -> Result<IndexDocument> {
    let mut entries = Vec::new();
    let mut current: Option<IndexEntry> = None;

    for line in raw.lines() {
        if let Some(title) = line.strip_prefix("## ") {
            if let Some(entry) = current.take() {
                entries.push(entry);
            }
            current = Some(IndexEntry {
                title: title.trim().to_string(),
                links: Vec::new(),
                summary: None,
            });
            continue;
        }

        if let Some(entry) = current.as_mut() {
            entry.links.extend(extract_wikilinks(line));
            if let Some(summary) = line.trim().strip_prefix("- summary:") {
                entry.summary = Some(summary.trim().to_string());
            }
        }
    }

    if let Some(entry) = current {
        entries.push(entry);
    }

    Ok(IndexDocument {
        path: path.to_string(),
        entries,
    })
}
