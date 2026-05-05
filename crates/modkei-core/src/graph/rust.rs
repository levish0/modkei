use std::{collections::HashSet, path::Path};

use super::resolve_candidate;

pub fn resolve(root: &Path, from: &Path, raw: &str, rel_set: &HashSet<String>) -> Option<String> {
    let parent = from.parent()?;
    if let Some(module) = raw.strip_prefix("mod:") {
        return resolve_candidate(root, &parent.join(module.trim()), rel_set, &["rs"]);
    }

    let module = raw.strip_prefix("use:")?;
    let src_root = nearest_src_dir(from).unwrap_or(root);
    resolve_candidate(root, &src_root.join(module), rel_set, &["rs"])
        .or_else(|| resolve_candidate(root, &parent.join(module), rel_set, &["rs"]))
}

fn nearest_src_dir(path: &Path) -> Option<&Path> {
    path.ancestors()
        .find(|ancestor| ancestor.file_name().and_then(|name| name.to_str()) == Some("src"))
}
