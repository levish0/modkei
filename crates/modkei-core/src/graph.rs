use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use serde::Serialize;

use crate::{Language, ResolvedEdge, walker::FileResult};

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

pub fn build_graph(
    _root: &Path,
    files: &[FileResult],
    semantic_edges: &[ResolvedEdge],
) -> GraphData {
    let rel_by_abs: HashMap<PathBuf, String> = files
        .iter()
        .map(|file| (file.path.clone(), file.rel_path.clone()))
        .collect();
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
    for edge in semantic_edges {
        let Some(source) = rel_by_abs.get(&edge.from) else {
            continue;
        };
        let Some(target) = rel_by_abs.get(&edge.to) else {
            continue;
        };
        if source == target || !seen.insert((source.clone(), target.clone())) {
            continue;
        }
        edges.push(Edge {
            source: source.clone(),
            target: target.clone(),
            label: edge.label.clone(),
        });
    }

    GraphData { nodes, edges }
}
