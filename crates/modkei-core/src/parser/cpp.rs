use tree_sitter::Node;

use super::{RawImport, RawImportKind, text};

pub fn extract(root: Node<'_>, bytes: &[u8]) -> Vec<RawImport> {
    let mut imports = Vec::new();
    let mut stack = vec![root];

    while let Some(node) = stack.pop() {
        if node.kind() == "preproc_include" {
            let mut include_cursor = node.walk();
            for child in node.children(&mut include_cursor) {
                if matches!(child.kind(), "string_literal" | "system_lib_string") {
                    let mut val = text(child, bytes).trim();
                    val = val.trim_matches('"').trim_matches('<').trim_matches('>');
                    imports.push(RawImport::new(val, RawImportKind::Include, child));
                }
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            stack.push(child);
        }
    }
    imports
}
