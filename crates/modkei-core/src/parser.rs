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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_extracts_use_trees_as_module_paths() {
        let imports = extract_imports(
            r#"
            mod parser;
            pub use parser::parse;
            use crate::graph::{Edge, Node};
            use super::module::Resolver as ModuleResolver;
            "#,
            Language::Rust,
        );

        assert_eq!(
            imports,
            vec![
                "mod:parser",
                "use:crate::graph::Edge",
                "use:crate::graph::Node",
                "use:parser::parse",
                "use:super::module::Resolver"
            ]
        );
    }

    #[test]
    fn typescript_extracts_import_sources() {
        let imports = extract_imports(
            r#"
            import Graph from "graphology";
            import { x } from "./local";
            export * from "../shared";
            "#,
            Language::TypeScript,
        );

        assert_eq!(
            imports,
            vec!["module:../shared", "module:./local", "module:graphology"]
        );
    }

    #[test]
    fn python_extracts_import_sources() {
        let imports = extract_imports(
            r#"
            import os
            import package.module as module
            from .utils import thing
            "#,
            Language::Python,
        );

        assert_eq!(
            imports,
            vec!["module:.utils", "module:os", "module:package.module"]
        );
    }

    #[test]
    fn go_extracts_import_sources() {
        let imports = extract_imports(
            r#"
            import "fmt"
            import (
                "example.com/project/pkg"
                alias "example.com/project/internal/tool"
            )
            "#,
            Language::Go,
        );

        assert_eq!(
            imports,
            vec![
                "module:example.com/project/internal/tool",
                "module:example.com/project/pkg",
                "module:fmt"
            ]
        );
    }
}
