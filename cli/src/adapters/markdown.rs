//! Translation between the markdown documents and the domain model.
//!
//! These parsers are pure (text in, domain objects out) so they are unit
//! testable without any filesystem.

use crate::domain::model::{Entry, Model, PatternRow};

/// A tag token must be lowercase alnum or '-', otherwise it is prose that
/// leaked out of a backtick pair - never a tag.
fn is_tag_token(t: &str) -> bool {
    !t.is_empty() && t.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn parse_arch_tags(arch_tail: &str) -> Vec<String> {
    arch_tail
        .split('`')
        .map(str::trim)
        .filter(|t| is_tag_token(t))
        .map(str::to_string)
        .collect()
}

/// README.md -> Model. Only lines of the form
/// `- [name](url) - ... **Blazing:** ... **Arch:** \`tags\``
/// are entries; other `- [x](y)` list items are ignored.
pub fn parse_readme(readme: &str) -> Model {
    let mut model = Model::default();
    let mut section = String::new();
    for line in readme.lines() {
        if let Some(s) = line.strip_prefix("## ") {
            section = s.trim().to_string();
            if section != "Contents" && !model.sections.contains(&section) {
                model.sections.push(section.clone());
            }
        }
        if let Some(rest) = line.strip_prefix("- [") {
            let close = match rest.find(']') {
                Some(i) => i,
                None => continue,
            };
            let name = rest[..close].to_string();
            let tail = &rest[close + 1..];
            let span = match (tail.find('('), tail.find(')')) {
                (Some(a), Some(b)) if b > a => Some((a, b)),
                _ => None,
            };
            let (a, b) = match span {
                Some(s) => s,
                None => continue,
            };
            let url = tail[a + 1..b].to_string();
            let after = &tail[b + 1..];
            if !after.contains("**Arch:**") {
                continue;
            }
            let body = after.trim_start_matches([' ', '-']).to_string();
            let description = body
                .split("**Blazing:**")
                .next()
                .unwrap_or("")
                .trim()
                .trim_end_matches('.')
                .to_string();
            let blazing = body
                .split("**Blazing:**")
                .nth(1)
                .unwrap_or("")
                .split("**Arch:**")
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            let arch = body
                .split("**Arch:**")
                .nth(1)
                .map(parse_arch_tags)
                .unwrap_or_default();
            model.entries.push(Entry { name, url, section: section.clone(), description, blazing, arch });
        }
    }
    model
}

/// PATTERNS.md -> ordered index rows (tag + used-by names).
pub fn parse_patterns(patterns: &str) -> Vec<PatternRow> {
    let mut rows = Vec::new();
    for line in patterns.lines() {
        if !(line.starts_with("| `") || line.starts_with("| [")) {
            continue;
        }
        let cells: Vec<&str> = line.trim_matches('|').split('|').map(str::trim).collect();
        if cells.len() != 2 {
            continue;
        }
        // strip markdown link wrapper and backticks from the pattern cell
        let tag = cells[0]
            .trim_start_matches(['[', '`'])
            .split(']')
            .next()
            .unwrap_or(cells[0])
            .trim_end_matches(['`', ')'])
            .to_string();
        let has_decision_link = cells[0].starts_with("[`");
        let used_by = cells[1].split(", ").map(str::to_string).filter(|s| !s.is_empty()).collect();
        rows.push(PatternRow { tag, has_decision_link, used_by });
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = "\
## Contents

- [Alpha](#alpha)

## Alpha

Intro line.

- [alpha](https://github.com/a/a) - does a thing. **Blazing:** 10x faster, 5 MB. **Arch:** `simd`, `lock-free`.
- [beta](https://github.com/b/b) - another thing. **Blazing:** maps well. **Arch:** `zero-copy`.

## Use

- [docs](https://example.com/docs) - see the docs.
";

    #[test]
    fn parses_entries_sections_and_tags() {
        let m = parse_readme(FIXTURE);
        assert_eq!(m.sections, vec!["Alpha".to_string(), "Use".to_string()]);
        assert_eq!(m.entries.len(), 2);
        let alpha = &m.entries[0];
        assert_eq!(alpha.arch, vec!["simd".to_string(), "lock-free".to_string()]);
        assert_eq!(alpha.blazing, "10x faster, 5 MB.");
        assert_eq!(m.entries[1].kind(), crate::domain::model::Kind::CaseStudy);
    }

    #[test]
    fn parses_pattern_rows() {
        let rows = parse_patterns("| [`cost-model-driven`](decisions/001.md) | bbtree, x |\n| `simd` | alpha |\n");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].tag, "cost-model-driven");
        assert!(rows[0].has_decision_link);
        assert_eq!(rows[1].tag, "simd");
        assert_eq!(rows[1].used_by, vec!["alpha".to_string()]);
    }
}
