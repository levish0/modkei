use super::*;

#[test]
fn extracts_mods_and_use_trees() {
    assert_eq!(
        pairs(
            r#"
            mod parser;
            pub use parser::parse;
            use crate::graph::{Edge, Node};
            use super::module::Resolver as ModuleResolver;
            fn load() {
                use self::local::Thing;
            }
            // use crate::commented::Fake;
            "#,
            Language::Rust,
        ),
        vec![
            ("crate::graph::Edge".to_string(), RawImportKind::Symbol),
            ("crate::graph::Node".to_string(), RawImportKind::Symbol),
            ("parser".to_string(), RawImportKind::Module),
            ("parser::parse".to_string(), RawImportKind::Symbol),
            ("self::local::Thing".to_string(), RawImportKind::Symbol),
            ("super::module::Resolver".to_string(), RawImportKind::Symbol),
        ]
    );
}
