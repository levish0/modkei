use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use super::common::{nearest_config_file, resolve_candidate};
use crate::ImportEdge;

pub fn resolve(root: &Path, import: &ImportEdge, rel_set: &HashSet<String>) -> Option<String> {
    if import.target.starts_with('.') {
        let candidate = import.from.parent()?.join(&import.target);
        return resolve_candidate(root, &candidate, rel_set, &["ts", "tsx", "js", "jsx"]);
    }

    for candidate in config_path_candidates(root, import) {
        if let Some(resolved) =
            resolve_candidate(root, &candidate, rel_set, &["ts", "tsx", "js", "jsx"])
        {
            return Some(resolved);
        }
    }
    None
}

fn config_path_candidates(root: &Path, import: &ImportEdge) -> Vec<PathBuf> {
    let Some(config_path) =
        nearest_config_file(&import.from, root, &["tsconfig.json", "jsconfig.json"])
    else {
        return Vec::new();
    };
    let Some(config_dir) = config_path.parent() else {
        return Vec::new();
    };
    let Some(config) = read_config(&config_path) else {
        return Vec::new();
    };

    let base_url = config
        .pointer("/compilerOptions/baseUrl")
        .and_then(|value| value.as_str())
        .unwrap_or(".");
    let base = config_dir.join(base_url);
    let mut candidates = Vec::new();

    if let Some(paths) = config
        .pointer("/compilerOptions/paths")
        .and_then(|value| value.as_object())
    {
        for (pattern, replacements) in paths {
            let Some(capture) = match_path_pattern(pattern, &import.target) else {
                continue;
            };
            let Some(replacements) = replacements.as_array() else {
                continue;
            };
            for replacement in replacements.iter().filter_map(|value| value.as_str()) {
                candidates.push(base.join(apply_replacement(replacement, &capture)));
            }
        }
    }

    candidates.push(base.join(&import.target));
    candidates
}

fn read_config(path: &Path) -> Option<serde_json::Value> {
    let source = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&source).ok()
}

fn match_path_pattern(pattern: &str, raw: &str) -> Option<String> {
    if pattern == raw {
        return Some(String::new());
    }
    let (prefix, suffix) = pattern.split_once('*')?;
    raw.strip_prefix(prefix)
        .and_then(|rest| rest.strip_suffix(suffix))
        .map(ToOwned::to_owned)
}

fn apply_replacement(replacement: &str, capture: &str) -> String {
    if replacement.contains('*') {
        replacement.replace('*', capture)
    } else {
        replacement.to_string()
    }
}
