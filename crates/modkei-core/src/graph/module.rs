use std::{collections::HashSet, path::Path};

use super::resolve_candidate;

pub fn resolve_relative(
    from: &Path,
    raw: &str,
    root: &Path,
    rel_set: &HashSet<String>,
    exts: &[&str],
) -> Option<String> {
    let module = raw.strip_prefix("module:")?;
    if !module.starts_with('.') {
        return None;
    }
    let candidate = from.parent()?.join(module);
    resolve_candidate(root, &candidate, rel_set, exts)
}

pub fn resolve_python(
    from: &Path,
    raw: &str,
    root: &Path,
    rel_set: &HashSet<String>,
) -> Option<String> {
    let module = raw.strip_prefix("module:")?;
    if module.starts_with('.') {
        // Python relative imports: count leading dots
        let dots = module.chars().take_while(|c| *c == '.').count();
        let rest = &module[dots..];

        let mut base = from.parent()?;
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
    resolve_candidate(root, Path::new(&module.replace('.', "/")), rel_set, &["py"])
}

pub fn resolve_go(raw: &str, rel_set: &HashSet<String>) -> Option<String> {
    let suffix = raw.strip_prefix("module:")?.replace('\\', "/");
    rel_set
        .iter()
        .find(|path| {
            path.contains(&format!("{suffix}/")) || path.ends_with(&format!("{suffix}.go"))
        })
        .cloned()
}
