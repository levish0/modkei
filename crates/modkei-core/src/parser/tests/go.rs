use super::*;

#[test]
fn extracts_single_group_alias_blank_and_dot_imports() {
    assert_eq!(
        pairs(
            r#"
            package main

            import "fmt"
            import (
                . "example.com/project/dot"
                _ "example.com/project/blank"
                alias "example.com/project/internal/tool"
                "example.com/project/pkg"
            )
            // import "example.com/project/commented"
            "#,
            Language::Go,
        ),
        vec![
            (
                "example.com/project/blank".to_string(),
                RawImportKind::Module
            ),
            ("example.com/project/dot".to_string(), RawImportKind::Module),
            (
                "example.com/project/internal/tool".to_string(),
                RawImportKind::Module,
            ),
            ("example.com/project/pkg".to_string(), RawImportKind::Module),
            ("fmt".to_string(), RawImportKind::Module),
        ]
    );
}
