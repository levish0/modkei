mod c_family;
mod common;
mod go;
mod python;
mod rust;
mod typescript;

use std::{collections::HashSet, path::Path};

use crate::{ImportEdge, Language};

pub fn resolve(root: &Path, import: &ImportEdge, rel_set: &HashSet<String>) -> Option<String> {
    match import.language {
        Language::TypeScript | Language::JavaScript => typescript::resolve(root, import, rel_set),
        Language::Python => python::resolve(root, import, rel_set),
        Language::Rust => rust::resolve(root, import, rel_set),
        Language::Go => go::resolve(root, import, rel_set),
        Language::C | Language::Cpp => c_family::resolve(root, import, rel_set),
        Language::Unknown | Language::Java | Language::Bash | Language::Make | Language::CMake => {
            None
        }
    }
}
