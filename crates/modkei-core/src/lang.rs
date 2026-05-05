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
    C,
    Cpp,
    Java,
    Kotlin,
    Bash,
    Make,
    CMake,
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
            Some("c" | "h") => Self::C,
            Some("cpp" | "cxx" | "cc" | "hpp" | "hxx") => Self::Cpp,
            Some("java") => Self::Java,
            Some("kt" | "kts") => Self::Kotlin,
            Some("sh" | "bash" | "zsh") => Self::Bash,
            Some("mk") => Self::Make,
            Some("cmake") => Self::CMake,
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
            Self::C => "C",
            Self::Cpp => "C++",
            Self::Java => "Java",
            Self::Kotlin => "Kotlin",
            Self::Bash => "Bash",
            Self::Make => "Makefile",
            Self::CMake => "CMake",
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
            Self::C => Some(tree_sitter_c::LANGUAGE.into()),
            Self::Cpp => Some(tree_sitter_cpp::LANGUAGE.into()),
            Self::Java => Some(tree_sitter_java::LANGUAGE.into()),
            Self::Kotlin => None, // Temporarily disabled due to tree-sitter version conflict
            Self::Bash => Some(tree_sitter_bash::LANGUAGE.into()),
            Self::Make => Some(tree_sitter_make::LANGUAGE.into()),
            Self::CMake => Some(tree_sitter_cmake::LANGUAGE.into()),
            Self::Unknown => None,
        }
    }

    pub(crate) fn comment_syntax(self) -> CommentSyntax {
        match self {
            Self::Rust
            | Self::TypeScript
            | Self::JavaScript
            | Self::Go
            | Self::C
            | Self::Cpp
            | Self::Java
            | Self::Kotlin => CommentSyntax {
                line: &["//"],
                block: &[("/*", "*/")],
            },
            Self::Python | Self::Bash | Self::Make | Self::CMake => CommentSyntax {
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
