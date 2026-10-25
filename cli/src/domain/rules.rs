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

    // 4. every card declares its type explicitly - no prose guessing
    for e in model.entries.iter().filter(|e| e.declared.is_none()) {
        out.push(Violation {
            code: "MISSING_TYPE",
            message: format!(
                "entry '{}' has no **Type:** marker - one of: {}",
                e.name,
                Kind::VALID
            ),
        });
    }

    // 5. measured entries need a non-empty Blazing (claim) field - now
    // reachable because the type is explicit, not digit-derived
    for e in model.entries.iter().filter(|e| e.kind() == Kind::Measured) {
        if e.blazing.is_empty() {
            out.push(Violation {
                code: "MEASURED_EMPTY_CLAIM",
                message: format!("measured entry '{}' has empty Blazing field", e.name),
            });
        } else if e.blazing.chars().any(|c| c.is_ascii_digit())
            && !["vs", "against", "baseline", "than", "above", "versus"]
                .iter()
                .any(|m| e.blazing.to_lowercase().contains(m))
        {
            out.push(Violation {
                code: "MEASURED_NO_BASELINE",
                message: format!(
                    "measured entry '{}' states a number but names no baseline (vs / against / baseline)",
                    e.name
                ),
            });
        }
    }

    // 6. agent-skill type and the Agent Skills section agree
    for e in &model.entries {
        let in_skills = e.section == "Agent Skills";
        if e.kind() == Kind::AgentSkill && !in_skills {
            out.push(Violation {
                code: "SKILL_OUTSIDE_SECTION",
                message: format!("agent-skill entry '{}' is outside the Agent Skills section", e.name),
            });
        }
        if in_skills && e.kind() != Kind::AgentSkill {
            out.push(Violation {
                code: "SKILL_SECTION_MISMATCH",
                message: format!(
                    "entry '{}' in Agent Skills must declare **Type:** agent-skill",
                    e.name
                ),
            });
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, arch: &[&str], blazing: &str) -> super::super::model::Entry {
        entry_typed(name, arch, blazing, None)
    }

    fn entry_typed(
        name: &str,
        arch: &[&str],
        blazing: &str,
        declared: Option<Kind>,
    ) -> super::super::model::Entry {
        super::super::model::Entry {
            name: name.to_string(),
            url: "https://example.com".to_string(),
            section: "S".to_string(),
            description: "d".to_string(),
            blazing: blazing.to_string(),
            arch: arch.iter().map(|s| s.to_string()).collect(),
            declared,
        }
    }

    // Negative tests for the type system: digits in prose, emptied claims
    // and missing markers must never silently reclassify a card.

    #[test]
    fn prose_digits_do_not_make_measured() {
        let e = entry_typed(
            "x",
            &[],
            "handles e2e, 0-RTT and p99 since 2026 in O(1)",
            Some(Kind::CaseStudy),
        );
        assert_eq!(e.kind(), Kind::CaseStudy);
    }

    #[test]
    fn empty_claim_on_measured_is_now_reachable() {
        let m = Model {
            sections: vec![],
            entries: vec![entry_typed("x", &[], "", Some(Kind::Measured))],
        };
        let v = check(&m, &[], None, "{}");
        assert!(v.iter().any(|x| x.code == "MEASURED_EMPTY_CLAIM"));
    }

    #[test]
    fn removing_claim_does_not_change_type() {
        // same card, claim removed: the type stays Measured (explicit) and
        // the rule fires instead of the card quietly becoming a case-study
        let m = Model {
            sections: vec![],
            entries: vec![entry_typed("x", &[], "", Some(Kind::Measured))],
        };
        let v = check(&m, &[], None, "{}");
        assert!(v.iter().any(|x| x.code == "MEASURED_EMPTY_CLAIM"));
        assert!(!v.iter().any(|x| x.code == "MISSING_TYPE"));
    }

    #[test]
    fn missing_type_marker_is_a_violation() {
        let m = Model {
            sections: vec![],
            entries: vec![entry("x", &[], "10x")],
        };
        let v = check(&m, &[], None, "{}");
        assert!(v.iter().any(|x| x.code == "MISSING_TYPE"));
    }

    #[test]
    fn measured_numbers_need_a_named_baseline() {
        let m = Model {
            sections: vec![],
            entries: vec![entry_typed("x", &[], "3x faster on tier two", Some(Kind::Measured))],
        };
        let v = check(&m, &[], None, "{}");
        assert!(v.iter().any(|x| x.code == "MEASURED_NO_BASELINE"));
    }

    #[test]
    fn skill_type_and_section_must_agree() {
        let m = Model {
            sections: vec![],
            entries: vec![entry_typed("skill x", &[], "", Some(Kind::AgentSkill))],
        };
        let v = check(&m, &[], None, "{}");
        assert!(v.iter().any(|x| x.code == "SKILL_OUTSIDE_SECTION"));
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
