use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaintenanceInbox {
    pub items: Vec<MaintenanceItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaintenanceItem {
    pub priority: String,
    pub kind: String,
    pub file: String,
    pub evidence: String,
    pub requires_confirmation: bool,
}
