use tree_sitter::Node;

use super::{RawImport, RawImportKind};

pub fn extract(root: Node<'_>, bytes: &[u8]) -> Vec<RawImport> {
    let mut imports = Vec::new();
    let mut stack = vec![root];

    while let Some(node) = stack.pop() {
        if node.kind() == "normal_command" {
            let mut cmd_cursor = node.walk();
            let mut is_include = false;
            for child in node.children(&mut cmd_cursor) {
                if child.kind() == "identifier" {
                    let name = super::text(child, bytes).trim().to_lowercase();
                    if name == "include" || name == "add_subdirectory" {
                        is_include = true;
                    }
                } else if is_include && child.kind() == "argument_list" {
                    let mut arg_cursor = child.walk();
                    for arg in child.children(&mut arg_cursor) {
                        if arg.kind() == "argument" {
                            let val = super::text(arg, bytes).trim().trim_matches('"');
                            imports.push(RawImport::new(val, RawImportKind::Include, arg));
                            break;
                        }
                    }
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
