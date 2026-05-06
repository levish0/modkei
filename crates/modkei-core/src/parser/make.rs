use tree_sitter::Node;

use super::{RawImport, RawImportKind};

pub fn extract(root: Node<'_>, bytes: &[u8]) -> Vec<RawImport> {
    let mut imports = Vec::new();
    let mut stack = vec![root];

    while let Some(node) = stack.pop() {
        if node.kind() == "include_directive" {
            collect_include_targets(node, bytes, &mut imports);
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            stack.push(child);
        }
    }
    imports
}

fn collect_include_targets(root: Node<'_>, bytes: &[u8], imports: &mut Vec<RawImport>) {
    let mut stack = vec![root];

    while let Some(node) = stack.pop() {
        if matches!(node.kind(), "word" | "string") {
            let val = super::text(node, bytes)
                .trim()
                .trim_matches('"')
                .trim_matches('\'');
            if !matches!(val, "include" | "-include" | "sinclude") {
                imports.push(RawImport::new(val, RawImportKind::Include, node));
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            stack.push(child);
        }
    }
}
