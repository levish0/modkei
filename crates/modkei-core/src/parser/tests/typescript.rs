use super::*;

#[test]
fn extracts_static_dynamic_side_effect_and_reexports() {
    assert_eq!(
        pairs(
            r#"
            import "reflect-metadata";
            import Graph from "graphology";
            import { x as y } from "./local";
            export * from "../shared";
            export { z } from "./reexport";
            async function load() {
                return import("./dynamic");
            }
            // import fake from "./commented";
            "#,
            Language::TypeScript,
        ),
        vec![
            ("../shared".to_string(), RawImportKind::ReExport),
            ("./dynamic".to_string(), RawImportKind::Dynamic),
            ("./local".to_string(), RawImportKind::Module),
            ("./reexport".to_string(), RawImportKind::ReExport),
            ("graphology".to_string(), RawImportKind::Module),
            ("reflect-metadata".to_string(), RawImportKind::SideEffect),
        ]
    );
}
