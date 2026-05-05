use tree_sitter::Node;

pub fn extract(root: Node<'_>, bytes: &[u8]) -> Vec<String> {
    let mut imports = Vec::new();
    let mut stack = vec![root];
    let mut cursor = root.walk();

    while let Some(node) = stack.pop() {
        if node.kind() == "include_directive" {
            let mut directive_cursor = node.walk();
            for child in node.children(&mut directive_cursor) {
                if child.kind() == "word" {
                    let val = super::text(child, bytes).trim();
                    if val != "include" && val != "-include" && val != "sinclude" {
                        imports.push(format!("include:{}", val));
                    }
                }
            }
        }

        for child in node.children(&mut cursor) {
            stack.push(child);
        }
    }
    imports
}
