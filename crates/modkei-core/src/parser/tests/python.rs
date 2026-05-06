use super::*;

#[test]
fn extracts_imports_from_imports_aliases_and_nested_imports() {
    assert_eq!(
        triples(
            r#"
            import os, sys
            import package.module as module
            from .utils import thing, other as alias
            from ..core.config import Settings
            def load():
                import nested.real
            # from commented import fake
            "#,
            Language::Python,
        ),
        vec![
            (
                "..core.config".to_string(),
                RawImportKind::Symbol,
                vec!["Settings".to_string()],
            ),
            (
                ".utils".to_string(),
                RawImportKind::Symbol,
                vec!["thing".to_string(), "other".to_string()],
            ),
            ("nested.real".to_string(), RawImportKind::Module, Vec::new()),
            ("os".to_string(), RawImportKind::Module, Vec::new()),
            (
                "package.module".to_string(),
                RawImportKind::Module,
                Vec::new(),
            ),
            ("sys".to_string(), RawImportKind::Module, Vec::new()),
        ]
    );
}
