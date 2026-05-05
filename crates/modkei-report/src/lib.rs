use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use modkei_core::GraphData;

const STATIC_REPORT_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/static-report");

pub fn generate(graph: &GraphData, output_path: &Path) -> Result<()> {
    let static_dir = Path::new(STATIC_REPORT_DIR);
    let index_path = static_dir.join("index.html");
    if !index_path.is_file() {
        bail!(
            "missing report UI build at {}; run `pnpm build` in crates/modkei-report/ui",
            static_dir.display()
        );
    }

    let output_dir = output_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(output_dir)
        .with_context(|| format!("failed to create {}", output_dir.display()))?;

    copy_static_assets(static_dir, output_dir)?;
    fs::copy(&index_path, output_path)
        .with_context(|| format!("failed to write {}", output_path.display()))?;

    let graph_path = output_dir.join("graph.json");
    let graph_json = serde_json::to_vec(graph)?;
    fs::write(&graph_path, graph_json)
        .with_context(|| format!("failed to write {}", graph_path.display()))?;
    Ok(())
}

pub fn open_in_browser(path: &Path) -> Result<()> {
    open::that(path).with_context(|| format!("failed to open {}", path.display()))?;
    Ok(())
}

fn copy_static_assets(source: &Path, destination: &Path) -> Result<()> {
    for entry in
        fs::read_dir(source).with_context(|| format!("failed to read {}", source.display()))?
    {
        let entry = entry?;
        let source_path = entry.path();
        let file_name = entry.file_name();
        if file_name == "index.html" || file_name == "graph.json" {
            continue;
        }
        let destination_path: PathBuf = destination.join(file_name);
        if source_path.is_dir() {
            fs::create_dir_all(&destination_path)
                .with_context(|| format!("failed to create {}", destination_path.display()))?;
            copy_dir_recursive(&source_path, &destination_path)?;
        } else {
            fs::copy(&source_path, &destination_path).with_context(|| {
                format!(
                    "failed to copy {} to {}",
                    source_path.display(),
                    destination_path.display()
                )
            })?;
        }
    }
    Ok(())
}

fn copy_dir_recursive(source: &Path, destination: &Path) -> Result<()> {
    for entry in
        fs::read_dir(source).with_context(|| format!("failed to read {}", source.display()))?
    {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            fs::create_dir_all(&destination_path)
                .with_context(|| format!("failed to create {}", destination_path.display()))?;
            copy_dir_recursive(&source_path, &destination_path)?;
        } else {
            fs::copy(&source_path, &destination_path).with_context(|| {
                format!(
                    "failed to copy {} to {}",
                    source_path.display(),
                    destination_path.display()
                )
            })?;
        }
    }
    Ok(())
}
