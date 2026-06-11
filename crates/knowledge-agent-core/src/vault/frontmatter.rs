use anyhow::Result;
use gray_matter::{Matter, engine::YAML};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedNote {
    pub title: Option<String>,
    pub note_type: Option<String>,
    pub tags: Vec<String>,
    pub body: String,
}

#[derive(Debug, Default, Deserialize)]
struct RawFrontmatter {
    title: Option<String>,
    #[serde(rename = "type")]
    note_type: Option<String>,
    tags: Option<Vec<String>>,
}

pub fn parse_markdown_note(raw: &str) -> Result<ParsedNote> {
    let matter = Matter::<YAML>::new();
    let parsed = matter.parse(raw);
    let data = parsed
        .data
        .and_then(|value| value.deserialize::<RawFrontmatter>().ok())
        .unwrap_or_default();

    Ok(ParsedNote {
        title: data.title,
        note_type: data.note_type,
        tags: data.tags.unwrap_or_default(),
        body: parsed.content,
    })
}
