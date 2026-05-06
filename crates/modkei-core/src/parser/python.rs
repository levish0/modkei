use tree_sitter::Node;

use super::{RawImport, RawImportKind, text};

pub fn extract(root: Node<'_>, bytes: &[u8]) -> Vec<RawImport> {
    let mut imports = Vec::new();
    collect(root, bytes, &mut imports);
    imports
}

fn collect(root: Node<'_>, bytes: &[u8], imports: &mut Vec<RawImport>) {
    let mut stack = vec![root];

    while let Some(node) = stack.pop() {
        match node.kind() {
            "import_statement" => collect_import_statement(node, bytes, imports),
            "import_from_statement" => {
                if let Some(target) = find_from_import_target(node, bytes) {
                    imports.push(
                        RawImport::new(target, RawImportKind::Symbol, node)
                            .with_symbols(imported_symbols(node, bytes)),
                    );
                }
            }
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            stack.push(child);
        }
    }
}

fn collect_import_statement(root: Node<'_>, bytes: &[u8], imports: &mut Vec<RawImport>) {
    let mut stack = vec![root];

    while let Some(node) = stack.pop() {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "dotted_name" {
                imports.push(RawImport::new(
                    text(child, bytes),
                    RawImportKind::Module,
                    child,
                ));
            } else {
                stack.push(child);
            }
        }
    }
}

fn find_from_import_target(root: Node<'_>, bytes: &[u8]) -> Option<String> {
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if child.kind() == "import" {
            return None;
        }
        if matches!(child.kind(), "dotted_name" | "relative_import") {
            return Some(text(child, bytes).to_string());
        }
        if let Some(target) = find_from_import_target(child, bytes) {
            return Some(target);
        }
    }
    None
}

fn imported_symbols(root: Node<'_>, bytes: &[u8]) -> Vec<String> {
    let mut symbols = Vec::new();
    let mut saw_import = false;
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if child.kind() == "import" {
            saw_import = true;
            continue;
        }
        if saw_import {
            collect_symbol_names(child, bytes, &mut symbols);
        }
    }
    symbols
}

fn collect_symbol_names(root: Node<'_>, bytes: &[u8], symbols: &mut Vec<String>) {
    let mut stack = vec![root];

    while let Some(node) = stack.pop() {
        match node.kind() {
            "aliased_import" => {
                if let Some(name) = node.child_by_field_name("name") {
                    symbols.push(text(name, bytes).to_string());
                }
            }
            "dotted_name" | "identifier" => symbols.push(text(node, bytes).to_string()),
            "as_pattern" => {
                let mut cursor = node.walk();
                if let Some(first) = node.children(&mut cursor).next() {
                    collect_symbol_names(first, bytes, symbols);
                }
            }
            _ => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    stack.push(child);
                }
            }
        }
    }
}
