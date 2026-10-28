//! Domain invariants: pure functions Model + documents -> violations.

use super::model::{Kind, Model, PatternRow, Provenance, Violation};
use std::collections::BTreeMap;

/// The closed vocabulary of signal flags. A free-form flag would let a
/// judgement call enter through the back door as data.
pub const SIGNAL_FLAGS: &[&str] = super::signals::ALL_FLAGS;

/// One row of catalog/signals.json, as far as the gate needs it.
#[derive(Debug, Default)]
pub struct SignalRow {
    pub repo: String,
    pub flags: Vec<String>,
    pub verdict: Option<String>,
}

/// Tolerant scan of the exact layout `scripts/signals/collect.py` writes
/// (json.dumps(indent=2)). Same contract style as the upstream projection: the
/// writer owns the format, the reader refuses everything else.
pub fn parse_signals(raw: &str) -> Vec<SignalRow> {
    let mut rows: Vec<SignalRow> = Vec::new();
    let mut in_flags = false;
    for line in raw.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("\"repo\": \"") {
            if let Some(name) = rest.split('"').next() {
                rows.push(SignalRow { repo: name.to_string(), flags: Vec::new(), verdict: None });
            }
            continue;
        }
        let Some(row) = rows.last_mut() else { continue };
        if in_flags {
            if t == "]," || t == "]" {
                in_flags = false;
            } else {
                let f = t.trim_end_matches(',').trim_matches('"');
                if !f.is_empty() {
                    row.flags.push(f.to_string());
                }
            }
            continue;
        }
        if let Some(rest) = t.strip_prefix("\"flags\": [") {
            let rest = rest.trim_end_matches(',').trim();
            if rest.is_empty() {
                in_flags = true; // the multi-line form opens here
            } else if !rest.starts_with(']') {
                for f in rest.trim_end_matches(']').split(',') {
                    let f = f.trim().trim_matches('"');
                    if !f.is_empty() {
                        row.flags.push(f.to_string());
                    }
                }
            }
            continue;
        }
        if let Some(rest) = t.strip_prefix("\"verdict\": \"") {
            if let Some(v) = rest.split('"').next() {
                row.verdict = Some(v.to_string());
            }
        }
    }
    rows
}

/// Signals may only hold or demote. A measured card carrying a contested
/// signal is an unresolved contradiction, not a card to merge.
pub fn check_signals(model: &Model, signals: &[SignalRow]) -> Vec<Violation> {
    let mut out = Vec::new();
    for s in signals {
        for f in &s.flags {
            if !SIGNAL_FLAGS.contains(&f.as_str()) {
                out.push(Violation {
                    code: "SIGNALS_UNKNOWN_FLAG",
                    message: format!("signal flag `{f}` on {} is not in the vocabulary", s.repo),
                });
            }
        }
        let contested =
            s.flags.iter().any(|f| f == "contested") || s.verdict.as_deref() == Some("contested");
        if !contested {
            continue;
        }
        for e in model.entries.iter().filter(|e| e.kind() == Kind::Measured) {
            if normalize_url(&e.url) == format!("https://github.com/{}", s.repo.to_lowercase()) {
                out.push(Violation {
                    code: "MEASURED_CONTESTED",
                    message: format!(
                        "measured entry '{}' has a contested signal - resolve the dispute or demote the card to case-study",
                        e.name
                    ),
                });
            }
        }
    }
    out
}

/// Patterns with a full ADR write-up.
pub const DECISION_LINKS: &[(&str, &str)] = &[
    ("cost-model-driven", "decisions/001-cost-model-driven.md"),
    ("findings-as-verdicts", "decisions/002-findings-as-verdicts.md"),
    ("proof-carrying-performance", "decisions/003-proof-carrying-performance.md"),
    ("memory-bounded-streaming", "decisions/004-memory-bounded-streaming.md"),
    ("skills-in-repo", "decisions/005-skills-in-repo.md"),
];

fn normalize_url(url: &str) -> String {
    let mut u = url.trim().to_lowercase();
    if let Some(s) = u.strip_suffix('/') {
        u = s.to_string();
    }
    if let Some(s) = u.strip_suffix(".git") {
        u = s.to_string();
    }
    u
}

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

    // 7. a measured claim names who measured it (METHODOLOGY §5): an author
    // counting their own sessions and an independent rerun are different
    // evidence, and the card must say which one it is.
    for e in &model.entries {
        if e.kind() == Kind::Measured && e.measured_by.is_none() {
            out.push(Violation {
                code: "MEASURED_NO_PROVENANCE",
                message: format!(
                    "measured entry '{}' has no **Measured by:** marker - one of: {}",
                    e.name,
                    Provenance::VALID
                ),
            });
        }
        if e.kind() != Kind::Measured && e.measured_by.is_some() {
            out.push(Violation {
                code: "PROVENANCE_ON_UNMEASURED",
                message: format!(
                    "entry '{}' declares **Measured by:** but is not measured",
                    e.name
                ),
            });
        }
        // A claim this catalog made itself must link the run that made it.
        if e.measured_by == Some(Provenance::Catalog) && !e.blazing.contains("runs/") {
            out.push(Violation {
                code: "CATALOG_RUN_UNLINKED",
                message: format!(
                    "entry '{}' is measured by the catalog but its claim links no runs/ artifact",
                    e.name
                ),
            });
        }
    }

    // 8. one entry per project (CONTRIBUTING): cross-posting the same project
    // into a second section is not allowed. Upper-cased by an author, with or
    // without a trailing slash or a .git suffix - same repository.
    let mut first_seen: BTreeMap<String, &str> = BTreeMap::new();
    for e in &model.entries {
        let key = normalize_url(&e.url);
        match first_seen.get(&key) {
            Some(first) => out.push(Violation {
                code: "DUPLICATE_ENTRY",
                message: format!("entry '{}' duplicates '{}' ({})", e.name, first, key),
            }),
            None => {
                first_seen.insert(key, &e.name);
            }
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
            measured_by: None,
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
    fn measured_needs_a_named_measurer() {
        let m = Model {
            sections: vec![],
            entries: vec![entry_typed("x", &[], "3x vs baseline", Some(Kind::Measured))],
        };
        let v = check(&m, &[], Some("x"), "x");
        assert!(v.iter().any(|x| x.code == "MEASURED_NO_PROVENANCE"));
    }

    #[test]
    fn provenance_on_a_case_study_is_a_violation() {
        let mut e = entry_typed("x", &[], "claim", Some(Kind::CaseStudy));
        e.measured_by = Some(Provenance::Author);
        let m = Model { sections: vec![], entries: vec![e] };
        let v = check(&m, &[], Some("x"), "x");
        assert!(v.iter().any(|x| x.code == "PROVENANCE_ON_UNMEASURED"));
    }

    #[test]
    fn a_catalog_claim_must_link_its_run() {
        let mut e = entry_typed("x", &[], "2.5 ms vs 1879 ms, see the run", Some(Kind::Measured));
        e.measured_by = Some(Provenance::Catalog);
        let m = Model { sections: vec![], entries: vec![e] };
        let v = check(&m, &[], Some("x"), "x");
        assert!(v.iter().any(|x| x.code == "CATALOG_RUN_UNLINKED"));

        let mut linked = entry_typed("x", &[], "see runs/x/RESULTS.md", Some(Kind::Measured));
        linked.measured_by = Some(Provenance::Catalog);
        let m = Model { sections: vec![], entries: vec![linked] };
        assert!(check(&m, &[], Some("x"), "x").iter().all(|v| v.code != "CATALOG_RUN_UNLINKED"));
    }

    #[test]
    fn duplicate_repo_url_is_a_violation() {
        let mk = |name: &str, url: &str| super::super::model::Entry {
            name: name.to_string(),
            url: url.to_string(),
            section: "S".to_string(),
            description: "d".to_string(),
            blazing: "10x vs baseline".to_string(),
            arch: vec![],
            declared: Some(Kind::CaseStudy),
                measured_by: None,
        };
        // same repository spelled differently: case, trailing slash, .git
        for (second_name, second_url) in [
            ("b", "https://github.com/apache/datafusion"),
            ("b", "https://github.com/Apache/datafusion/"),
            ("b", "https://github.com/apache/datafusion.git"),
        ] {
            let m = Model {
                sections: vec![],
                entries: vec![
                    mk("a", "https://github.com/apache/datafusion"),
                    mk(second_name, second_url),
                ],
            };
            let v = check(&m, &[], Some("x"), "x");
            assert!(
                v.iter().any(|x| x.code == "DUPLICATE_ENTRY"),
                "expected DUPLICATE_ENTRY for {second_url}"
            );
        }
    }

    #[test]
    fn distinct_repos_are_not_duplicates() {
        let mk = |name: &str, url: &str| super::super::model::Entry {
            name: name.to_string(),
            url: url.to_string(),
            section: "S".to_string(),
            description: "d".to_string(),
            blazing: "10x vs baseline".to_string(),
            arch: vec![],
            declared: Some(Kind::CaseStudy),
                measured_by: None,
        };
        let m = Model {
            sections: vec![],
            entries: vec![
                mk("a", "https://github.com/apache/datafusion"),
                mk("b", "https://github.com/apache/datafusion-cli"),
            ],
        };
        let v = check(&m, &[], Some("x"), "x");
        assert!(!v.iter().any(|x| x.code == "DUPLICATE_ENTRY"));
    }

    #[test]
    fn signals_parse_flags_and_verdict() {
        let raw = r#"{
  "meta": {"repos": 2},
  "repos": [
    {
      "repo": "a/b",
      "flags": [
        "security",
        "unmaintained"
      ],
      "verdict": "contested"
    },
    {
      "repo": "c/d",
      "flags": []
    }
  ]
}"#;
        let rows = parse_signals(raw);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].repo, "a/b");
        assert_eq!(rows[0].flags, vec!["security".to_string(), "unmaintained".to_string()]);
        assert_eq!(rows[0].verdict.as_deref(), Some("contested"));
        assert!(rows[1].flags.is_empty());
        assert!(rows[1].verdict.is_none());
    }

    #[test]
    fn contested_signal_blocks_a_measured_card() {
        let m = Model {
            sections: vec![],
            entries: vec![super::super::model::Entry {
                name: "fast thing".into(),
                url: "https://github.com/Fast/thing".into(),
                section: "S".into(),
                description: "d".into(),
                blazing: "3x vs baseline".into(),
                arch: vec![],
                declared: Some(Kind::Measured),
                measured_by: None,
            }],
        };
        let signals = vec![SignalRow {
            repo: "fast/thing".into(),
            flags: vec![],
            verdict: Some("contested".into()),
        }];
        let v = check_signals(&m, &signals);
        assert!(v.iter().any(|x| x.code == "MEASURED_CONTESTED"));
    }

    #[test]
    fn contested_signal_does_not_block_a_case_study() {
        let m = Model {
            sections: vec![],
            entries: vec![entry_typed("thing", &[], "claim", Some(Kind::CaseStudy))],
        };
        let signals = vec![SignalRow {
            repo: "example.com".into(),
            flags: vec!["contested".into()],
            verdict: None,
        }];
        assert!(check_signals(&m, &signals).is_empty());
    }

    #[test]
    fn unknown_signal_flag_is_a_violation() {
        let m = Model { sections: vec![], entries: vec![] };
        let signals = vec![SignalRow {
            repo: "a/b".into(),
            flags: vec!["feels-hypey".into()],
            verdict: None,
        }];
        let v = check_signals(&m, &signals);
        assert!(v.iter().any(|x| x.code == "SIGNALS_UNKNOWN_FLAG"));
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
