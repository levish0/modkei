use std::{fs, path::PathBuf};

use crossbeam_channel::unbounded;
use modkei_core::{IgnoreOptions, ScanOptions};

#[test]
fn rust_modules_create_file_edges_without_manifest() {
    let root = unique_temp_dir();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("src/lib.rs"),
        "mod parser;\npub use parser::parse;\n",
    )
    .unwrap();
    fs::write(root.join("src/parser.rs"), "pub fn parse() {}\n").unwrap();

    let output = scan_dir(&root);

    assert_eq!(output.graph.nodes.len(), 2);
    assert_edge(&output.graph.edges, "src/lib.rs", "src/parser.rs");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rust_module_directories_resolve_to_mod_rs_without_manifest() {
    let root = unique_temp_dir();
    fs::create_dir_all(root.join("src/parser")).unwrap();
    fs::write(root.join("src/lib.rs"), "mod parser;\n").unwrap();
    fs::write(root.join("src/parser/mod.rs"), "pub fn parse() {}\n").unwrap();

    let output = scan_dir(&root);

    assert_edge(&output.graph.edges, "src/lib.rs", "src/parser/mod.rs");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rust_workspace_members_resolve_with_rust_analyzer() {
    let root = unique_temp_dir();
    fs::create_dir_all(root.join("crates/app/src/parser")).unwrap();
    fs::create_dir_all(root.join("crates/lib/src/parser")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        r#"[workspace]
members = ["crates/app", "crates/lib"]
"#,
    )
    .unwrap();
    fs::write(
        root.join("crates/app/Cargo.toml"),
        r#"[package]
name = "app"
version = "0.1.0"
edition = "2024"
"#,
    )
    .unwrap();
    fs::write(
        root.join("crates/lib/Cargo.toml"),
        r#"[package]
name = "lib"
version = "0.1.0"
edition = "2024"
"#,
    )
    .unwrap();
    fs::write(
        root.join("crates/app/src/lib.rs"),
        "mod parser;\nuse crate::parser::parse;\n",
    )
    .unwrap();
    fs::write(
        root.join("crates/app/src/parser/mod.rs"),
        "pub fn parse() {}\n",
    )
    .unwrap();
    fs::write(
        root.join("crates/lib/src/parser/mod.rs"),
        "pub fn parse() {}\n",
    )
    .unwrap();
    fs::write(root.join("crates/lib/src/lib.rs"), "pub mod parser;\n").unwrap();

    let output = scan_dir(&root);

    assert_edge(
        &output.graph.edges,
        "crates/app/src/lib.rs",
        "crates/app/src/parser/mod.rs",
    );
    assert_no_edge(
        &output.graph.edges,
        "crates/app/src/lib.rs",
        "crates/lib/src/parser/mod.rs",
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn gitignore_is_respected_by_default() {
    let root = unique_temp_dir();
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join(".gitignore"), "ignored.rs\n").unwrap();
    fs::write(root.join("kept.rs"), "pub fn kept() {}\n").unwrap();
    fs::write(root.join("ignored.rs"), "pub fn ignored() {}\n").unwrap();

    let output = scan_dir(&root);

    assert!(output.graph.nodes.iter().any(|node| node.id == "kept.rs"));
    assert!(
        !output
            .graph
            .nodes
            .iter()
            .any(|node| node.id == "ignored.rs")
    );

    let _ = fs::remove_dir_all(root);
}

fn scan_dir(root: &std::path::Path) -> modkei_core::ScanOutput {
    let (tx, rx) = unbounded();
    let output = modkei_core::scan(
        root,
        ScanOptions {
            ignore: IgnoreOptions::default(),
        },
        tx,
    )
    .unwrap();
    drop(rx);
    output
}

fn assert_edge(edges: &[modkei_core::Edge], source: &str, target: &str) {
    assert!(
        edges
            .iter()
            .any(|edge| edge.source == source && edge.target == target),
        "expected edge {source} -> {target}, got {edges:?}"
    );
}

fn assert_no_edge(edges: &[modkei_core::Edge], source: &str, target: &str) {
    assert!(
        !edges
            .iter()
            .any(|edge| edge.source == source && edge.target == target),
        "did not expect edge {source} -> {target}, got {edges:?}"
    );
}

fn unique_temp_dir() -> PathBuf {
    std::env::temp_dir().join(format!(
        "modkei-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}
