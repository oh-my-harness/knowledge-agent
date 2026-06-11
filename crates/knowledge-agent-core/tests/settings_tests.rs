use knowledge_agent_core::settings::{VaultSettings, load_vault_settings};
use std::path::Path;

#[test]
fn loads_shared_vault_settings() {
    let vault = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/basic-vault");

    let settings = load_vault_settings(&vault).expect("settings should load");

    assert_eq!(settings.docs_dir, "docs");
    assert_eq!(settings.research_dir, "docs/research");
    assert_eq!(settings.concepts_dir, "docs/concepts");
    assert_eq!(
        settings.required_frontmatter,
        vec!["title", "type", "created", "updated"]
    );
}

#[test]
fn defaults_when_shared_settings_missing() {
    let temp = tempfile::tempdir().expect("tempdir");

    let settings = load_vault_settings(temp.path()).expect("defaults should load");

    assert_eq!(settings, VaultSettings::default());
}
