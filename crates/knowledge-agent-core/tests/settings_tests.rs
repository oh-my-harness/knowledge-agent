use knowledge_agent_core::settings::{
    LocalSettings, VaultSettings, load_local_settings, load_vault_settings, save_local_settings,
};
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

#[test]
fn saves_and_loads_local_settings() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut settings = LocalSettings::default();
    settings.llm.deepseek_api_key = Some("secret".to_string());
    settings.llm.deepseek_model = "deepseek-chat".to_string();
    settings.web_search.enabled = true;
    settings.web_search.provider = "duckduckgo".to_string();

    save_local_settings(temp.path(), &settings).expect("local settings should save");
    let loaded = load_local_settings(temp.path()).expect("local settings should load");

    assert_eq!(loaded, settings);
}

#[test]
fn defaults_when_local_settings_missing() {
    let temp = tempfile::tempdir().expect("tempdir");

    let settings = load_local_settings(temp.path()).expect("defaults should load");

    assert_eq!(settings, LocalSettings::default());
}
