use std::path::Path;

use serde::Serialize;

#[derive(Debug, Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum Language {
    Rust,
    Unknown,
}

impl Language {
    pub fn from_path(path: &Path) -> Self {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("rs") => Self::Rust,
            _ => Self::Unknown,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Rust => "Rust",
            Self::Unknown => "Unknown",
        }
    }

    pub(crate) fn comment_syntax(self) -> CommentSyntax {
        match self {
            Self::Rust => CommentSyntax {
                line: &["//"],
                block: &[("/*", "*/")],
            },
            Self::Unknown => CommentSyntax {
                line: &[],
                block: &[],
            },
        }
    }

    pub fn is_supported(self) -> bool {
        !matches!(self, Self::Unknown)
    }
}

pub(crate) struct CommentSyntax {
    pub line: &'static [&'static str],
    pub block: &'static [(&'static str, &'static str)],
}
