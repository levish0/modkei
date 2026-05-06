use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use crate::walker::normalize_path;

pub fn resolve_candidate(
    root: &Path,
    candidate: &Path,
    rel_set: &HashSet<String>,
    exts: &[&str],
) -> Option<String> {
    let rel = candidate.strip_prefix(root).unwrap_or(candidate);
    let base = normalize_path(rel);
    let mut options = vec![base.clone()];
    for ext in exts {
        options.push(format!("{base}.{ext}"));
        options.push(format!("{base}/mod.{ext}"));
        options.push(format!("{base}/index.{ext}"));
        options.push(format!("{base}/__init__.{ext}"));
    }
    options.into_iter().find(|path| rel_set.contains(path))
}

pub fn nearest_config_dir(from: &Path, root: &Path, names: &[&str]) -> Option<PathBuf> {
    let mut dir = from.parent()?;
    loop {
        if names.iter().any(|name| dir.join(name).is_file()) {
            return Some(dir.to_path_buf());
        }
        if dir == root {
            return None;
        }
        dir = dir.parent()?;
    }
}

pub fn nearest_config_file(from: &Path, root: &Path, names: &[&str]) -> Option<PathBuf> {
    let dir = nearest_config_dir(from, root, names)?;
    names
        .iter()
        .map(|name| dir.join(name))
        .find(|path| path.is_file())
}
