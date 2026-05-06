use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use ra_ap_ide::{
    Analysis, AnalysisHost, FilePosition, GotoDefinitionConfig, NavigationTarget, RaFixtureConfig,
};
use ra_ap_load_cargo::{LoadCargoConfig, ProcMacroServerChoice, load_workspace_at};
use ra_ap_project_model::CargoConfig;
use ra_ap_syntax::{
    AstNode, Edition, SourceFile, SyntaxToken,
    ast::{self, HasName},
};
use ra_ap_vfs::VfsPath;

use crate::{Language, ResolvedEdge, walker::normalize_path};

pub fn semantic_edges(root: &Path, files: &[PathBuf]) -> Result<Vec<ResolvedEdge>> {
    let canonical_root = canonicalize_lossy(root);
    let file_map = files
        .iter()
        .map(|path| (canonicalize_lossy(path), path.clone()))
        .collect::<HashMap<_, _>>();
    let file_set = files
        .iter()
        .map(|path| canonicalize_lossy(path))
        .collect::<HashSet<_>>();
    let mut grouped = HashMap::<PathBuf, Vec<PathBuf>>::new();
    let mut standalone = Vec::new();

    for file in files {
        if let Some(manifest) = nearest_manifest(file) {
            grouped.entry(manifest).or_default().push(file.clone());
        } else {
            standalone.push(file.clone());
        }
    }

    let mut edges = Vec::new();
    for (manifest, group_files) in grouped {
        edges.extend(workspace_edges(
            &canonical_root,
            &manifest,
            &group_files,
            &file_set,
            &file_map,
        )?);
    }
    if !standalone.is_empty() {
        edges.extend(fallback_edges(root, &standalone, &file_map)?);
    }

    Ok(dedup_edges(edges))
}

fn workspace_edges(
    root: &Path,
    manifest: &Path,
    files: &[PathBuf],
    file_set: &HashSet<PathBuf>,
    file_map: &HashMap<PathBuf, PathBuf>,
) -> Result<Vec<ResolvedEdge>> {
    let cargo_config = CargoConfig::default();
    let load_config = LoadCargoConfig {
        load_out_dirs_from_check: false,
        with_proc_macro_server: ProcMacroServerChoice::None,
        prefill_caches: false,
        num_worker_threads: 1,
        proc_macro_processes: 1,
    };
    let (db, vfs, _) = load_workspace_at(manifest, &cargo_config, &load_config, &|_| {})
        .with_context(|| format!("failed to load Rust workspace at {}", manifest.display()))?;
    let host = AnalysisHost::with_database(db);
    let analysis = host.analysis();
    let goto_config = GotoDefinitionConfig {
        ra_fixture: RaFixtureConfig::default(),
    };

    let mut edges = Vec::new();
    for file in files {
        let Some(file_id) = file_id_for_path(&vfs, file) else {
            continue;
        };
        let syntax = analysis.parse(file_id).map_err(|_| {
            anyhow::anyhow!("rust-analyzer cancelled while parsing {}", file.display())
        })?;
        collect_workspace_use_edges(
            root,
            file,
            file_id,
            &syntax,
            &analysis,
            &goto_config,
            &vfs,
            file_set,
            file_map,
            &mut edges,
        );
        collect_workspace_mod_edges(
            root,
            file,
            file_id,
            &syntax,
            &analysis,
            &goto_config,
            &vfs,
            file_set,
            file_map,
            &mut edges,
        );
    }

    Ok(edges)
}

#[allow(clippy::too_many_arguments)]
fn collect_workspace_use_edges(
    root: &Path,
    from: &Path,
    file_id: ra_ap_ide::FileId,
    syntax: &SourceFile,
    analysis: &Analysis,
    goto_config: &GotoDefinitionConfig<'_>,
    vfs: &ra_ap_vfs::Vfs,
    file_set: &HashSet<PathBuf>,
    file_map: &HashMap<PathBuf, PathBuf>,
    edges: &mut Vec<ResolvedEdge>,
) {
    for use_item in syntax.syntax().descendants().filter_map(ast::Use::cast) {
        let Some(use_tree) = use_item.use_tree() else {
            continue;
        };
        collect_workspace_use_tree_edges(
            root,
            from,
            file_id,
            &use_tree,
            analysis,
            goto_config,
            vfs,
            file_set,
            file_map,
            edges,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn collect_workspace_use_tree_edges(
    root: &Path,
    from: &Path,
    file_id: ra_ap_ide::FileId,
    use_tree: &ast::UseTree,
    analysis: &Analysis,
    goto_config: &GotoDefinitionConfig<'_>,
    vfs: &ra_ap_vfs::Vfs,
    file_set: &HashSet<PathBuf>,
    file_map: &HashMap<PathBuf, PathBuf>,
    edges: &mut Vec<ResolvedEdge>,
) {
    if let Some(list) = use_tree.use_tree_list() {
        for child in list.use_trees() {
            collect_workspace_use_tree_edges(
                root,
                from,
                file_id,
                &child,
                analysis,
                goto_config,
                vfs,
                file_set,
                file_map,
                edges,
            );
        }
        return;
    }

    let Some(token) = use_tree_resolution_token(use_tree) else {
        return;
    };
    let label = use_tree.syntax().text().to_string();
    for target in resolve_targets(analysis, goto_config, vfs, file_id, &token) {
        let target = canonicalize_lossy(&target);
        if !file_set.contains(&target) {
            continue;
        }
        let target = file_map.get(&target).cloned().unwrap_or(target);
        push_edge(from, &target, label.clone(), edges);
    }
}

#[allow(clippy::too_many_arguments)]
fn collect_workspace_mod_edges(
    _root: &Path,
    from: &Path,
    file_id: ra_ap_ide::FileId,
    syntax: &SourceFile,
    analysis: &Analysis,
    goto_config: &GotoDefinitionConfig<'_>,
    vfs: &ra_ap_vfs::Vfs,
    file_set: &HashSet<PathBuf>,
    file_map: &HashMap<PathBuf, PathBuf>,
    edges: &mut Vec<ResolvedEdge>,
) {
    for module in syntax.syntax().descendants().filter_map(ast::Module::cast) {
        if module.semicolon_token().is_none() {
            continue;
        }
        let Some(name) = module.name() else {
            continue;
        };
        let Some(token) = name.ident_token().or_else(|| name.self_token()) else {
            continue;
        };
        let label = format!("mod {}", name.syntax().text());
        for target in resolve_targets(analysis, goto_config, vfs, file_id, &token) {
            let target = canonicalize_lossy(&target);
            if !file_set.contains(&target) {
                continue;
            }
            let target = file_map.get(&target).cloned().unwrap_or(target);
            push_edge(from, &target, label.clone(), edges);
        }
    }
}

fn fallback_edges(
    root: &Path,
    files: &[PathBuf],
    file_map: &HashMap<PathBuf, PathBuf>,
) -> Result<Vec<ResolvedEdge>> {
    let rel_set = files
        .iter()
        .map(|path| normalize_path(path.strip_prefix(root).unwrap_or(path)))
        .collect::<HashSet<_>>();
    let mut edges = Vec::new();

    for file in files {
        let source = std::fs::read_to_string(file)
            .with_context(|| format!("failed to read {}", file.display()))?;
        let syntax = SourceFile::parse(&source, Edition::CURRENT).tree();
        for module in syntax.syntax().descendants().filter_map(ast::Module::cast) {
            if module.semicolon_token().is_none() {
                continue;
            }
            let Some(name) = module.name() else {
                continue;
            };
            let target_name = name.syntax().text().to_string();
            if let Some(target) = resolve_rust_candidate(
                root,
                &file.parent().unwrap_or(root).join(target_name.as_str()),
                &rel_set,
            ) {
                let target_path = root.join(&target);
                let target_path = file_map
                    .get(&canonicalize_lossy(&target_path))
                    .cloned()
                    .unwrap_or(target_path);
                push_edge(
                    file,
                    &target_path,
                    format!("mod {}", target_name),
                    &mut edges,
                );
            }
        }

        for use_item in syntax.syntax().descendants().filter_map(ast::Use::cast) {
            let Some(use_tree) = use_item.use_tree() else {
                continue;
            };
            for path in collect_fallback_use_paths(&use_tree, "") {
                let candidates = rust_path_candidates(root, file.parent().unwrap_or(root), &path);
                if let Some(target) = candidates
                    .iter()
                    .find_map(|candidate| resolve_rust_candidate(root, candidate, &rel_set))
                {
                    let target_path = root.join(&target);
                    let target_path = file_map
                        .get(&canonicalize_lossy(&target_path))
                        .cloned()
                        .unwrap_or(target_path);
                    push_edge(file, &target_path, path.clone(), &mut edges);
                }
            }
        }
    }

    Ok(edges)
}

fn collect_fallback_use_paths(use_tree: &ast::UseTree, prefix: &str) -> Vec<String> {
    let mut current = prefix.trim_end_matches("::").to_string();
    if let Some(path) = use_tree.path() {
        let text = path.syntax().text().to_string();
        current = join_path(&current, text.trim());
    }

    if let Some(list) = use_tree.use_tree_list() {
        return list
            .use_trees()
            .flat_map(|child| collect_fallback_use_paths(&child, &current))
            .collect();
    }

    if use_tree.star_token().is_some() {
        return (!current.is_empty())
            .then_some(current)
            .into_iter()
            .collect();
    }

    let current = current.trim_end_matches("::self").to_string();
    (!current.is_empty())
        .then_some(current)
        .into_iter()
        .collect()
}

fn use_tree_resolution_token(use_tree: &ast::UseTree) -> Option<SyntaxToken> {
    let path = use_tree.path()?;
    let segment = path.segment()?;
    let name_ref = segment.name_ref()?;
    name_ref
        .ident_token()
        .or_else(|| name_ref.self_token())
        .or_else(|| name_ref.super_token())
        .or_else(|| name_ref.crate_token())
        .or_else(|| name_ref.Self_token())
}

fn resolve_targets(
    analysis: &Analysis,
    goto_config: &GotoDefinitionConfig<'_>,
    vfs: &ra_ap_vfs::Vfs,
    file_id: ra_ap_ide::FileId,
    token: &SyntaxToken,
) -> Vec<PathBuf> {
    analysis
        .goto_definition(
            FilePosition {
                file_id,
                offset: token.text_range().start(),
            },
            goto_config,
        )
        .ok()
        .flatten()
        .into_iter()
        .flat_map(|info| info.info.into_iter())
        .filter_map(|nav| navigation_target_path(vfs, &nav))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect()
}

fn navigation_target_path(vfs: &ra_ap_vfs::Vfs, nav: &NavigationTarget) -> Option<PathBuf> {
    vfs.file_path(nav.file_id)
        .as_path()
        .map(|path| PathBuf::from(path.to_string()))
}

fn file_id_for_path(vfs: &ra_ap_vfs::Vfs, path: &Path) -> Option<ra_ap_ide::FileId> {
    let path = canonicalize_lossy(path);
    let vfs_path = VfsPath::new_real_path(path.to_string_lossy().into_owned());
    vfs.file_id(&vfs_path).map(|(file_id, _)| file_id)
}

fn nearest_manifest(path: &Path) -> Option<PathBuf> {
    path.ancestors()
        .map(|ancestor| ancestor.join("Cargo.toml"))
        .find(|candidate| candidate.is_file())
        .map(|candidate| canonicalize_lossy(&candidate))
}

fn canonicalize_lossy(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn push_edge(from: &Path, to: &Path, label: String, edges: &mut Vec<ResolvedEdge>) {
    if from == to {
        return;
    }
    edges.push(ResolvedEdge {
        from: from.to_path_buf(),
        to: to.to_path_buf(),
        label,
        language: Language::Rust,
    });
}

fn rust_path_candidates(root: &Path, parent: &Path, module: &str) -> Vec<PathBuf> {
    let src_root = nearest_src_dir(parent).unwrap_or(root);
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

fn nearest_src_dir(path: &Path) -> Option<&Path> {
    path.ancestors()
        .find(|ancestor| ancestor.file_name().and_then(|name| name.to_str()) == Some("src"))
}

fn join_path(prefix: &str, suffix: &str) -> String {
    let prefix = prefix.trim().trim_end_matches("::");
    let suffix = suffix.trim().trim_start_matches("::");
    match (prefix.is_empty(), suffix.is_empty()) {
        (true, true) => String::new(),
        (true, false) => suffix.to_string(),
        (false, true) => prefix.to_string(),
        (false, false) => format!("{prefix}::{suffix}"),
    }
}

fn resolve_rust_candidate(
    root: &Path,
    candidate: &Path,
    rel_set: &HashSet<String>,
) -> Option<String> {
    let rel = candidate.strip_prefix(root).unwrap_or(candidate);
    let base = normalize_path(rel);
    [base.clone(), format!("{base}.rs"), format!("{base}/mod.rs")]
        .into_iter()
        .find(|option| rel_set.contains(option))
}

fn dedup_edges(edges: Vec<ResolvedEdge>) -> Vec<ResolvedEdge> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();
    for edge in edges {
        let key = (edge.from.clone(), edge.to.clone(), edge.label.clone());
        if seen.insert(key) {
            deduped.push(edge);
        }
    }
    deduped
}
