use std::{collections::HashSet, path::Path};

use super::common::resolve_candidate;
use crate::ImportEdge;

pub fn resolve(root: &Path, import: &ImportEdge, rel_set: &HashSet<String>) -> Option<String> {
    let raw = &import.target;
    if raw.starts_with('.') {
        let dots = raw.chars().take_while(|c| *c == '.').count();
        let rest = &raw[dots..];

        let mut base = import.from.parent()?;
        for _ in 1..dots {
            base = base.parent()?;
        }

        let candidate = if rest.is_empty() {
            base.to_path_buf()
        } else {
            base.join(rest.replace('.', "/"))
        };

        return resolve_candidate(root, &candidate, rel_set, &["py"]);
    }
    resolve_candidate(root, Path::new(&raw.replace('.', "/")), rel_set, &["py"])
}
