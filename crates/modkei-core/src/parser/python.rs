use tree_sitter::Node;

use super::text;

pub fn extract(root: Node<'_>, bytes: &[u8]) -> Vec<String> {
    let mut imports = Vec::new();
    collect(root, bytes, &mut imports);
    imports
}

fn collect(node: Node<'_>, bytes: &[u8], imports: &mut Vec<String>) {
    match node.kind() {
        "import_statement" => collect_import_statement(node, bytes, imports),
        "import_from_statement" => {
            if let Some(target) = find_from_import_target(node, bytes) {
                imports.push(format!("module:{target}"));
            }
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect(child, bytes, imports);
    }
}

fn collect_import_statement(node: Node<'_>, bytes: &[u8], imports: &mut Vec<String>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "dotted_name" {
            imports.push(format!("module:{}", text(child, bytes)));
        } else {
            collect_import_statement(child, bytes, imports);
        }
    }
}

fn find_from_import_target(node: Node<'_>, bytes: &[u8]) -> Option<String> {
    if matches!(node.kind(), "dotted_name" | "relative_import") {
        return Some(text(node, bytes).to_string());
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(target) = find_from_import_target(child, bytes) {
            return Some(target);
        }
    }
    None
}
