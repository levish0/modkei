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

#[test]
fn rust_module_directories_resolve_to_mod_rs() {
    let root = unique_temp_dir();
    fs::create_dir_all(root.join("src/parser")).unwrap();
    fs::write(root.join("src/lib.rs"), "mod parser;\n").unwrap();
    fs::write(root.join("src/parser/mod.rs"), "pub fn parse() {}\n").unwrap();

    let output = scan_dir(&root);

    assert_edge(&output.graph.edges, "src/lib.rs", "src/parser/mod.rs");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rust_workspace_members_resolve_crate_paths_inside_nearest_package() {
    let root = unique_temp_dir();
    fs::create_dir_all(root.join("crates/app/src/parser")).unwrap();
    fs::create_dir_all(root.join("crates/lib/src/parser")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        r#"
        [workspace]
        members = ["crates/app", "crates/lib"]
        "#,
    )
    .unwrap();
    fs::write(
        root.join("crates/app/Cargo.toml"),
        "[package]\nname = \"app\"\n",
    )
    .unwrap();
    fs::write(
        root.join("crates/lib/Cargo.toml"),
        "[package]\nname = \"lib\"\n",
    )
    .unwrap();
    fs::write(
        root.join("crates/app/src/lib.rs"),
        "use crate::parser::parse;\n",
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
fn typescript_relative_imports_create_file_edges_and_ignore_comments() {
    let root = unique_temp_dir();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("src/a.ts"),
        r#"
        // import commented from "./commented";
        export async function load() {
            return import("./b");
        }
        "#,
    )
    .unwrap();
    fs::write(root.join("src/b.ts"), "export const value = 1;\n").unwrap();
    fs::write(root.join("src/commented.ts"), "export const fake = 1;\n").unwrap();

    let output = scan_dir(&root);

    assert_edge(&output.graph.edges, "src/a.ts", "src/b.ts");
    assert_no_edge(&output.graph.edges, "src/a.ts", "src/commented.ts");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn typescript_directory_imports_resolve_to_index_files() {
    let root = unique_temp_dir();
    fs::create_dir_all(root.join("src/lib")).unwrap();
    fs::write(root.join("src/main.ts"), "import { value } from './lib';\n").unwrap();
    fs::write(root.join("src/lib/index.ts"), "export const value = 1;\n").unwrap();

    let output = scan_dir(&root);

    assert_edge(&output.graph.edges, "src/main.ts", "src/lib/index.ts");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn typescript_tsconfig_paths_resolve_alias_imports() {
    let root = unique_temp_dir();
    fs::create_dir_all(root.join("src/lib")).unwrap();
    fs::write(
        root.join("tsconfig.json"),
        r#"{
          "compilerOptions": {
            "baseUrl": ".",
            "paths": {
              "@/*": ["src/*"],
              "@lib/*": ["src/lib/*"]
            }
          }
        }"#,
    )
    .unwrap();
    fs::write(
        root.join("src/main.ts"),
        "import { value } from '@/lib/value';\nimport { other } from '@lib/other';\n",
    )
    .unwrap();
    fs::write(root.join("src/lib/value.ts"), "export const value = 1;\n").unwrap();
    fs::write(root.join("src/lib/other.ts"), "export const other = 2;\n").unwrap();

    let output = scan_dir(&root);

    assert_edge(&output.graph.edges, "src/main.ts", "src/lib/value.ts");
    assert_edge(&output.graph.edges, "src/main.ts", "src/lib/other.ts");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn typescript_uses_nearest_tsconfig_for_monorepo_packages() {
    let root = unique_temp_dir();
    fs::create_dir_all(root.join("packages/app/src/lib")).unwrap();
    fs::create_dir_all(root.join("packages/other/src/lib")).unwrap();
    fs::write(
        root.join("packages/app/tsconfig.json"),
        r#"{"compilerOptions":{"baseUrl":".","paths":{"@/*":["src/*"]}}}"#,
    )
    .unwrap();
    fs::write(
        root.join("packages/other/tsconfig.json"),
        r#"{"compilerOptions":{"baseUrl":".","paths":{"@/*":["src/*"]}}}"#,
    )
    .unwrap();
    fs::write(
        root.join("packages/app/src/main.ts"),
        "import { value } from '@/lib/value';\n",
    )
    .unwrap();
    fs::write(
        root.join("packages/app/src/lib/value.ts"),
        "export const value = 1;\n",
    )
    .unwrap();
    fs::write(
        root.join("packages/other/src/lib/value.ts"),
        "export const value = 2;\n",
    )
    .unwrap();

    let output = scan_dir(&root);

    assert_edge(
        &output.graph.edges,
        "packages/app/src/main.ts",
        "packages/app/src/lib/value.ts",
    );
    assert_no_edge(
        &output.graph.edges,
        "packages/app/src/main.ts",
        "packages/other/src/lib/value.ts",
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn python_relative_imports_create_file_edges_and_ignore_comments() {
    let root = unique_temp_dir();
    fs::create_dir_all(root.join("pkg/utils")).unwrap();
    fs::write(
        root.join("pkg/main.py"),
        r#"
        # import commented
        def load():
            from .utils import thing
        "#,
    )
    .unwrap();
    fs::write(root.join("pkg/utils/__init__.py"), "thing = 1\n").unwrap();
    fs::write(root.join("pkg/commented.py"), "fake = 1\n").unwrap();

    let output = scan_dir(&root);

    assert_edge(&output.graph.edges, "pkg/main.py", "pkg/utils/__init__.py");
    assert_no_edge(&output.graph.edges, "pkg/main.py", "pkg/commented.py");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn python_relative_imports_resolve_to_file_modules() {
    let root = unique_temp_dir();
    fs::create_dir_all(root.join("pkg")).unwrap();
    fs::write(
        root.join("pkg/main.py"),
        r#"
        from .utils import thing
        "#,
    )
    .unwrap();
    fs::write(root.join("pkg/utils.py"), "thing = 1\n").unwrap();

    let output = scan_dir(&root);

    assert_edge(&output.graph.edges, "pkg/main.py", "pkg/utils.py");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn go_module_imports_create_file_edges() {
    let root = unique_temp_dir();
    fs::create_dir_all(root.join("pkg/tool")).unwrap();
    fs::write(root.join("go.mod"), "module example.com/modkei\n").unwrap();
    fs::write(
        root.join("main.go"),
        r#"
        package main

        import "example.com/modkei/pkg/tool"

        func main() {}
        "#,
    )
    .unwrap();
    fs::write(root.join("pkg/tool/tool.go"), "package tool\n").unwrap();

    let output = scan_dir(&root);

    assert_edge(&output.graph.edges, "main.go", "pkg/tool/tool.go");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn go_uses_nearest_go_mod_for_nested_modules() {
    let root = unique_temp_dir();
    fs::create_dir_all(root.join("services/app/pkg/tool")).unwrap();
    fs::create_dir_all(root.join("other/pkg/tool")).unwrap();
    fs::write(root.join("services/app/go.mod"), "module example.com/app\n").unwrap();
    fs::write(root.join("other/go.mod"), "module example.com/other\n").unwrap();
    fs::write(
        root.join("services/app/main.go"),
        r#"
        package main
        import "example.com/app/pkg/tool"
        func main() {}
        "#,
    )
    .unwrap();
    fs::write(root.join("services/app/pkg/tool/tool.go"), "package tool\n").unwrap();
    fs::write(root.join("other/pkg/tool/tool.go"), "package tool\n").unwrap();

    let output = scan_dir(&root);

    assert_edge(
        &output.graph.edges,
        "services/app/main.go",
        "services/app/pkg/tool/tool.go",
    );
    assert_no_edge(
        &output.graph.edges,
        "services/app/main.go",
        "other/pkg/tool/tool.go",
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn c_includes_create_file_edges_and_ignore_comments() {
    let root = unique_temp_dir();
    fs::create_dir_all(root.join("include")).unwrap();
    fs::write(
        root.join("main.c"),
        r#"
        // #include "commented.h"
        #include "real.h"
        "#,
    )
    .unwrap();
    fs::write(root.join("include/real.h"), "void real(void);\n").unwrap();
    fs::write(root.join("include/commented.h"), "void fake(void);\n").unwrap();

    let output = scan_dir(&root);

    assert_edge(&output.graph.edges, "main.c", "include/real.h");
    assert_no_edge(&output.graph.edges, "main.c", "include/commented.h");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn gitignore_is_respected_by_default() {
    let root = unique_temp_dir();
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join(".gitignore"), "ignored.ts\n").unwrap();
    fs::write(root.join("kept.ts"), "export const kept = 1;\n").unwrap();
    fs::write(root.join("ignored.ts"), "export const ignored = 1;\n").unwrap();

    let output = scan_dir(&root);

    assert!(output.graph.nodes.iter().any(|node| node.id == "kept.ts"));
    assert!(
        !output
            .graph
            .nodes
            .iter()
            .any(|node| node.id == "ignored.ts")
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
