use tree_sitter::Node;

use super::{RawImport, RawImportKind};

pub fn extract(root: Node<'_>, bytes: &[u8]) -> Vec<RawImport> {
    let mut imports = Vec::new();
    let mut stack = vec![root];

    while let Some(node) = stack.pop() {
        if node.kind() == "command" {
            let mut command_cursor = node.walk();
            let mut is_source = false;
            for child in node.children(&mut command_cursor) {
                if child.kind() == "command_name" {
                    let name = super::text(child, bytes).trim();
                    if name == "source" || name == "." {
                        is_source = true;
                    }
                } else if is_source {
                    // Extract the string
                    let val = super::text(child, bytes)
                        .trim()
                        .trim_matches('"')
                        .trim_matches('\'');
                    imports.push(RawImport::new(val, RawImportKind::Include, child));
                    break;
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
