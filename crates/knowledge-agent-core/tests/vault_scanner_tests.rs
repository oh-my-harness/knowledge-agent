use knowledge_agent_core::vault::scanner::scan_vault;
use std::path::Path;

#[test]
fn scans_markdown_notes_relative_to_vault_root() {
    let vault = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/basic-vault");

    let scan = scan_vault(&vault).expect("vault scans");

    let note = scan
        .notes
        .iter()
        .find(|note| note.relative_path == "docs/concepts/agent-harness.md")
        .expect("concept note exists");

    assert_eq!(note.title.as_deref(), Some("Agent Harness"));
    assert_eq!(note.tags, vec!["agent", "runtime"]);
    assert_eq!(note.links[0].target, "LLM Harness");
}
