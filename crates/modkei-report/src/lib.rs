use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, bail};
use modkei_core::GraphData;

const UI_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/ui");
const STATIC_REPORT_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/static-report");
const GENERATED_GRAPH_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/ui/src/lib/generated/graph-data.json"
);

pub fn generate(graph: &GraphData, output_path: &Path) -> Result<()> {
    write_generated_graph(graph)?;
    build_ui()?;
    copy_report(output_path)?;
    Ok(())
}

pub fn open_in_browser(path: &Path) -> Result<()> {
    open::that(path).with_context(|| format!("failed to open {}", path.display()))?;
    Ok(())
}

fn write_generated_graph(graph: &GraphData) -> Result<()> {
    let graph_path = Path::new(GENERATED_GRAPH_PATH);
    if let Some(parent) = graph_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let json = serde_json::to_vec_pretty(graph)?;
    fs::write(graph_path, json)
        .with_context(|| format!("failed to write {}", graph_path.display()))?;
    Ok(())
}

fn build_ui() -> Result<()> {
    let status = Command::new("pnpm")
        .arg("build")
        .current_dir(UI_DIR)
        .status()
        .with_context(|| format!("failed to run `pnpm build` in {UI_DIR}"))?;

    if !status.success() {
        bail!("`pnpm build` failed in {UI_DIR}");
    }
    Ok(())
}

fn copy_report(output_path: &Path) -> Result<()> {
    let index_path = PathBuf::from(STATIC_REPORT_DIR).join("index.html");
    if !index_path.is_file() {
        bail!(
            "missing built report at {}; `pnpm build` did not produce index.html",
            index_path.display()
        );
    }

    let output_dir = output_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(output_dir)
        .with_context(|| format!("failed to create {}", output_dir.display()))?;
    fs::copy(&index_path, output_path)
        .with_context(|| format!("failed to write {}", output_path.display()))?;
    Ok(())
}
