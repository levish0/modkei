use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::Duration,
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
    let output_dir = Path::new(UI_DIR).join(".svelte-kit").join("output");
    if output_dir.exists() {
        fs::remove_dir_all(&output_dir)
            .with_context(|| format!("failed to remove {}", output_dir.display()))?;
    }

    let pnpm = if cfg!(windows) { "pnpm.cmd" } else { "pnpm" };
    let status = Command::new(pnpm)
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
    copy_static_assets(Path::new(STATIC_REPORT_DIR), output_dir)?;
    fs::copy(&index_path, output_path)
        .with_context(|| format!("failed to write {}", output_path.display()))?;
    Ok(())
}

pub fn serve_and_open(path: &Path) -> Result<()> {
    let output_dir = path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
        .canonicalize()
        .with_context(|| format!("failed to resolve report directory for {}", path.display()))?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("index.html");

    let pnpm = if cfg!(windows) { "pnpm.cmd" } else { "pnpm" };
    Command::new(pnpm)
        .args([
            "exec",
            "vite",
            "preview",
            "--host",
            "127.0.0.1",
            "--port",
            "4173",
            "--strictPort",
            "--outDir",
        ])
        .arg(&output_dir)
        .args(["--logLevel", "error"])
        .current_dir(UI_DIR)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| {
            format!(
                "failed to start preview server for {}",
                output_dir.display()
            )
        })?;

    thread::sleep(Duration::from_millis(700));
    let url = if file_name == "index.html" {
        "http://127.0.0.1:4173/".to_string()
    } else {
        format!("http://127.0.0.1:4173/{}", url_path_segment(file_name))
    };
    open::that(&url).with_context(|| format!("failed to open {url}"))?;
    Ok(())
}

fn copy_static_assets(source: &Path, destination: &Path) -> Result<()> {
    for entry in
        fs::read_dir(source).with_context(|| format!("failed to read {}", source.display()))?
    {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            fs::create_dir_all(&destination_path)
                .with_context(|| format!("failed to create {}", destination_path.display()))?;
            copy_static_assets(&source_path, &destination_path)?;
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

fn url_path_segment(segment: &str) -> String {
    segment
        .replace('%', "%25")
        .replace(' ', "%20")
        .replace('#', "%23")
        .replace('?', "%3F")
}
