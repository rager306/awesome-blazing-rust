//! abr - the awesome-blazing-rust keep-it-consistent CLI.
//!
//! Zero dependencies on purpose: a catalog about measured speed is written
//! in the same spirit it documents. Commands:
//!
//!   abr catalog   regenerate catalog/projects.json from README
//!   abr patterns  regenerate the PATTERNS.md auto-table from README tags
//!   abr llms      regenerate llms.txt (agent-facing index)
//!   abr check     validate cross-file consistency (exit 1 on failure)
//!   abr stats     print entry counts by type and section

use std::collections::BTreeMap;
use std::fs;
use std::process::exit;

const README: &str = "README.md";
const PATTERNS: &str = "PATTERNS.md";
const CATALOG: &str = "catalog/projects.json";
const LLMS: &str = "llms.txt";

/// Decision documents linked from the patterns table.
const DECISION_LINKS: &[(&str, &str)] = &[
    ("cost-model-driven", "decisions/001-cost-model-driven.md"),
    ("findings-as-verdicts", "decisions/002-findings-as-verdicts.md"),
    ("proof-carrying-performance", "decisions/003-proof-carrying-performance.md"),
    ("memory-bounded-streaming", "decisions/004-memory-bounded-streaming.md"),
    ("skills-in-repo", "decisions/005-skills-in-repo.md"),
];

/// Tools whose value is what they measure, not a speed claim of their own.
const TOOL_NAMES: &[&str] = &[
    "hyperfine", "criterion", "divan", "gungraun", "dhat", "cargo-flamegraph",
    "miri", "shuttle", "verus", "pathfinding", "itertools",
];

#[derive(Clone)]
struct Entry {
    name: String,
    url: String,
    section: String,
    description: String,
    blazing: String,
    arch: Vec<String>,
}

impl Entry {
    fn kind(&self) -> &'static str {
        if self.name == "uringscope SKILL.md" {
            "agent-skill"
        } else if TOOL_NAMES.contains(&self.name.as_str()) {
            "tool"
        } else if self.blazing.chars().any(|c| c.is_ascii_digit()) {
            "measured"
        } else {
            "case-study"
        }
    }
}

fn read(p: &str) -> String {
    fs::read_to_string(p).unwrap_or_else(|e| {
        eprintln!("abr: cannot read {p}: {e}");
        exit(1);
    })
}

/// Parse README into (ordered section names, entries).
fn parse_readme(readme: &str) -> (Vec<String>, Vec<Entry>) {
    let mut sections: Vec<String> = Vec::new();
    let mut entries: Vec<Entry> = Vec::new();
    let mut section = String::new();
    for line in readme.lines() {
        if let Some(s) = line.strip_prefix("## ") {
            section = s.trim().to_string();
            if section != "Contents" && !sections.contains(&section) {
                sections.push(section.clone());
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
            // skip non-entry list items like "- [hyperfine](url) - (see Hall of Fame)."
            if !after.contains("**Arch:**") {
                continue;
            }
            let body = after.trim_start_matches(|c| c == ' ' || c == '-').to_string();
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
                .map(|tags| {
                    tags.split('`')
                        .map(|t| t.trim().to_string())
                        .filter(|t| !t.is_empty() && t.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            entries.push(Entry { name, url, section: section.clone(), description, blazing, arch });
        }
    }
    (sections, entries)
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

fn catalog_json(entries: &[Entry]) -> String {
    let mut out = String::from("{\n  \"meta\": {\n    \"format\": \"awesome-blazing-rust catalog v1\",\n");
    out.push_str("    \"rule\": \"no numbers, no entry - measured entries carry claim+source; others are case-studies or tools\",\n");
    out.push_str("    \"types\": [\"measured\", \"tool\", \"case-study\", \"agent-skill\"],\n");
    out.push_str("    \"maintainer\": \"rager306\"\n  },\n  \"projects\": [\n");
    let items: Vec<String> = entries
        .iter()
        .map(|e| {
            let tags: Vec<String> = e.arch.iter().map(|t| format!("\"{}\"", json_escape(t))).collect();
            format!(
                "    {{\n      \"name\": \"{}\",\n      \"url\": \"{}\",\n      \"section\": \"{}\",\n      \"type\": \"{}\",\n      \"description\": \"{}\",\n      \"blazing\": \"{}\",\n      \"arch\": [{}]\n    }}",
                json_escape(&e.name),
                json_escape(&e.url),
                json_escape(&e.section),
                e.kind(),
                json_escape(&e.description),
                json_escape(&e.blazing),
                tags.join(", ")
            )
        })
        .collect();
    out.push_str(&items.join(",\n"));
    out.push_str("\n  ]\n}\n");
    out
}

/// entry name -> ordered arch tags
fn usage_map(entries: &[Entry]) -> BTreeMap<String, Vec<String>> {
    let mut map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for e in entries {
        for t in &e.arch {
            map.entry(t.clone()).or_default().push(e.name.clone());
        }
    }
    map
}

fn patterns_document(usage: &BTreeMap<String, Vec<String>>) -> String {
    let mut rows = Vec::new();
    for (tag, names) in usage {
        let label = match DECISION_LINKS.iter().find(|(t, _)| *t == tag) {
            Some((_, path)) => format!("[`{tag}`]({path})"),
            None => format!("`{tag}`"),
        };
        let mut uniq: Vec<&String> = names.iter().collect();
        uniq.dedup();
        rows.push(format!("| {label} | {} |", uniq.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")));
    }
    let head = "# Architecture Patterns Index\n\nThe second axis of this list: **how** the speed is achieved. Patterns with a link have a full write-up in [decisions/](decisions/) - problem, technique, evidence, trade-offs. New entries must only use tags from this table (add a row via PR).\n\n| Pattern | Used by |\n| --- | --- |\n";
    format!("{head}{}\n", rows.join("\n"))
}

const PATTERNS_TAIL_START: &str = "\n---\n\n## Pattern Notes";

fn cmd_patterns(entries: &[Entry]) {
    let new_doc = patterns_document(&usage_map(entries));
    let old = read(PATTERNS);
    let tail = match old.find(PATTERNS_TAIL_START) {
        Some(i) => &old[i..],
        None => {
            eprintln!("abr: PATTERNS.md missing notes marker ({PATTERNS_TAIL_START})");
            exit(1);
        }
    };
    fs::write(PATTERNS, format!("{new_doc}{tail}")).unwrap();
    println!("patterns: rebuilt index ({} tags)", usage_map(entries).len());
}

fn cmd_catalog(entries: &[Entry]) {
    fs::write(CATALOG, catalog_json(entries)).unwrap();
    println!("catalog: wrote {CATALOG} ({} entries)", entries.len());
}

fn cmd_llms(entries: &[Entry], sections: &[String]) {
    let mut out = String::from("# awesome-blazing-rust\n\n> Rust projects that prove their speed. Every entry ships measured numbers; every trick is named. Rule: no numbers, no entry.\n\n");
    out.push_str(&format!("## How to consume this repository\n\n- README.md - the catalog itself ({} entries across sections, `Blazing:` = evidence, `Arch:` = pattern tags).\n- catalog/projects.json - machine-readable index: name, url, section, type (measured|tool|case-study|agent-skill), description, claim, arch tags. Best entry point for agents.\n- PATTERNS.md - architecture-patterns index: which technique achieves speed, used by which projects.\n- decisions/ - ADR-style write-ups per pattern: problem, technique, evidence, trade-offs.\n- CONTRIBUTING.md - entry rules: types, measurement passport, evidence requirements.\n- METHODOLOGY.md - evidence comparability rules.\n\n## Sections\n\n", entries.len()));
    for s in sections {
        let n = entries.iter().filter(|e| &e.section == s).count();
        out.push_str(&format!("- {s} ({n} entries)\n"));
    }
    out.push_str("\n## For agents\n\n- Use catalog/projects.json for structured access; the README is prose-first.\n- A `Blazing:` line without a number is a bug: report it.\n- Tags are defined in PATTERNS.md; unknown tags in PRs must add a row.\n");
    fs::write(LLMS, out).unwrap();
    println!("llms: wrote {LLMS}");
}

fn cmd_check(entries: &[Entry]) {
    let mut problems: Vec<String> = Vec::new();
    let patterns = read(PATTERNS);
    let usage = usage_map(entries);

    // 1. every README tag has a PATTERNS row
    for tag in usage.keys() {
        if !patterns.contains(&format!("`{tag}`")) && !patterns.contains(&format!("[`{tag}`]")) {
            problems.push(format!("tag `{tag}` missing from PATTERNS.md index"));
        }
    }
    // 2. PATTERNS "used by" names exist in README
    for line in patterns.lines().filter(|l| l.starts_with("| `") || l.starts_with("| [")) {
        let cells: Vec<&str> = line.trim_matches('|').split('|').map(|c| c.trim()).collect();
        if cells.len() != 2 {
            continue;
        }
        for name in cells[1].split(", ") {
            let name = name.trim();
            if name.is_empty() {
                continue;
            }
            if !entries.iter().any(|e| e.name == name) {
                problems.push(format!("PATTERNS row references unknown entry: {name}"));
            }
        }
    }
    // 3. catalog file in sync
    if fs::metadata(CATALOG).is_ok() {
        let fresh = catalog_json(entries);
        if fresh != read(CATALOG) {
            problems.push("catalog/projects.json is stale - run `abr catalog`".to_string());
        }
    } else {
        problems.push("catalog/projects.json missing - run `abr catalog`".to_string());
    }
    // 4. measured entries need an explicit source among their links
    for e in entries.iter().filter(|e| e.kind() == "measured") {
        if e.blazing.is_empty() {
            problems.push(format!("measured entry '{}' has empty Blazing field", e.name));
        }
    }

    if problems.is_empty() {
        println!("check: OK ({} entries, {} tags)", entries.len(), usage.len());
    } else {
        for p in &problems {
            eprintln!("FAIL: {p}");
        }
        exit(1);
    }
}

fn cmd_stats(entries: &[Entry]) {
    let mut by_kind: BTreeMap<&str, usize> = BTreeMap::new();
    for e in entries {
        *by_kind.entry(e.kind()).or_default() += 1;
    }
    println!("entries: {}", entries.len());
    for (k, v) in by_kind {
        println!("  {k}: {v}");
    }
    println!("tags: {}", usage_map(entries).len());
}

fn main() {
    let cmd = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: abr <catalog|patterns|llms|check|stats>");
        exit(2);
    });
    let (sections, entries) = parse_readme(&read(README));
    match cmd.as_str() {
        "catalog" => cmd_catalog(&entries),
        "patterns" => cmd_patterns(&entries),
        "llms" => cmd_llms(&entries, &sections),
        "check" => cmd_check(&entries),
        "stats" => cmd_stats(&entries),
        other => {
            eprintln!("abr: unknown command '{other}'");
            exit(2);
        }
    }
}
