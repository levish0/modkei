use std::{fs, path::Path};

use anyhow::{Context, Result};
use modkei_core::GraphData;

const TEMPLATE: &str = include_str!("../templates/report.html");

pub fn generate(graph: &GraphData, output_path: &Path) -> Result<()> {
    let graph_json = serde_json::to_string(graph)?;
    let html = TEMPLATE.replace("__GRAPH_JSON__", &graph_json);
    fs::write(output_path, html)
        .with_context(|| format!("failed to write {}", output_path.display()))?;
    Ok(())
}

pub fn open_in_browser(path: &Path) -> Result<()> {
    open::that(path).with_context(|| format!("failed to open {}", path.display()))?;
    Ok(())
}
