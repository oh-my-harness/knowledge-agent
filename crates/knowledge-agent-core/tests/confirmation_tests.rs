use knowledge_agent_core::vault::confirmation::{
    ConfirmationKind, CreateReplaceNoteConfirmation, apply_confirmation,
    create_replace_note_confirmation, list_confirmations, reject_confirmation,
};

#[test]
fn creates_lists_and_applies_replace_note_confirmation() {
    let vault = tempfile::tempdir().expect("tempdir");
    let note_path = vault.path().join("docs").join("note.md");
    std::fs::create_dir_all(note_path.parent().unwrap()).unwrap();
    std::fs::write(&note_path, "# Old\n").unwrap();

    let item = create_replace_note_confirmation(
        vault.path(),
        CreateReplaceNoteConfirmation {
            path: "docs/note.md".to_string(),
            reason: Some("update summary".to_string()),
            proposed_content: "# New\n".to_string(),
        },
    )
    .unwrap();

    assert_eq!(item.kind, ConfirmationKind::ReplaceNote);
    assert_eq!(item.original_content, "# Old\n");
    assert_eq!(item.proposed_content, "# New\n");

    let queue = list_confirmations(vault.path()).unwrap();
    assert_eq!(queue.items.len(), 1);
    assert_eq!(queue.items[0].id, item.id);

    let applied = apply_confirmation(vault.path(), &item.id).unwrap();
    assert_eq!(applied.path, "docs/note.md");
    assert_eq!(std::fs::read_to_string(&note_path).unwrap(), "# New\n");
    assert!(list_confirmations(vault.path()).unwrap().items.is_empty());
}

#[test]
fn rejects_replace_note_confirmation_without_writing() {
    let vault = tempfile::tempdir().expect("tempdir");
    let note_path = vault.path().join("note.md");
    std::fs::write(&note_path, "# Keep\n").unwrap();

    let item = create_replace_note_confirmation(
        vault.path(),
        CreateReplaceNoteConfirmation {
            path: "note.md".to_string(),
            reason: None,
            proposed_content: "# Replace\n".to_string(),
        },
    )
    .unwrap();

    let rejected = reject_confirmation(vault.path(), &item.id).unwrap();
    assert_eq!(rejected.id, item.id);
    assert_eq!(std::fs::read_to_string(&note_path).unwrap(), "# Keep\n");
    assert!(list_confirmations(vault.path()).unwrap().items.is_empty());
}
