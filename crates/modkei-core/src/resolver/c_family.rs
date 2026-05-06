use std::{collections::HashSet, path::Path};

use super::common::resolve_candidate;
use crate::{ImportEdge, RawImportKind};

pub fn resolve(root: &Path, import: &ImportEdge, rel_set: &HashSet<String>) -> Option<String> {
    if import.kind != RawImportKind::Include {
        return None;
    }
    let candidates = vec![
        import.from.parent()?.join(&import.target),
        root.join(&import.target),
        root.join("include").join(&import.target),
        root.join("src").join(&import.target),
    ];
    candidates.into_iter().find_map(|candidate| {
        resolve_candidate(
            root,
            &candidate,
            rel_set,
            &["h", "hpp", "c", "cpp", "cxx", "cc"],
        )
    })
}
