use knowledge_agent_core::{
    maintenance::checks::run_maintenance_scan,
    vault::policy::{VaultWriteOperation, VaultWritePolicy, WriteDecision},
};
use std::path::Path;

#[test]
fn reports_broken_wikilinks() {
    let vault = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/basic-vault");

    let inbox = run_maintenance_scan(&vault).expect("scan succeeds");

    assert!(inbox.items.iter().any(|item| {
        item.kind == "broken_wikilink"
            && item.file == "docs/concepts/agent-harness.md"
            && item.evidence.contains("LLM Harness")
    }));
}

#[test]
fn write_policy_allows_only_low_risk_automatic_writes() {
    let policy = VaultWritePolicy;

    assert_eq!(
        policy.decide(&VaultWriteOperation::AddIndexEntry {
            index_path: "docs/concepts/_index.md".to_string(),
            target_path: "docs/concepts/agent-harness.md".to_string(),
        }),
        WriteDecision::AllowAutomatic
    );

    assert_eq!(
        policy.decide(&VaultWriteOperation::ModifyBodyMeaning {
            path: "docs/concepts/agent-harness.md".to_string(),
        }),
        WriteDecision::RequireConfirmation
    );
}
