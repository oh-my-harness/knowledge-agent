use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WikiLink {
    pub target: String,
    pub alias: Option<String>,
}

pub fn extract_wikilinks(markdown: &str) -> Vec<WikiLink> {
    let mut links = Vec::new();
    let mut rest = markdown;

    while let Some(start) = rest.find("[[") {
        let after_start = &rest[start + 2..];
        let Some(end) = after_start.find("]]") else {
            break;
        };
        let inner = &after_start[..end];
        let mut parts = inner.splitn(2, '|');
        let target = parts.next().unwrap_or("").trim();
        let alias = parts.next().map(str::trim).filter(|s| !s.is_empty());

        if !target.is_empty() {
            links.push(WikiLink {
                target: target.to_string(),
                alias: alias.map(str::to_string),
            });
        }

        rest = &after_start[end + 2..];
    }

    links
}
