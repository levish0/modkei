mod c_family;
mod go;
mod java;
mod python;
mod rust;
mod shell;
mod typescript;

use super::*;

fn pairs(source: &str, language: Language) -> Vec<(String, RawImportKind)> {
    extract_imports(source, language)
        .into_iter()
        .map(|import| (import.target, import.kind))
        .collect()
}

fn triples(source: &str, language: Language) -> Vec<(String, RawImportKind, Vec<String>)> {
    extract_imports(source, language)
        .into_iter()
        .map(|import| (import.target, import.kind, import.symbols))
        .collect()
}
