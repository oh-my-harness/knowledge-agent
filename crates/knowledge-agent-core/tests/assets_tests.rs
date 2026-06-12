use knowledge_agent_core::vault::assets::list_pdf_assets;

#[test]
fn lists_pdf_assets_inside_vault() {
    let vault = tempfile::tempdir().expect("tempdir");
    let papers = vault.path().join("assets").join("papers");
    let local_state = vault.path().join(".knowledge-agent");
    std::fs::create_dir_all(&papers).expect("papers dir");
    std::fs::create_dir_all(&local_state).expect("local state dir");
    std::fs::write(papers.join("one.pdf"), b"%PDF-1.4\n").expect("pdf");
    std::fs::write(local_state.join("ignored.pdf"), b"%PDF-1.4\n").expect("ignored pdf");
    std::fs::write(vault.path().join("note.md"), "# Note\n").expect("note");

    let pdfs = list_pdf_assets(vault.path()).expect("pdf assets");

    assert_eq!(pdfs.len(), 1);
    assert_eq!(pdfs[0].path, "assets/papers/one.pdf");
    assert_eq!(pdfs[0].bytes, 9);
}
