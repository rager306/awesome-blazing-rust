//! Pure data model. The whole CLI is a set of views over `Model`.

use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Measured,
    Tool,
    CaseStudy,
    AgentSkill,
}

impl Kind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Kind::Measured => "measured",
            Kind::Tool => "tool",
            Kind::CaseStudy => "case-study",
            Kind::AgentSkill => "agent-skill",
        }
    }

    pub fn parse(s: &str) -> Option<Kind> {
        match s.trim() {
            "measured" => Some(Kind::Measured),
            "tool" => Some(Kind::Tool),
            "case-study" => Some(Kind::CaseStudy),
            "agent-skill" => Some(Kind::AgentSkill),
            _ => None,
        }
    }

    /// The exact spellings the `**Type:**` marker accepts.
    pub const VALID: &'static str = "measured | tool | case-study | agent-skill";
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub name: String,
    pub url: String,
    pub section: String,
    pub description: String,
    pub blazing: String,
    pub arch: Vec<String>,
    /// Explicit `**Type:**` marker from the card. Type is DATA, never
    /// guessed from prose: "e2e", "0-RTT", "O(1)", "p99" and years all
    /// contain digits, and a prose heuristic silently reclassifies a card
    /// when its claim text changes (an emptied claim would flip the type
    /// instead of raising an error).
    pub declared: Option<Kind>,
}

impl Entry {
    /// Cards without a marker fall back to case-study; `check` raises
    /// MISSING_TYPE so the gap cannot pass silently.
    pub fn kind(&self) -> Kind {
        self.declared.unwrap_or(Kind::CaseStudy)
    }
}

/// The canonical source: README's semantic projection. Everything the CLI
/// emits or validates derives from this - never from parsing twice.
#[derive(Debug, Default)]
pub struct Model {
    pub sections: Vec<String>,
    pub entries: Vec<Entry>,
}

impl Model {
    /// tag -> entry names (deduplicated, sorted by first appearance).
    pub fn usage(&self) -> BTreeMap<&str, Vec<&str>> {
        let mut map: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for e in &self.entries {
            for t in &e.arch {
                let slot = map.entry(t.as_str()).or_default();
                if !slot.contains(&e.name.as_str()) {
                    slot.push(e.name.as_str());
                }
            }
        }
        map
    }

    pub fn count_in_section(&self, section: &str) -> usize {
        self.entries.iter().filter(|e| e.section == section).count()
    }
}

/// One row of the PATTERNS.md index, parsed into domain terms.
#[derive(Debug)]
pub struct PatternRow {
    pub tag: String,
    #[allow(dead_code)] // semantic fact, consumed by future rules
    pub has_decision_link: bool,
    pub used_by: Vec<String>,
}

#[derive(Debug)]
pub struct Violation {
    pub code: &'static str,
    pub message: String,
}
