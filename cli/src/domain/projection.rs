//! Deterministic projections: Model -> artifact text. No I/O.
//!
//! This is the "metaprompt" half of the design: one canonical model, and
//! every published artifact (catalog JSON, PATTERNS index, llms.txt) is a
//! pure function of it. Same model in, byte-identical artifacts out.

use super::model::Model;
use super::rules::DECISION_LINKS;

pub const PATTERNS_PATH: &str = "PATTERNS.md";
pub const CATALOG_PATH: &str = "catalog/projects.json";
pub const LLMS_PATH: &str = "llms.txt";
const PATTERNS_TAIL_MARKER: &str = "\n---\n\n## Pattern Notes";

#[derive(Debug, Clone)]
pub struct Artifact {
    pub path: &'static str,
    pub content: String,
}

fn json_escape(s: &str) -> String {
    super::json::escape(s)
}

/// catalog/projects.json - the machine-readable (agent-facing) projection.
pub fn catalog(model: &Model) -> Artifact {
    use super::json::Value;
    let meta = Value::obj(vec![
        ("format", Value::Str("awesome-blazing-rust catalog v1".into())),
        (
            "rule",
            Value::Str(
                "no numbers, no entry - measured entries carry claim+source; others are case-studies or tools"
                    .into(),
            ),
        ),
        ("types", Value::strs(&["measured", "tool", "case-study", "agent-skill"])),
        ("maintainer", Value::Str("rager306".into())),
    ]);
    let projects: Vec<Value> = model
        .entries
        .iter()
        .map(|e| {
            let mut fields = vec![
                ("name", Value::Str(e.name.clone())),
                ("url", Value::Str(e.url.clone())),
                ("section", Value::Str(e.section.clone())),
                ("type", Value::Str(e.kind().as_str().to_string())),
            ];
            if let Some(p) = e.measured_by {
                fields.push(("measured_by", Value::Str(p.as_str().to_string())));
            }
            fields.push(("description", Value::Str(e.description.clone())));
            fields.push(("blazing", Value::Str(e.blazing.clone())));
            fields.push(("arch", Value::strs(&e.arch)));
            Value::obj(fields)
        })
        .collect();
    let doc = Value::obj(vec![("meta", meta), ("projects", Value::Block(projects))]);
    Artifact {
        path: CATALOG_PATH,
        content: format!("{}\n", doc.render()),
    }
}

fn index_table(model: &Model) -> String {
    let mut rows = Vec::new();
    for (tag, names) in model.usage() {
        let label = match DECISION_LINKS.iter().find(|(t, _)| *t == tag) {
            Some((_, path)) => format!("[`{tag}`]({path})"),
            None => format!("`{tag}`"),
        };
        rows.push(format!("| {label} | {} |", names.join(", ")));
    }
    let head = "# Architecture Patterns Index\n\nThe second axis of this list: **how** the speed is achieved. Patterns with a link have a full write-up in [decisions/](decisions/) - problem, technique, evidence, trade-offs. New entries must only use tags from this table (add a row via PR).\n\n| Pattern | Used by |\n| --- | --- |\n";
    format!("{head}{}\n", rows.join("\n"))
}

/// PATTERNS.md - regenerate the index table, preserving the hand-written
/// notes tail below the marker.
pub fn patterns_doc(model: &Model, current: &str) -> Result<Artifact, String> {
    let tail_pos = current.find(PATTERNS_TAIL_MARKER).ok_or_else(|| {
        format!(
            "PATTERNS.md missing notes marker ({PATTERNS_TAIL_MARKER}) - refusing to regenerate"
        )
    })?;
    let tail = &current[tail_pos..];
    Ok(Artifact {
        path: PATTERNS_PATH,
        content: format!("{}{}", index_table(model), tail),
    })
}

/// llms.txt - the LLM-facing entry point (same spirit as llms.txt on sites).
pub fn llms(model: &Model) -> Artifact {
    let mut out = String::from("# awesome-blazing-rust\n\n> Rust projects that prove their speed. Every entry ships measured numbers; every trick is named. Rule: no numbers, no entry.\n\n");
    out.push_str(&format!(
        "## How to consume this repository\n\n- README.md - the catalog itself ({} entries across sections, `Blazing:` = evidence, `Arch:` = pattern tags).\n- catalog/projects.json - machine-readable index: name, url, section, type (measured|tool|case-study|agent-skill), description, claim, arch tags. Best entry point for agents.\n- PATTERNS.md - architecture-patterns index: which technique achieves speed, used by which projects.\n- decisions/ - ADR-style write-ups per pattern: problem, technique, evidence, trade-offs.\n- recipes/ - symptom-driven guides: from a developer problem to patterns, projects and their limits.\n- CONTRIBUTING.md - entry rules: types, measurement passport, evidence requirements.\n- METHODOLOGY.md - evidence comparability rules.\n- docs/agents/ - the system prompts of the repository's own agents (discovery, intake, curator): house rules, the instruction-versus-data boundary, and the ban on running a research subject's code.\n- runs/ - replication artifacts and experiments; a `catalog` claim links one.\n- catalog/signals.json - third-party context (adoption, advisories, disputes). Data, not evidence: it can hold or demote a card, never raise its Type.\n\n## Sections\n\n",
        model.entries.len()
    ));
    for s in &model.sections {
        out.push_str(&format!(
            "- {} ({} entries)\n",
            s,
            model.count_in_section(s)
        ));
    }
    out.push_str("\n## For agents\n\n- Use catalog/projects.json for structured access; the README is prose-first.\n- Card types are explicit (**Type:** in the card, the type field here): measured = numbers plus a named baseline required; tool and case-study = mechanism claims, numbers optional; agent-skill = points at a skill file. A measured card without numbers is a bug; a tool card without numbers is normal.\n- Tags are defined in PATTERNS.md; unknown tags in PRs must add a row.\n");
    Artifact {
        path: LLMS_PATH,
        content: out,
    }
}

#[cfg(test)]
mod tests {
    use super::{catalog, patterns_doc};
    use crate::domain::model::{Entry, Model};

    fn model() -> Model {
        Model {
            sections: vec!["S".into()],
            entries: vec![Entry {
                declared: None,
                measured_by: None,
                name: "x".into(),
                url: "https://e.com".into(),
                section: "S".into(),
                description: "d".into(),
                blazing: "10x".into(),
                arch: vec!["cost-model-driven".into()],
            }],
        }
    }

    #[test]
    fn catalog_is_deterministic() {
        assert_eq!(catalog(&model()).content, catalog(&model()).content);
    }

    #[test]
    fn patterns_preserves_tail_and_links_decisions() {
        let cur = "OLD\n---\n\n## Pattern Notes\n\nkeep me";
        let doc = patterns_doc(&model(), cur).unwrap();
        assert!(doc
            .content
            .contains("[`cost-model-driven`](decisions/001-cost-model-driven.md)"));
        assert!(doc.content.ends_with("keep me"));
    }

    #[test]
    fn patterns_refuses_without_marker() {
        assert!(patterns_doc(&model(), "no marker").is_err());
    }
}
