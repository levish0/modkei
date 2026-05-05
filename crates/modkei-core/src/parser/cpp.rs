use tree_sitter::Node;

use super::text;

pub fn extract(root: Node<'_>, bytes: &[u8]) -> Vec<String> {
    let mut imports = Vec::new();
    let mut stack = vec![root];
    let mut cursor = root.walk();

    while let Some(node) = stack.pop() {
        if node.kind() == "preproc_include" {
            // Find the string_literal or system_lib_string inside
            let mut include_cursor = node.walk();
            for child in node.children(&mut include_cursor) {
                if matches!(child.kind(), "string_literal" | "system_lib_string") {
                    let mut val = text(child, bytes).trim();
                    val = val.trim_matches('"').trim_matches('<').trim_matches('>');
                    imports.push(format!("include:{}", val));
                }
            }
        }

        for child in node.children(&mut cursor) {
            stack.push(child);
        }
    }
    imports
}
