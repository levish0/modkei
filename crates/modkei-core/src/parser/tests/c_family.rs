use super::*;

#[test]
fn extracts_quoted_and_system_includes() {
    assert_eq!(
        pairs(
            r#"
            // #include "commented.h"
            #include "local.h"
            #include <stdio.h>
            "#,
            Language::C,
        ),
        vec![
            ("local.h".to_string(), RawImportKind::Include),
            ("stdio.h".to_string(), RawImportKind::Include),
        ]
    );

    assert_eq!(
        pairs(
            r#"
            /* #include "commented.hpp" */
            #include "local.hpp"
            #include <vector>
            "#,
            Language::Cpp,
        ),
        vec![
            ("local.hpp".to_string(), RawImportKind::Include),
            ("vector".to_string(), RawImportKind::Include),
        ]
    );
}
