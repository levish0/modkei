use tree_sitter::Node;

use super::collect_string_literals;

pub fn extract(root: Node<'_>, bytes: &[u8]) -> Vec<String> {
    let mut imports = Vec::new();
    collect(root, bytes, &mut imports);
    imports
}

fn collect(root: Node<'_>, bytes: &[u8], imports: &mut Vec<String>) {
    let mut stack = vec![root];
    let mut cursor = root.walk();

    while let Some(node) = stack.pop() {
        if matches!(node.kind(), "import_statement" | "export_statement") {
            collect_string_literals(node, bytes, imports, "module:");
        }

        for child in node.children(&mut cursor) {
            stack.push(child);
        }
    }
}
