//! The FSM: every pipeline step is a total, pure transition between states.
//!
//!   Boot -> Loaded -> Checked -> Planned -> Done
//!                                  \\-> Failed
//!
//! Transitions never perform I/O; the driver (app::run) supplies inputs and
//! commits artifacts through ports. A transition is only legal from the state
//! it takes as its argument, which the type system enforces by construction.

use super::model::{Kind, Model, PatternRow, Violation};
use super::projection::Artifact;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cmd {
    Catalog,
    Patterns,
    Llms,
    Check,
    Stats,
    Upstream,
    Signals,
    Verify,
}

impl Cmd {
    pub fn from_arg(s: &str) -> Option<Cmd> {
        Some(match s {
            "catalog" => Cmd::Catalog,
            "patterns" => Cmd::Patterns,
            "llms" => Cmd::Llms,
            "check" => Cmd::Check,
            "stats" => Cmd::Stats,
            "upstream" => Cmd::Upstream,
            "signals" => Cmd::Signals,
            "verify" => Cmd::Verify,
            _ => return None,
        })
    }
}

/// Everything the FSM needs, already translated into domain terms by adapters.
pub struct Workspace {
    pub model: Model,
    pub patterns_index: Vec<PatternRow>,
    pub patterns_raw: String,
    pub catalog_raw: Option<String>,
    pub upstream_raw: Option<String>,
    pub signals_raw: Option<String>,
    pub pattern_specs: Vec<super::model::PatternSpec>,
}

#[derive(Debug)]
pub struct Report {
    pub ok: bool,
    pub summary: String,
    pub failures: Vec<String>,
    #[allow(dead_code)] // part of the report contract for future sinks
    pub artifacts_written: usize,
}

pub enum State {
    Boot,
    Loaded(Workspace),
    Checked(Workspace, Vec<Violation>),
    Planned(Workspace, Vec<Artifact>),
    Done(Report),
    Failed(Report),
}

/// Loaded -> Checked: run all domain invariants.
pub fn run_checks(ws: Workspace) -> State {
    let fresh = super::projection::catalog(&ws.model).content;
    let mut violations = super::rules::check_with_specs(
        &ws.model,
        &ws.patterns_index,
        ws.catalog_raw.as_deref(),
        &fresh,
        &ws.pattern_specs,
    );
    // Signals are data about the project, never evidence about its speed: they
    // can only hold or demote a card (see rules::check_signals).
    if let Some(raw) = ws.signals_raw.as_deref() {
        let signals = super::rules::parse_signals(raw);
        violations.extend(super::rules::check_signals(&ws.model, &signals));
    }
    State::Checked(ws, violations)
}

/// Checked -> Planned: projections for generator commands.
/// `patterns` needs the raw current file (tail preservation), so it may fail.
pub fn plan(cmd: Cmd, ws: Workspace) -> Result<State, String> {
    let artifacts = match cmd {
        Cmd::Catalog => vec![super::projection::catalog(&ws.model)],
        Cmd::Patterns => vec![super::projection::patterns_doc(
            &ws.model,
            &ws.patterns_raw,
        )?],
        Cmd::Llms => vec![super::projection::llms(&ws.model)],
        // Check/Stats are terminal observations; Upstream is handled in the
        // driver at Checked (the fetch is I/O and lives behind a port).
        Cmd::Check | Cmd::Stats | Cmd::Upstream | Cmd::Signals | Cmd::Verify => Vec::new(),
    };
    Ok(State::Planned(ws, artifacts))
}

pub fn stats_report(ws: &Workspace) -> Report {
    let mut by_kind: Vec<(String, usize)> = Vec::new();
    for k in [Kind::Measured, Kind::Tool, Kind::CaseStudy] {
        let n = ws.model.entries.iter().filter(|e| e.kind() == k).count();
        by_kind.push((k.as_str().to_string(), n));
    }
    let n_skill = ws
        .model
        .entries
        .iter()
        .filter(|e| e.kind() == super::model::Kind::AgentSkill)
        .count();
    by_kind.push(("agent-skill".to_string(), n_skill));
    let mut summary = format!("entries: {}\n", ws.model.entries.len());
    for (k, n) in &by_kind {
        summary.push_str(&format!("  {k}: {n}\n"));
    }
    summary.push_str(&format!("tags: {}", ws.model.usage().len()));
    Report {
        ok: true,
        summary,
        failures: Vec::new(),
        artifacts_written: 0,
    }
}

pub fn check_report(ws: &Workspace, violations: &[Violation]) -> Report {
    if violations.is_empty() {
        Report {
            ok: true,
            summary: format!(
                "check: OK ({} entries, {} tags)",
                ws.model.entries.len(),
                ws.model.usage().len()
            ),
            failures: Vec::new(),
            artifacts_written: 0,
        }
    } else {
        Report {
            ok: false,
            summary: "check: FAILED".to_string(),
            failures: violations
                .iter()
                .map(|v| format!("FAIL [{}]: {}", v.code, v.message))
                .collect(),
            artifacts_written: 0,
        }
    }
}
