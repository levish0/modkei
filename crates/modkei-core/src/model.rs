use serde::Serialize;

use crate::Language;

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct ResolvedEdge {
    pub from: std::path::PathBuf,
    pub to: std::path::PathBuf,
    pub label: String,
    pub language: Language,
}
