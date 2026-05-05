use tree_sitter::Node;


pub fn extract(root: Node<'_>, bytes: &[u8]) -> Vec<String> {
    let mut imports = Vec::new();
    let mut stack = vec![root];
    let mut cursor = root.walk();

    while let Some(node) = stack.pop() {
        if matches!(node.kind(), "import_declaration") {
            // Java imports are usually identifiers like java.util.List, not string literals.
            // Wait, we need a custom text extractor.
            let mut import_cursor = node.walk();
            for child in node.children(&mut import_cursor) {
                if child.kind() == "scoped_identifier" || child.kind() == "identifier" {
                    let val = super::text(child, bytes).trim();
                    imports.push(format!("module:{}", val));
                }
            }
        }

        for child in node.children(&mut cursor) {
            stack.push(child);
        }
    }
    imports
}
