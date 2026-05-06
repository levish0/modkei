mod bash;
mod cmake;
mod cpp;
mod go;
mod java;
mod make;
mod python;
mod rust;
mod typescript;

use serde::Serialize;
use tree_sitter::{Node, Parser};

use crate::Language;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
pub enum RawImportKind {
    Module,
    Symbol,
    SideEffect,
    Dynamic,
    ReExport,
    Include,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct RawImport {
    pub target: String,
    pub kind: RawImportKind,
    pub symbols: Vec<String>,
    pub byte_start: usize,
    pub byte_end: usize,
}

impl RawImport {
    pub fn new(target: impl Into<String>, kind: RawImportKind, node: Node<'_>) -> Self {
        Self {
            target: target.into(),
            kind,
            symbols: Vec::new(),
            byte_start: node.start_byte(),
            byte_end: node.end_byte(),
        }
    }

    pub fn with_symbols(mut self, symbols: Vec<String>) -> Self {
        self.symbols = symbols;
        self
    }
}

pub fn extract_imports(source: &str, language: Language) -> Vec<RawImport> {
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
        Language::C | Language::Cpp => cpp::extract(tree.root_node(), source.as_bytes()),
        Language::Java => java::extract(tree.root_node(), source.as_bytes()),
        Language::Bash => bash::extract(tree.root_node(), source.as_bytes()),
        Language::Make => make::extract(tree.root_node(), source.as_bytes()),
        Language::CMake => cmake::extract(tree.root_node(), source.as_bytes()),
        Language::Unknown => Vec::new(),
    };
    imports.retain(|item| !item.target.is_empty());
    imports.sort_by(|left, right| {
        left.target
            .cmp(&right.target)
            .then_with(|| left.byte_start.cmp(&right.byte_start))
            .then_with(|| left.byte_end.cmp(&right.byte_end))
    });
    imports.dedup_by(|left, right| {
        left.target == right.target
            && left.kind == right.kind
            && left.symbols == right.symbols
            && left.byte_start == right.byte_start
            && left.byte_end == right.byte_end
    });
    imports
}

pub(super) fn collect_string_literals(
    root: Node<'_>,
    bytes: &[u8],
    imports: &mut Vec<RawImport>,
    kind: RawImportKind,
) {
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        match node.kind() {
            "string_fragment" => imports.push(RawImport::new(text(node, bytes), kind, node)),
            "interpreted_string_literal" | "raw_string_literal" | "string" => {
                imports.push(RawImport::new(unquote(text(node, bytes)), kind, node));
            }
            _ => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    stack.push(child);
                }
            }
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

#[cfg(test)]
mod tests;
