use std::path::Path;

use serde::Serialize;
use tree_sitter::Language as TsLanguage;

#[derive(Debug, Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum Language {
    Rust,
    TypeScript,
    Python,
    Go,
    JavaScript,
    Unknown,
}

impl Language {
    pub fn from_path(path: &Path) -> Self {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("rs") => Self::Rust,
            Some("ts" | "tsx") => Self::TypeScript,
            Some("js" | "jsx" | "mjs" | "cjs") => Self::JavaScript,
            Some("py") => Self::Python,
            Some("go") => Self::Go,
            _ => Self::Unknown,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Rust => "Rust",
            Self::TypeScript => "TypeScript",
            Self::Python => "Python",
            Self::Go => "Go",
            Self::JavaScript => "JavaScript",
            Self::Unknown => "Unknown",
        }
    }

    pub(crate) fn tree_sitter(self) -> Option<TsLanguage> {
        match self {
            Self::Rust => Some(tree_sitter_rust::LANGUAGE.into()),
            Self::TypeScript | Self::JavaScript => {
                Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
            }
            Self::Python => Some(tree_sitter_python::LANGUAGE.into()),
            Self::Go => Some(tree_sitter_go::LANGUAGE.into()),
            Self::Unknown => None,
        }
    }

    pub(crate) fn comment_syntax(self) -> CommentSyntax {
        match self {
            Self::Rust | Self::TypeScript | Self::JavaScript | Self::Go => CommentSyntax {
                line: &["//"],
                block: &[("/*", "*/")],
            },
            Self::Python => CommentSyntax {
                line: &["#"],
                block: &[],
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
