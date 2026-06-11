use knowledge_agent_core::vault::{
    graph::build_link_graph, index_docs::parse_index_document, scanner::scan_vault,
};
use std::path::Path;

#[test]
fn parses_index_links_and_summaries() {
    let raw = include_str!("fixtures/basic-vault/docs/concepts/_index.md");

    let index = parse_index_document("docs/concepts/_index.md", raw).expect("index parses");

    assert_eq!(index.path, "docs/concepts/_index.md");
    assert_eq!(index.entries.len(), 1);
    assert_eq!(index.entries[0].title, "Agent Harness");
    assert_eq!(index.entries[0].links[0].target, "agent-harness");
    assert_eq!(
        index.entries[0].summary.as_deref(),
        Some("Agent runtime, tools, and sessions.")
    );
}

#[test]
fn builds_backlinks_from_scan() {
    let vault = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/basic-vault");
    let scan = scan_vault(&vault).expect("vault scans");

    let graph = build_link_graph(&scan);

    let backlinks = graph.backlinks_to("LLM Harness");
    assert_eq!(backlinks, vec!["docs/concepts/agent-harness.md"]);
}
