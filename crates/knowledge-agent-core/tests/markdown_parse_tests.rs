use knowledge_agent_core::vault::{
    frontmatter::parse_markdown_note,
    wikilinks::{WikiLink, extract_wikilinks},
};

#[test]
fn parses_yaml_frontmatter_and_body() {
    let raw = r#"---
title: Agent Harness
type: concept
tags: [agent, runtime]
created: 2026-06-11
updated: 2026-06-11
---

# Agent Harness

Links to [[LLM Harness|runtime harness]].
"#;

    let note = parse_markdown_note(raw).expect("note parses");

    assert_eq!(note.title.as_deref(), Some("Agent Harness"));
    assert_eq!(note.note_type.as_deref(), Some("concept"));
    assert_eq!(note.tags, vec!["agent", "runtime"]);
    assert!(note.body.contains("# Agent Harness"));
}

#[test]
fn extracts_wikilinks_with_aliases() {
    let links = extract_wikilinks("See [[Agent Harness]] and [[LLM Harness|runtime harness]].");

    assert_eq!(
        links,
        vec![
            WikiLink {
                target: "Agent Harness".to_string(),
                alias: None
            },
            WikiLink {
                target: "LLM Harness".to_string(),
                alias: Some("runtime harness".to_string())
            },
        ]
    );
}
