use tree_sitter::Node;

use super::text;

pub fn extract(root: Node<'_>, bytes: &[u8]) -> Vec<String> {
    let mut imports = Vec::new();
    collect(root, bytes, &mut imports);
    imports
}

fn collect(root: Node<'_>, bytes: &[u8], imports: &mut Vec<String>) {
    let mut stack = vec![root];
    let mut cursor = root.walk();

    while let Some(node) = stack.pop() {
        match node.kind() {
            "mod_item" if text(node, bytes).trim().ends_with(';') => {
                collect_mod_target(node, bytes, imports)
            }
            "use_declaration" => collect_use_targets(node, bytes, imports),
            _ => {}
        }

        for child in node.children(&mut cursor) {
            stack.push(child);
        }
    }
}

fn collect_mod_target(node: Node<'_>, bytes: &[u8], imports: &mut Vec<String>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            imports.push(format!("mod:{}", text(child, bytes)));
            return;
        }
    }
}

fn collect_use_targets(node: Node<'_>, bytes: &[u8], imports: &mut Vec<String>) {
    let declaration = text(node, bytes);
    let Some(use_tree) = declaration
        .split_once(" use ")
        .map(|(_, tree)| tree)
        .or_else(|| declaration.strip_prefix("use "))
    else {
        return;
    };

    collect_use_tree(use_tree.trim().trim_end_matches(';'), "", imports);
}

fn collect_use_tree(tree: &str, prefix: &str, imports: &mut Vec<String>) {
    let tree = strip_alias(tree.trim());
    if tree.is_empty() || tree == "*" {
        push_use(prefix, imports);
        return;
    }

    if let Some(open) = find_top_level(tree, '{') {
        let prefix_part = tree[..open].trim().trim_end_matches("::");
        let combined_prefix = join_path(prefix, prefix_part);
        let close = tree.rfind('}').unwrap_or(tree.len());
        let inner = &tree[open + 1..close];
        for item in split_top_level(inner) {
            collect_use_tree(item, &combined_prefix, imports);
        }
        return;
    }

    push_use(&join_path(prefix, tree), imports);
}

fn strip_alias(value: &str) -> &str {
    value.split(" as ").next().unwrap_or(value).trim()
}

fn push_use(path: &str, imports: &mut Vec<String>) {
    let path = path.trim().trim_matches(':');
    if !path.is_empty() {
        imports.push(format!("use:{path}"));
    }
}

fn join_path(prefix: &str, suffix: &str) -> String {
    let prefix = prefix.trim().trim_end_matches("::");
    let suffix = suffix.trim().trim_start_matches("::");
    match (prefix.is_empty(), suffix.is_empty()) {
        (true, true) => String::new(),
        (true, false) => suffix.to_string(),
        (false, true) => prefix.to_string(),
        (false, false) => format!("{prefix}::{suffix}"),
    }
}

fn find_top_level(value: &str, needle: char) -> Option<usize> {
    let mut depth = 0usize;
    for (index, ch) in value.char_indices() {
        match ch {
            '{' if ch == needle && depth == 0 => return Some(index),
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            _ if ch == needle && depth == 0 => return Some(index),
            _ => {}
        }
    }
    None
}

fn split_top_level(value: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (index, ch) in value.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                parts.push(value[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
    }
    parts.push(value[start..].trim());
    parts.into_iter().filter(|part| !part.is_empty()).collect()
}
