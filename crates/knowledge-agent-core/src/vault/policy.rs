#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VaultWriteOperation {
    AddIndexEntry {
        index_path: String,
        target_path: String,
    },
    UpdateFrontmatterField {
        path: String,
        field: String,
    },
    MarkNonSemanticMetadata {
        path: String,
        field: String,
    },
    ModifyBodyMeaning {
        path: String,
    },
    DeleteNote {
        path: String,
    },
    MoveNote {
        from: String,
        to: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteDecision {
    AllowAutomatic,
    RequireConfirmation,
}

#[derive(Debug, Default)]
pub struct VaultWritePolicy;

impl VaultWritePolicy {
    pub fn decide(&self, operation: &VaultWriteOperation) -> WriteDecision {
        match operation {
            VaultWriteOperation::AddIndexEntry { .. }
            | VaultWriteOperation::MarkNonSemanticMetadata { .. } => WriteDecision::AllowAutomatic,
            VaultWriteOperation::UpdateFrontmatterField { field, .. } => {
                if is_low_risk_frontmatter_field(field) {
                    WriteDecision::AllowAutomatic
                } else {
                    WriteDecision::RequireConfirmation
                }
            }
            VaultWriteOperation::ModifyBodyMeaning { .. }
            | VaultWriteOperation::DeleteNote { .. }
            | VaultWriteOperation::MoveNote { .. } => WriteDecision::RequireConfirmation,
        }
    }
}

fn is_low_risk_frontmatter_field(field: &str) -> bool {
    matches!(field, "created" | "updated")
}
