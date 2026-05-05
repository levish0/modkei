use std::{
    fs,
    net::TcpListener,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
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

/// Returns the self-contained directory for the built report assets.
/// Named `<stem>-files/` next to the output HTML so the project root stays clean.
fn report_serve_dir(output_path: &Path) -> PathBuf {
    let stem = output_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("modkei-report");
    let parent = output_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    parent.join(format!("{stem}-files"))
}

fn copy_report(output_path: &Path) -> Result<()> {
    let index_path = PathBuf::from(STATIC_REPORT_DIR).join("index.html");
    if !index_path.is_file() {
        bail!(
            "missing built report at {}; `pnpm build` did not produce index.html",
            index_path.display()
        );
    }

    // Copy everything (index.html + _app/ + robots.txt …) into the self-contained
    // serve dir.  Because index.html and _app/ are siblings there, no path
    // rewriting is needed — the relative imports just work.
    let serve_dir = report_serve_dir(output_path);
    fs::create_dir_all(&serve_dir)
        .with_context(|| format!("failed to create {}", serve_dir.display()))?;
    copy_static_assets(Path::new(STATIC_REPORT_DIR), &serve_dir)?;

    // Write a thin redirect at the user-visible output path so double-clicking
    // it in a file manager still opens the report.
    let stem = output_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("modkei-report");
    let redirect = format!(
        "<!doctype html><html><head>\
         <meta http-equiv=\"refresh\" content=\"0;url=./{stem}-files/\"/>\
         </head><body></body></html>"
    );
    if let Some(p) = output_path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(p).with_context(|| format!("failed to create {}", p.display()))?;
    }
    fs::write(output_path, redirect)
        .with_context(|| format!("failed to write {}", output_path.display()))?;
    Ok(())
}

pub fn serve_and_open(path: &Path) -> Result<String> {
    // Serve the self-contained `<stem>-files/` directory directly so that
    // index.html and _app/ share the same root — no URL path confusion.
    let serve_dir = report_serve_dir(path)
        .canonicalize()
        .with_context(|| format!("failed to resolve serve directory for {}", path.display()))?;

    let port = available_port()?;
    let npx = if cfg!(windows) { "npx.cmd" } else { "npx" };
    let mut child = Command::new(npx)
        .args([
            "-y",
            "serve",
            "--listen",
            &port.to_string(),
            "--no-clipboard",
        ])
        .arg(&serve_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("failed to start preview server for {}", serve_dir.display()))?;

    // Give npx time to download `serve` on first run and bind the port.
    thread::sleep(Duration::from_millis(1400));
    let url = format!("http://127.0.0.1:{port}/");
    open::that(&url).with_context(|| format!("failed to open {url}"))?;

    // Block until Ctrl+C so the child server process stays alive.
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || r.store(false, Ordering::SeqCst))
        .context("failed to set Ctrl+C handler")?;
    eprintln!("Serving report at {url}  (press Ctrl+C to stop)");
    while running.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(200));
    }
    let _ = child.kill();
    Ok(url)
}

fn copy_static_assets(source: &Path, destination: &Path) -> Result<()> {
    for entry in
        fs::read_dir(source).with_context(|| format!("failed to read {}", source.display()))?
    {
        let entry = entry?;
        let src = entry.path();
        let dst = destination.join(entry.file_name());
        if src.is_dir() {
            fs::create_dir_all(&dst)
                .with_context(|| format!("failed to create {}", dst.display()))?;
            copy_static_assets(&src, &dst)?;
        } else {
            fs::copy(&src, &dst).with_context(|| {
                format!("failed to copy {} to {}", src.display(), dst.display())
            })?;
        }
    }
    Ok(())
}

fn available_port() -> Result<u16> {
    let listener =
        TcpListener::bind(("127.0.0.1", 0)).context("failed to reserve preview server port")?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}
