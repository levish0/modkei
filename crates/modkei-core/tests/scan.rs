use std::{fs, path::PathBuf};

use crossbeam_channel::unbounded;
use modkei_core::{IgnoreOptions, ScanOptions};

#[test]
fn rust_modules_create_file_edges() {
    let root = unique_temp_dir();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("src/lib.rs"),
        "mod parser;\npub use parser::parse;\n",
    )
    .unwrap();
    fs::write(root.join("src/parser.rs"), "pub fn parse() {}\n").unwrap();

    let (tx, rx) = unbounded();
    let output = modkei_core::scan(
        &root,
        ScanOptions {
            ignore: IgnoreOptions::default(),
        },
        tx,
    )
    .unwrap();
    drop(rx);

    assert_eq!(output.graph.nodes.len(), 2);
    assert!(
        output
            .graph
            .edges
            .iter()
            .any(|edge| edge.source == "src/lib.rs" && edge.target == "src/parser.rs"),
        "expected lib.rs to be connected to parser.rs, got {:?}",
        output.graph.edges
    );

    let _ = fs::remove_dir_all(root);
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
