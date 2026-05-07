use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use crossbeam_channel::Sender;
use ignore::WalkBuilder;
use rayon::prelude::*;
use serde::Serialize;

use crate::{Language, backend, build_graph, stats};

#[derive(Debug, Clone, Copy, Default)]
pub struct IgnoreOptions {
    pub no_ignore: bool,
    pub no_ignore_parent: bool,
    pub no_ignore_dot: bool,
    pub no_ignore_vcs: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ScanOptions {
    pub ignore: IgnoreOptions,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileResult {
    pub path: PathBuf,
    pub rel_path: String,
    pub language: Language,
    pub lines: u64,
    pub code: u64,
    pub comments: u64,
    pub blanks: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressStage {
    ScanningFiles,
    ResolvingDependencies,
    BuildingGraph,
}

impl ProgressStage {
    pub fn message(self) -> &'static str {
        match self {
            Self::ScanningFiles => "scanning files",
            Self::ResolvingDependencies => "resolving Rust dependencies",
            Self::BuildingGraph => "building graph",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressEvent {
    Stage(ProgressStage),
    FileScanned,
}

#[derive(Debug, Clone)]
pub struct ScanOutput {
    pub files: Vec<FileResult>,
    pub graph: crate::GraphData,
}

pub fn scan(root: &Path, options: ScanOptions, tx: Sender<ProgressEvent>) -> Result<ScanOutput> {
    let files = collect_files(root, options)?;
    let results = std::sync::Mutex::new(Vec::new());

    let _ = tx.send(ProgressEvent::Stage(ProgressStage::ScanningFiles));
    files.par_iter().for_each(|path| {
        if let Ok(file) = analyze_file(root, path) {
            let _ = tx.send(ProgressEvent::FileScanned);
            results.lock().expect("results mutex poisoned").push(file);
        }
    });

    let files = results.into_inner().expect("results mutex poisoned");
    let _ = tx.send(ProgressEvent::Stage(ProgressStage::ResolvingDependencies));
    let semantic_edges = backend::semantic_edges(
        root,
        &files
            .iter()
            .map(|file| file.path.clone())
            .collect::<Vec<_>>(),
    )?;
    let _ = tx.send(ProgressEvent::Stage(ProgressStage::BuildingGraph));
    let graph = build_graph(root, &files, &semantic_edges);
    drop(tx);

    Ok(ScanOutput { files, graph })
}

fn collect_files(root: &Path, options: ScanOptions) -> Result<Vec<PathBuf>> {
    let ignore = options.ignore;
    let mut builder = WalkBuilder::new(root);
    builder
        .standard_filters(true)
        .parents(!ignore.no_ignore_parent)
        .ignore(!ignore.no_ignore_dot)
        .git_ignore(!ignore.no_ignore_vcs)
        .git_global(!ignore.no_ignore_vcs)
        .git_exclude(!ignore.no_ignore_vcs)
        .require_git(false);
    if ignore.no_ignore {
        builder
            .parents(false)
            .ignore(false)
            .git_ignore(false)
            .git_global(false)
            .git_exclude(false);
    }

    let mut files = Vec::new();
    for entry in builder.build() {
        let entry = entry.with_context(|| format!("failed walking {}", root.display()))?;
        if entry.path().is_file() {
            let language = Language::from_path(entry.path());
            if language.is_supported() {
                files.push(entry.into_path());
            }
        }
    }
    Ok(files)
}

fn analyze_file(root: &Path, path: &Path) -> Result<FileResult> {
    let language = Language::from_path(path);
    let source = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let (lines, code, comments, blanks) = stats::count_lines(&source, language);
    let rel_path = normalize_path(path.strip_prefix(root).unwrap_or(path));
    Ok(FileResult {
        path: path.to_path_buf(),
        rel_path,
        language,
        lines,
        code,
        comments,
        blanks,
    })
}

pub(crate) fn normalize_path(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
