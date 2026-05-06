use super::*;

#[test]
fn extracts_normal_and_static_imports() {
    assert_eq!(
        pairs(
            r#"
            // import commented.Fake;
            import java.util.List;
            import static java.util.Collections.emptyList;
            "#,
            Language::Java,
        ),
        vec![
            (
                "java.util.Collections.emptyList".to_string(),
                RawImportKind::Module,
            ),
            ("java.util.List".to_string(), RawImportKind::Module),
        ]
    );
}
