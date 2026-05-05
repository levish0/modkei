use tree_sitter::Node;

use super::{collect_identifiers, text};

pub fn extract(root: Node<'_>, bytes: &[u8]) -> Vec<String> {
    let mut imports = Vec::new();
    collect(root, bytes, &mut imports);
    imports
}

fn collect(node: Node<'_>, bytes: &[u8], imports: &mut Vec<String>) {
    match node.kind() {
        "mod_item" if text(node, bytes).trim().ends_with(';') => {
            collect_mod_target(node, bytes, imports)
        }
        "use_declaration" => collect_use_targets(node, bytes, imports),
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect(child, bytes, imports);
    }
}

fn collect_mod_target(node: Node<'_>, bytes: &[u8], imports: &mut Vec<String>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            imports.push(format!("mod:{}", text(child, bytes)));
            return;
        }
    }
}

fn collect_use_targets(node: Node<'_>, bytes: &[u8], imports: &mut Vec<String>) {
    let mut identifiers = Vec::new();
    collect_identifiers(node, bytes, &mut identifiers);
    for identifier in identifiers {
        if !matches!(identifier.as_str(), "crate" | "self" | "super") {
            imports.push(format!("use:{identifier}"));
        }
    }
}
