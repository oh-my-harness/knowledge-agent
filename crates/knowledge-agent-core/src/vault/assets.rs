use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use walkdir::WalkDir;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PdfAsset {
    pub path: String,
    pub bytes: u64,
}

pub fn list_pdf_assets(vault_root: &Path) -> Result<Vec<PdfAsset>> {
    let mut pdfs = Vec::new();

    for entry in WalkDir::new(vault_root).into_iter().filter_entry(|entry| {
        entry.file_name() != ".git" && entry.file_name() != ".knowledge-agent"
    }) {
        let entry = entry?;
        if !entry.file_type().is_file()
            || entry.path().extension().and_then(|value| value.to_str()) != Some("pdf")
        {
            continue;
        }

        let metadata = entry.metadata()?;
        pdfs.push(PdfAsset {
            path: entry
                .path()
                .strip_prefix(vault_root)?
                .to_string_lossy()
                .replace('\\', "/"),
            bytes: metadata.len(),
        });
    }

    pdfs.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(pdfs)
}
