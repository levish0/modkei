mod module;
mod rust;

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use serde::Serialize;

use crate::{
    Language,
    walker::{FileResult, ImportEdge, normalize_path},
};

#[derive(Debug, Clone, Serialize)]
pub struct Node {
    pub id: String,
    pub label: String,
    pub language: Language,
    pub lines: u64,
    pub code: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Edge {
    pub source: String,
    pub target: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphData {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

pub fn build_graph(root: &Path, files: &[FileResult], imports: &[ImportEdge]) -> GraphData {
    let rel_by_abs: HashMap<PathBuf, String> = files
        .iter()
        .map(|file| (file.path.clone(), file.rel_path.clone()))
        .collect();
    let rel_set: HashSet<String> = files.iter().map(|file| file.rel_path.clone()).collect();

    let nodes = files
        .iter()
        .map(|file| Node {
            id: file.rel_path.clone(),
            label: Path::new(&file.rel_path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(&file.rel_path)
                .to_string(),
            language: file.language,
            lines: file.lines,
            code: file.code,
        })
        .collect();

    let mut seen = HashSet::new();
    let mut edges = Vec::new();
    for import in imports {
        let Some(source) = rel_by_abs.get(&import.from) else {
            continue;
        };
        let Some(target) =
            resolve_import(root, &import.from, import.language, &import.to, &rel_set)
        else {
            continue;
        };
        if source == &target || !seen.insert((source.clone(), target.clone())) {
            continue;
        }
        edges.push(Edge {
            source: source.clone(),
            target,
            label: display_import_label(&import.to),
        });
    }

    GraphData { nodes, edges }
}

fn display_import_label(raw: &str) -> String {
    raw.strip_prefix("module:")
        .or_else(|| raw.strip_prefix("mod:"))
        .or_else(|| raw.strip_prefix("use:"))
        .or_else(|| raw.strip_prefix("include:"))
        .unwrap_or(raw)
        .to_string()
}

fn resolve_import(
    root: &Path,
    from: &Path,
    language: Language,
    raw: &str,
    rel_set: &HashSet<String>,
) -> Option<String> {
    match language {
        Language::TypeScript | Language::JavaScript => {
            module::resolve_relative(from, raw, root, rel_set, &["ts", "tsx", "js", "jsx"])
        }
        Language::Python => module::resolve_python(from, raw, root, rel_set),
        Language::Rust => rust::resolve(root, from, raw, rel_set),
        Language::Go => module::resolve_go(raw, rel_set),
        Language::C | Language::Cpp => {
            let path = raw.strip_prefix("include:")?;
            // Usually, includes already have the extension (.h, .hpp), so we just join it.
            // If it starts with a common directory (e.g. include/ or src/), it's usually relative to workspace root or current dir.
            let mut candidates = vec![
                from.parent()?.join(path),
                root.join(path),
                root.join("include").join(path),
                root.join("src").join(path),
            ];
            candidates.into_iter().find_map(|c| {
                resolve_candidate(root, &c, rel_set, &["h", "hpp", "c", "cpp", "cxx", "cc"])
            })
        }
        Language::Unknown => None,
    }
}

pub(super) fn resolve_candidate(
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
