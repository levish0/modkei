mod go;
mod python;
mod rust;
mod typescript;

use tree_sitter::{Node, Parser};

use crate::Language;

pub fn extract_imports(source: &str, language: Language) -> Vec<String> {
    let Some(ts_language) = language.tree_sitter() else {
        return Vec::new();
    };
    let mut parser = Parser::new();
    if parser.set_language(&ts_language).is_err() {
        return Vec::new();
    }
    let Some(tree) = parser.parse(source, None) else {
        return Vec::new();
    };
    let mut imports = match language {
        Language::Rust => rust::extract(tree.root_node(), source.as_bytes()),
        Language::TypeScript | Language::JavaScript => {
            typescript::extract(tree.root_node(), source.as_bytes())
        }
        Language::Python => python::extract(tree.root_node(), source.as_bytes()),
        Language::Go => go::extract(tree.root_node(), source.as_bytes()),
        Language::Unknown => Vec::new(),
    };
    imports.retain(|item| !item.is_empty());
    imports.sort();
    imports.dedup();
    imports
}

pub(super) fn collect_identifiers(node: Node<'_>, bytes: &[u8], identifiers: &mut Vec<String>) {
    if node.kind() == "identifier" {
        identifiers.push(text(node, bytes).to_string());
        return;
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_identifiers(child, bytes, identifiers);
    }
}

pub(super) fn collect_string_literals(
    node: Node<'_>,
    bytes: &[u8],
    imports: &mut Vec<String>,
    prefix: &str,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "string_fragment" => imports.push(format!("{prefix}{}", text(child, bytes))),
            "interpreted_string_literal" | "raw_string_literal" | "string" => {
                imports.push(format!("{prefix}{}", unquote(text(child, bytes))));
            }
            _ => collect_string_literals(child, bytes, imports, prefix),
        }
    }
}

pub(super) fn text<'a>(node: Node<'_>, bytes: &'a [u8]) -> &'a str {
    node.utf8_text(bytes).unwrap_or("")
}

pub(super) fn unquote(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim_matches('`')
        .to_string()
}
