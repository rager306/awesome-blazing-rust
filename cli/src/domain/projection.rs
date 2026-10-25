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
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// catalog/projects.json - the machine-readable (agent-facing) projection.
pub fn catalog(model: &Model) -> Artifact {
    let mut out = String::from(
        "{\n  \"meta\": {\n    \"format\": \"awesome-blazing-rust catalog v1\",\n    \"rule\": \"no numbers, no entry - measured entries carry claim+source; others are case-studies or tools\",\n    \"types\": [\"measured\", \"tool\", \"case-study\", \"agent-skill\"],\n    \"maintainer\": \"rager306\"\n  },\n  \"projects\": [\n",
    );
    let items: Vec<String> = model
        .entries
        .iter()
        .map(|e| {
            let tags: Vec<String> = e.arch.iter().map(|t| format!("\"{}\"", json_escape(t))).collect();
            format!(
                "    {{\n      \"name\": \"{}\",\n      \"url\": \"{}\",\n      \"section\": \"{}\",\n      \"type\": \"{}\",\n      \"description\": \"{}\",\n      \"blazing\": \"{}\",\n      \"arch\": [{}]\n    }}",
                json_escape(&e.name),
                json_escape(&e.url),
                json_escape(&e.section),
                e.kind().as_str(),
                json_escape(&e.description),
                json_escape(&e.blazing),
                tags.join(", ")
            )
        })
        .collect();
    out.push_str(&items.join(",\n"));
    out.push_str("\n  ]\n}\n");
    Artifact { path: CATALOG_PATH, content: out }
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
        format!("PATTERNS.md missing notes marker ({PATTERNS_TAIL_MARKER}) - refusing to regenerate")
    })?;
    let tail = &current[tail_pos..];
    Ok(Artifact { path: PATTERNS_PATH, content: format!("{}{}", index_table(model), tail) })
}

/// llms.txt - the LLM-facing entry point (same spirit as llms.txt on sites).
pub fn llms(model: &Model) -> Artifact {
    let mut out = String::from("# awesome-blazing-rust\n\n> Rust projects that prove their speed. Every entry ships measured numbers; every trick is named. Rule: no numbers, no entry.\n\n");
    out.push_str(&format!(
        "## How to consume this repository\n\n- README.md - the catalog itself ({} entries across sections, `Blazing:` = evidence, `Arch:` = pattern tags).\n- catalog/projects.json - machine-readable index: name, url, section, type (measured|tool|case-study|agent-skill), description, claim, arch tags. Best entry point for agents.\n- PATTERNS.md - architecture-patterns index: which technique achieves speed, used by which projects.\n- decisions/ - ADR-style write-ups per pattern: problem, technique, evidence, trade-offs.\n- CONTRIBUTING.md - entry rules: types, measurement passport, evidence requirements.\n- METHODOLOGY.md - evidence comparability rules.\n\n## Sections\n\n",
        model.entries.len()
    ));
    for s in &model.sections {
        out.push_str(&format!("- {} ({} entries)\n", s, model.count_in_section(s)));
    }
    out.push_str("\n## For agents\n\n- Use catalog/projects.json for structured access; the README is prose-first.\n- A `Blazing:` line without a number is a bug: report it.\n- Tags are defined in PATTERNS.md; unknown tags in PRs must add a row.\n");
    Artifact { path: LLMS_PATH, content: out }
}

#[cfg(test)]
mod tests {
    use super::{catalog, patterns_doc};
    use crate::domain::model::{Entry, Model};

    fn model() -> Model {
        Model {
            sections: vec!["S".into()],
            entries: vec![Entry {
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
        assert!(doc.content.contains("[`cost-model-driven`](decisions/001-cost-model-driven.md)"));
        assert!(doc.content.ends_with("keep me"));
    }

    #[test]
    fn patterns_refuses_without_marker() {
        assert!(patterns_doc(&model(), "no marker").is_err());
    }
}
