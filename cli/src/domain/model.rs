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
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub name: String,
    pub url: String,
    pub section: String,
    pub description: String,
    pub blazing: String,
    pub arch: Vec<String>,
}

impl Entry {
    pub fn kind(&self) -> Kind {
        if self.name == "uringscope SKILL.md" {
            Kind::AgentSkill
        } else if super::rules::TOOL_NAMES.contains(&self.name.as_str()) {
            Kind::Tool
        } else if self.blazing.chars().any(|c| c.is_ascii_digit()) {
            Kind::Measured
        } else {
            Kind::CaseStudy
        }
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
