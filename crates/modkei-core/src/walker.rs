use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};

use anyhow::{Context, Result};
use crossbeam_channel::Sender;
use ignore::WalkBuilder;
use rayon::prelude::*;
use serde::Serialize;

use crate::{Language, build_graph, parser, stats};

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

#[derive(Debug, Clone, Serialize)]
pub struct ImportEdge {
    pub from: PathBuf,
    pub to: String,
    pub language: Language,
}

#[derive(Debug, Clone)]
pub struct ScanOutput {
    pub files: Vec<FileResult>,
    pub imports: Vec<ImportEdge>,
    pub graph: crate::GraphData,
}

pub fn scan(root: &Path, options: ScanOptions, tx: Sender<FileResult>) -> Result<ScanOutput> {
    let files = collect_files(root, options)?;
    let imports = Mutex::new(Vec::new());
    let results = Mutex::new(Vec::new());

    files.par_iter().for_each(|path| {
        if let Ok((file, edges)) = analyze_file(root, path) {
            let _ = tx.send(file.clone());
            results.lock().expect("results mutex poisoned").push(file);
            imports
                .lock()
                .expect("imports mutex poisoned")
                .extend(edges);
        }
    });
    drop(tx);

    let files = results.into_inner().expect("results mutex poisoned");
    let imports = imports.into_inner().expect("imports mutex poisoned");
    let graph = build_graph(root, &files, &imports);

    Ok(ScanOutput {
        files,
        imports,
        graph,
    })
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
        .git_exclude(!ignore.no_ignore_vcs);
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

fn analyze_file(root: &Path, path: &Path) -> Result<(FileResult, Vec<ImportEdge>)> {
    let language = Language::from_path(path);
    let source = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let (lines, code, comments, blanks) = stats::count_lines(&source, language);
    let rel_path = normalize_path(path.strip_prefix(root).unwrap_or(path));
    let imports = parser::extract_imports(&source, language)
        .into_iter()
        .map(|to| ImportEdge {
            from: path.to_path_buf(),
            to,
            language,
        })
        .collect();
    Ok((
        FileResult {
            path: path.to_path_buf(),
            rel_path,
            language,
            lines,
            code,
            comments,
            blanks,
        },
        imports,
    ))
}

pub(crate) fn normalize_path(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
