//! Translation between the markdown documents and the domain model.
//!
//! These parsers are pure (text in, domain objects out) so they are unit
//! testable without any filesystem.

use crate::domain::model::{Entry, Kind, Model, PatternRow, PatternSpec, Provenance};

/// A tag token must be lowercase alnum or '-', otherwise it is prose that
/// leaked out of a backtick pair - never a tag.
fn is_tag_token(t: &str) -> bool {
    !t.is_empty()
        && t.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
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
            // Field runs end at the next " **" bold marker. The Blazing
            // header may carry a variant ("**Blazing (for trust):**") - an
            // unparsed variant must not empty the claim or smuggle markdown
            // into the description.
            fn field_end(s: &str) -> &str {
                match s.find(" **") {
                    Some(i) => &s[..i],
                    None => s,
                }
            }
            let description = match body.find("**") {
                Some(i) => body[..i].trim().trim_end_matches('.'),
                None => body.trim(),
            }
            .to_string();
            let blazing = body
                .find("**Blazing")
                .and_then(|i| body[i..].find(":**").map(|c| i + c + 3))
                .map(|start| field_end(&body[start..]).trim().to_string())
                .unwrap_or_default();
            let declared = body
                .find("**Type:**")
                .map(|i| {
                    field_end(&body[i + "**Type:**".len()..])
                        .trim()
                        .trim_end_matches('.')
                })
                .and_then(Kind::parse);
            let measured_by = body
                .find("**Measured by:**")
                .map(|i| {
                    field_end(&body[i + "**Measured by:**".len()..])
                        .trim()
                        .trim_end_matches('.')
                })
                .and_then(Provenance::parse);
            let arch = body
                .split("**Arch:**")
                .nth(1)
                .map(parse_arch_tags)
                .unwrap_or_default();
            model.entries.push(Entry {
                name,
                url,
                section: section.clone(),
                description,
                blazing,
                arch,
                declared,
                measured_by,
            });
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
        // Two columns is the classic layout; three adds the typed contract's
        // `Removes` column. Anything else is not an index row.
        if cells.len() != 2 && cells.len() != 3 {
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
        let used_by = cells[1]
            .split(", ")
            .map(str::to_string)
            .filter(|s| !s.is_empty())
            .collect();
        rows.push(PatternRow {
            tag,
            has_decision_link,
            used_by,
        });
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
        assert_eq!(
            alpha.arch,
            vec!["simd".to_string(), "lock-free".to_string()]
        );
        assert_eq!(alpha.blazing, "10x faster, 5 MB.");
        assert_eq!(m.entries[1].kind(), crate::domain::model::Kind::CaseStudy);
    }

    #[test]
    fn parses_pattern_rows() {
        let rows = parse_patterns(
            "| [`cost-model-driven`](decisions/001.md) | bbtree, x |\n| `simd` | alpha |\n",
        );
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].tag, "cost-model-driven");
        assert!(rows[0].has_decision_link);
        assert_eq!(rows[1].tag, "simd");
        assert_eq!(rows[1].used_by, vec!["alpha".to_string()]);
    }

    #[test]
    fn parses_type_and_blazing_variants() {
        let m = parse_readme(
            "## S\n\n- [trust](https://github.com/t/t) - proves things. **Blazing (for trust):** machine-checked proofs, no runtime cost. **Type:** tool. **Arch:** `simd`.\n- [dig](https://github.com/d/d) - prose with e2e, 0-RTT, O(1), p99, since 2026. **Blazing:** handles every tier. **Type:** case-study. **Arch:** `jit`.\n",
        );
        let t = &m.entries[0];
        // the trust variant must land in the claim field, not the description
        assert_eq!(t.blazing, "machine-checked proofs, no runtime cost.");
        assert!(!t.description.contains("**"));
        assert_eq!(t.kind(), crate::domain::model::Kind::Tool);
        // digits in prose must not flip the declared type
        let d = &m.entries[1];
        assert_eq!(d.kind(), crate::domain::model::Kind::CaseStudy);
        assert_eq!(d.blazing, "handles every tier.");
    }
}


/// `catalog/patterns.json` -> typed specs. A tolerant scan of the layout the
/// writer emits, the same contract style as the upstream projection.
pub fn parse_pattern_specs(raw: &str) -> Vec<PatternSpec> {
    let mut out: Vec<PatternSpec> = Vec::new();
    let mut current: Option<PatternSpec> = None;
    for line in raw.lines() {
        let t = line.trim();
        if t.starts_with('"') && t.contains("\": {") && !t.starts_with("\"meta\"") {
            if let Some(spec) = current.take() {
                out.push(spec);
            }
            // the tag is the text between the first and second quote
            let tag = t
                .split('"')
                .nth(1)
                .map(str::to_string)
                .unwrap_or_default();
            current = Some(PatternSpec { tag, ..Default::default() });
            continue;
        }
        if let Some(spec) = current.as_mut() {
            for (key, slot) in [("removes", 0), ("fails_when", 1)] {
                if let Some(rest) = t.strip_prefix(&format!("\"{key}\": \"")) {
                    let v = rest.trim_end_matches(',').trim_end_matches('"');
                    if slot == 0 { spec.removes = v.to_string(); } else { spec.fails_when = v.to_string(); }
                }
            }
        }
    }
    if let Some(spec) = current.take() {
        out.push(spec);
    }
    out
}
