use tree_sitter::Node;

use super::{RawImport, RawImportKind};

pub fn extract(root: Node<'_>, bytes: &[u8]) -> Vec<RawImport> {
    let mut imports = Vec::new();
    let mut stack = vec![root];

    while let Some(node) = stack.pop() {
        if matches!(node.kind(), "import_declaration") {
            let mut import_cursor = node.walk();
            for child in node.children(&mut import_cursor) {
                if child.kind() == "scoped_identifier" || child.kind() == "identifier" {
                    let val = super::text(child, bytes).trim();
                    imports.push(RawImport::new(val, RawImportKind::Module, child));
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
