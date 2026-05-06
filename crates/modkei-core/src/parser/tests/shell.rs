use super::*;

#[test]
fn extracts_bash_make_and_cmake_includes_without_comments() {
    assert_eq!(
        pairs(
            r#"
            # source commented.sh
source real.sh
. ./other.sh
            "#,
            Language::Bash,
        ),
        vec![
            ("./other.sh".to_string(), RawImportKind::Include),
            ("real.sh".to_string(), RawImportKind::Include),
        ]
    );

    assert_eq!(
        pairs(
            "# include commented.mk\ninclude real.mk\n-include optional.mk\nsinclude fallback.mk\n",
            Language::Make,
        ),
        vec![
            ("fallback.mk".to_string(), RawImportKind::Include),
            ("optional.mk".to_string(), RawImportKind::Include),
            ("real.mk".to_string(), RawImportKind::Include),
        ]
    );

    assert_eq!(
        pairs(
            r#"
            # include(commented)
            include(real)
            add_subdirectory(src/lib)
            "#,
            Language::CMake,
        ),
        vec![
            ("real".to_string(), RawImportKind::Include),
            ("src/lib".to_string(), RawImportKind::Include),
        ]
    );
}
