use std::{collections::HashSet, path::Path};

use super::common::resolve_candidate;
use crate::{ImportEdge, RawImportKind};

pub fn resolve(root: &Path, import: &ImportEdge, rel_set: &HashSet<String>) -> Option<String> {
    let parent = import.from.parent()?;
    if import.kind == RawImportKind::Module {
        return resolve_candidate(root, &parent.join(import.target.trim()), rel_set, &["rs"]);
    }

    if import.kind != RawImportKind::Symbol {
        return None;
    }

    let src_root = nearest_src_dir(&import.from).unwrap_or(root);
    let candidates = rust_path_candidates(src_root, parent, &import.target);
    candidates
        .iter()
        .find_map(|candidate| resolve_candidate(root, candidate, rel_set, &["rs"]))
}

fn nearest_src_dir(path: &Path) -> Option<&Path> {
    path.ancestors()
        .find(|ancestor| ancestor.file_name().and_then(|name| name.to_str()) == Some("src"))
}

fn rust_path_candidates(src_root: &Path, parent: &Path, module: &str) -> Vec<std::path::PathBuf> {
    let module = module.trim();
    let (base, path) = if let Some(rest) = module.strip_prefix("crate::") {
        (src_root, rest)
    } else if let Some(rest) = module.strip_prefix("self::") {
        (parent, rest)
    } else if let Some(rest) = module.strip_prefix("super::") {
        (parent.parent().unwrap_or(parent), rest)
    } else {
        (src_root, module)
    };

    let parts = path
        .split("::")
        .filter(|part| !part.is_empty() && *part != "*")
        .collect::<Vec<_>>();
    let mut candidates = Vec::new();
    for len in (1..=parts.len()).rev() {
        candidates.push(base.join(parts[..len].join("/")));
    }
    if !matches!(module.split("::").next(), Some("crate" | "self" | "super")) {
        for len in (1..=parts.len()).rev() {
            candidates.push(parent.join(parts[..len].join("/")));
        }
    }
    candidates
}
