mod rust;

use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::ResolvedEdge;

pub fn semantic_edges(root: &Path, files: &[PathBuf]) -> Result<Vec<ResolvedEdge>> {
    rust::semantic_edges(root, files)
}
