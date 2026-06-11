use crate::{
    maintenance::inbox::{MaintenanceInbox, MaintenanceItem},
    vault::scanner::scan_vault,
};
use anyhow::Result;
use std::{collections::BTreeSet, path::Path};

pub fn run_maintenance_scan(vault_root: &Path) -> Result<MaintenanceInbox> {
    let scan = scan_vault(vault_root)?;
    let known_titles = scan
        .notes
        .iter()
        .filter_map(|note| note.title.clone())
        .collect::<BTreeSet<_>>();
    let known_stems = scan
        .notes
        .iter()
        .filter_map(|note| {
            note.relative_path
                .rsplit('/')
                .next()
                .and_then(|name| name.strip_suffix(".md"))
                .map(str::to_string)
        })
        .collect::<BTreeSet<_>>();

    let mut items = Vec::new();

    for note in &scan.notes {
        for link in &note.links {
            if !known_titles.contains(&link.target) && !known_stems.contains(&link.target) {
                items.push(MaintenanceItem {
                    priority: "P0".to_string(),
                    kind: "broken_wikilink".to_string(),
                    file: note.relative_path.clone(),
                    evidence: format!("Missing target [[{}]]", link.target),
                    requires_confirmation: false,
                });
            }
        }
    }

    Ok(MaintenanceInbox { items })
}
