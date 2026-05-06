use std::{collections::HashSet, path::Path};

use super::common::{nearest_config_file, resolve_candidate};
use crate::ImportEdge;

pub fn resolve(root: &Path, import: &ImportEdge, rel_set: &HashSet<String>) -> Option<String> {
    let module = nearest_go_module(root, &import.from);
    let suffix = module
        .as_ref()
        .and_then(|module| import.target.strip_prefix(&module.path))
        .map(|path| path.trim_start_matches('/').replace('\\', "/"))
        .filter(|path| !path.is_empty());

    if let (Some(module), Some(suffix)) = (&module, &suffix) {
        if let Some(target) = resolve_candidate(root, &module.dir.join(suffix), rel_set, &["go"]) {
            return Some(target);
        }
    }

    let suffix = suffix.unwrap_or_else(|| import.target.replace('\\', "/"));

    if let Some(target) = resolve_candidate(root, Path::new(&suffix), rel_set, &["go"]) {
        return Some(target);
    }

    rel_set
        .iter()
        .find(|path| {
            path.contains(&format!("{suffix}/")) || path.ends_with(&format!("{suffix}.go"))
        })
        .cloned()
}

struct GoModule {
    dir: std::path::PathBuf,
    path: String,
}

fn nearest_go_module(root: &Path, from: &Path) -> Option<GoModule> {
    let go_mod = nearest_config_file(from, root, &["go.mod"])?;
    let dir = go_mod.parent()?.to_path_buf();
    let source = std::fs::read_to_string(go_mod).ok()?;
    let path = source
        .lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix("module ").map(str::trim))
        .filter(|module| !module.is_empty())
        .map(ToOwned::to_owned)?;
    Some(GoModule { dir, path })
}
