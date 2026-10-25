//! Domain invariants: pure functions Model + documents -> violations.

use super::model::{Kind, Model, PatternRow, Violation};

/// Patterns with a full ADR write-up.
pub const DECISION_LINKS: &[(&str, &str)] = &[
    ("cost-model-driven", "decisions/001-cost-model-driven.md"),
    ("findings-as-verdicts", "decisions/002-findings-as-verdicts.md"),
    ("proof-carrying-performance", "decisions/003-proof-carrying-performance.md"),
    ("memory-bounded-streaming", "decisions/004-memory-bounded-streaming.md"),
    ("skills-in-repo", "decisions/005-skills-in-repo.md"),
];

/// Tools whose value is what they measure, not a speed claim of their own.
pub const TOOL_NAMES: &[&str] = &[
    "hyperfine",
    "criterion",
    "divan",
    "gungraun",
    "dhat",
    "cargo-flamegraph",
    "miri",
    "shuttle",
    "verus",
    "pathfinding",
    "itertools",
];

pub fn check(
    model: &Model,
    patterns: &[PatternRow],
    catalog_on_disk: Option<&str>,
    catalog_fresh: &str,
) -> Vec<Violation> {
    let mut out = Vec::new();
    let usage = model.usage();

    // 1. every README tag has a PATTERNS row
    for tag in usage.keys() {
        if !patterns.iter().any(|p| &p.tag == tag) {
            out.push(Violation {
                code: "TAG_NOT_INDEXED",
                message: format!("tag `{tag}` missing from PATTERNS.md index"),
            });
        }
    }

    // 2. PATTERNS "used by" names exist in README
    for p in patterns {
        for name in &p.used_by {
            if !model.entries.iter().any(|e| &e.name == name) {
                out.push(Violation {
                    code: "PATTERN_UNKNOWN_ENTRY",
                    message: format!("PATTERNS row references unknown entry: {name}"),
                });
            }
        }
    }

    // 3. catalog file in sync with the model
    match catalog_on_disk {
        None => out.push(Violation {
            code: "CATALOG_MISSING",
            message: "catalog/projects.json missing - run `abr catalog`".to_string(),
        }),
        Some(text) if text != catalog_fresh => out.push(Violation {
            code: "CATALOG_STALE",
            message: "catalog/projects.json is stale - run `abr catalog`".to_string(),
        }),
        _ => {}
    }

    // 4. measured entries need a non-empty Blazing (claim) field
    for e in model.entries.iter().filter(|e| e.kind() == Kind::Measured) {
        if e.blazing.is_empty() {
            out.push(Violation {
                code: "MEASURED_EMPTY_CLAIM",
                message: format!("measured entry '{}' has empty Blazing field", e.name),
            });
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, arch: &[&str], blazing: &str) -> super::super::model::Entry {
        super::super::model::Entry {
            name: name.to_string(),
            url: "https://example.com".to_string(),
            section: "S".to_string(),
            description: "d".to_string(),
            blazing: blazing.to_string(),
            arch: arch.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn missing_tag_is_violation() {
        let m = Model {
            sections: vec!["S".into()],
            entries: vec![entry("x", &["unindexed-tag"], "10x")],
        };
        let v = check(&m, &[], None, "{}");
        assert!(v.iter().any(|x| x.code == "TAG_NOT_INDEXED"));
        assert!(v.iter().any(|x| x.code == "CATALOG_MISSING"));
    }

    #[test]
    fn stale_catalog_detected() {
        let m = Model {
            sections: vec!["S".into()],
            entries: vec![entry("x", &["t"], "10x")],
        };
        let patterns = vec![PatternRow {
            tag: "t".into(),
            has_decision_link: false,
            used_by: vec!["x".into()],
        }];
        let v = check(&m, &patterns, Some("OLD"), "FRESH");
        assert!(v.iter().any(|x| x.code == "CATALOG_STALE"));
    }

    #[test]
    fn unknown_pattern_reference_detected() {
        let m = Model { sections: vec![], entries: vec![] };
        let patterns = vec![PatternRow {
            tag: "t".into(),
            has_decision_link: false,
            used_by: vec!["ghost".into()],
        }];
        let v = check(&m, &patterns, Some("x"), "x");
        assert!(v.iter().any(|x| x.code == "PATTERN_UNKNOWN_ENTRY"));
    }
}
