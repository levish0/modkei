use tree_sitter::Node;

use super::{RawImport, RawImportKind, collect_string_literals};

pub fn extract(root: Node<'_>, bytes: &[u8]) -> Vec<RawImport> {
    let mut imports = Vec::new();
    collect(root, bytes, &mut imports);
    imports
}

fn collect(root: Node<'_>, bytes: &[u8], imports: &mut Vec<RawImport>) {
    let mut stack = vec![root];

    while let Some(node) = stack.pop() {
        if matches!(node.kind(), "import_declaration" | "import_spec") {
            collect_string_literals(node, bytes, imports, RawImportKind::Module);
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            stack.push(child);
        }
    }
}
