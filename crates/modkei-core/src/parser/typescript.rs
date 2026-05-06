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
        match node.kind() {
            "import_statement" => {
                collect_string_literals(node, bytes, imports, import_kind(node, bytes))
            }
            "export_statement" => {
                collect_string_literals(node, bytes, imports, RawImportKind::ReExport)
            }
            "call_expression" if text_starts_with_import(node, bytes) => {
                collect_string_literals(node, bytes, imports, RawImportKind::Dynamic)
            }
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            stack.push(child);
        }
    }
}

fn import_kind(node: Node<'_>, bytes: &[u8]) -> RawImportKind {
    let source = super::text(node, bytes);
    if source.trim_start().starts_with("import \"") || source.trim_start().starts_with("import '") {
        RawImportKind::SideEffect
    } else {
        RawImportKind::Module
    }
}

fn text_starts_with_import(node: Node<'_>, bytes: &[u8]) -> bool {
    super::text(node, bytes).trim_start().starts_with("import(")
}
