use std::{collections::HashSet, path::Path};

use super::resolve_candidate;

pub fn resolve(root: &Path, from: &Path, raw: &str, rel_set: &HashSet<String>) -> Option<String> {
    let parent = from.parent()?;
    if let Some(module) = raw.strip_prefix("mod:") {
        return resolve_candidate(root, &parent.join(module.trim()), rel_set, &["rs"]);
    }

    let module = raw.strip_prefix("use:")?;
    let src_root = nearest_src_dir(from).unwrap_or(root);
    let candidates = rust_path_candidates(src_root, parent, module);
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
