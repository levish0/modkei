use tree_sitter::Node;

use super::text;

pub fn extract(root: Node<'_>, bytes: &[u8]) -> Vec<String> {
    let mut imports = Vec::new();
    collect(root, bytes, &mut imports);
    imports
}

fn collect(root: Node<'_>, bytes: &[u8], imports: &mut Vec<String>) {
    let mut stack = vec![root];
    let mut cursor = root.walk();

    while let Some(node) = stack.pop() {
        match node.kind() {
            "import_statement" => collect_import_statement(node, bytes, imports),
            "import_from_statement" => {
                if let Some(target) = find_from_import_target(node, bytes) {
                    imports.push(format!("module:{target}"));
                }
            }
            _ => {}
        }

        for child in node.children(&mut cursor) {
            stack.push(child);
        }
    }
}

fn collect_import_statement(root: Node<'_>, bytes: &[u8], imports: &mut Vec<String>) {
    let mut stack = vec![root];
    let mut cursor = root.walk();

    while let Some(node) = stack.pop() {
        for child in node.children(&mut cursor) {
            if child.kind() == "dotted_name" {
                imports.push(format!("module:{}", text(child, bytes)));
            } else {
                stack.push(child);
            }
        }
    }
}

fn find_from_import_target(root: Node<'_>, bytes: &[u8]) -> Option<String> {
    let mut stack = vec![root];
    let mut cursor = root.walk();

    while let Some(node) = stack.pop() {
        if matches!(node.kind(), "dotted_name" | "relative_import") {
            return Some(text(node, bytes).to_string());
        }

        for child in node.children(&mut cursor) {
            stack.push(child);
        }
    }
    None
}
