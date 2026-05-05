use tree_sitter::Node;

use super::text;

pub fn extract(root: Node<'_>, bytes: &[u8]) -> Vec<String> {
    let mut imports = Vec::new();
    collect(root, bytes, &mut imports);
    imports
}

fn collect(node: Node<'_>, bytes: &[u8], imports: &mut Vec<String>) {
    if matches!(node.kind(), "import_statement" | "import_from_statement") {
        collect_import_target(node, bytes, imports);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect(child, bytes, imports);
    }
}

fn collect_import_target(node: Node<'_>, bytes: &[u8], imports: &mut Vec<String>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "dotted_name" | "relative_import" => {
                imports.push(format!("module:{}", text(child, bytes)));
            }
            _ => collect_import_target(child, bytes, imports),
        }
    }
}
