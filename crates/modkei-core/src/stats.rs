use crate::Language;

pub fn count_lines(source: &str, language: Language) -> (u64, u64, u64, u64) {
    let syntax = language.comment_syntax();
    let mut lines = 0;
    let mut code = 0;
    let mut comments = 0;
    let mut blanks = 0;
    let mut in_block: Option<&str> = None;

    for line in source.lines() {
        lines += 1;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            blanks += 1;
            continue;
        }
        if let Some(end) = in_block {
            comments += 1;
            if trimmed.contains(end) {
                in_block = None;
            }
            continue;
        }
        if syntax.line.iter().any(|prefix| trimmed.starts_with(prefix)) {
            comments += 1;
            continue;
        }
        if let Some((_, end)) = syntax
            .block
            .iter()
            .find(|(start, _)| trimmed.starts_with(start))
        {
            comments += 1;
            if !trimmed.contains(end) {
                in_block = Some(*end);
            }
            continue;
        }
        code += 1;
    }

    (lines, code, comments, blanks)
}
