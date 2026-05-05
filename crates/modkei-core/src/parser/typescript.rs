use tree_sitter::Node;

use super::collect_string_literals;

pub fn extract(root: Node<'_>, bytes: &[u8]) -> Vec<String> {
    let mut imports = Vec::new();
    collect(root, bytes, &mut imports);
    imports
}

fn collect(node: Node<'_>, bytes: &[u8], imports: &mut Vec<String>) {
    if matches!(node.kind(), "import_statement" | "export_statement") {
        collect_string_literals(node, bytes, imports, "module:");
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect(child, bytes, imports);
    }
}
