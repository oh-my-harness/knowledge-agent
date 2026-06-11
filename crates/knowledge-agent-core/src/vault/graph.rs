use crate::vault::scanner::VaultScan;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkGraph {
    backlinks: BTreeMap<String, Vec<String>>,
}

impl LinkGraph {
    pub fn backlinks_to(&self, target: &str) -> Vec<&str> {
        self.backlinks
            .get(target)
            .map(|paths| paths.iter().map(String::as_str).collect())
            .unwrap_or_default()
    }
}

pub fn build_link_graph(scan: &VaultScan) -> LinkGraph {
    let mut backlinks: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for note in &scan.notes {
        for link in &note.links {
            backlinks
                .entry(link.target.clone())
                .or_default()
                .push(note.relative_path.clone());
        }
    }

    for paths in backlinks.values_mut() {
        paths.sort();
        paths.dedup();
    }

    LinkGraph { backlinks }
}
