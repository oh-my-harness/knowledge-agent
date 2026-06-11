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
            | VaultWriteOperation::UpdateFrontmatterField { .. }
            | VaultWriteOperation::MarkNonSemanticMetadata { .. } => {
                WriteDecision::AllowAutomatic
            }
            VaultWriteOperation::ModifyBodyMeaning { .. }
            | VaultWriteOperation::DeleteNote { .. }
            | VaultWriteOperation::MoveNote { .. } => WriteDecision::RequireConfirmation,
        }
    }
}
