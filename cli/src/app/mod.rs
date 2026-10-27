//! Application layer: drives the FSM from Boot to a terminal state.
//!
//! The only place that combines ports + domain transitions. Every command is
//! the same machine with a different terminal branch:

use crate::adapters::markdown;
use crate::domain::fsm::{self, Cmd, Report, State, Workspace};
use crate::domain::upstream::{self, UpstreamRow};
use crate::ports::sink::Sink;
use crate::ports::source::Source;
use crate::ports::upstream::{TagState, UpstreamSource};

pub const README: &str = "README.md";
pub const PATTERNS: &str = "PATTERNS.md";
pub const CATALOG: &str = "catalog/projects.json";
pub const UPSTREAM: &str = "catalog/upstream.json";
pub const STRIKE: &str = "catalog/upstream-state.json";

pub fn run(cmd: Cmd, io: &(impl Source + Sink)) -> Report {
    let mut state = State::Boot;
    loop {
        state = match state {
            // Boot -> Loaded: adapters translate raw texts into domain terms.
            State::Boot => {
                let readme = io.read(README);
                let patterns_raw = io.read(PATTERNS);
                let catalog_raw = io.read_opt(CATALOG);
                let upstream_raw = io.read_opt(UPSTREAM);
                let model = markdown::parse_readme(&readme);
                let patterns_index = markdown::parse_patterns(&patterns_raw);
                State::Loaded(Workspace { model, patterns_index, patterns_raw, catalog_raw, upstream_raw })
            }
            // Loaded -> Checked: domain invariants run on every command.
            State::Loaded(ws) => fsm::run_checks(ws),
            State::Checked(ws, violations) => match cmd {
                Cmd::Check => {
                    let r = fsm::check_report(&ws, &violations);
                    if r.ok { State::Done(r) } else { State::Failed(r) }
                }
                Cmd::Stats => State::Done(fsm::stats_report(&ws)),
                // Generators are the fix path for violations (e.g. new tags
                // are "not indexed" until `patterns` regenerates the index),
                // so they run regardless; the gate lives in `check` (and CI).
                _ => match fsm::plan(cmd, ws) {
                    Ok(s) => s,
                    Err(e) => State::Failed(Report {
                        ok: false,
                        summary: format!("abr: {e}"),
                        failures: vec![],
                        artifacts_written: 0,
                    }),
                },
            },
            // Planned -> Done: commit artifacts through the Sink port.
            State::Planned(ws, artifacts) => {
                let n = artifacts.len();
                for a in &artifacts {
                    io.write(a.path, &a.content);
                }
                let summary = match cmd {
                    Cmd::Catalog => format!("catalog: wrote {CATALOG} ({} entries)", ws.model.entries.len()),
                    Cmd::Patterns => format!("patterns: rebuilt index ({} tags)", ws.model.usage().len()),
                    Cmd::Llms => "llms: wrote llms.txt".to_string(),
                    _ => String::new(),
                };
                State::Done(Report { ok: true, summary, failures: vec![], artifacts_written: n })
            }
            State::Done(r) | State::Failed(r) => return r,
        };
    }
}

/// Upstream scan: fetch latest tags through the port, diff against the
/// previous projection, commit the new one. Runs from Checked; the report
/// carries the changed-repo lines so the pipeline can act on them.
pub fn run_upstream(io: &(impl Source + Sink), upstream: &impl UpstreamSource) -> Report {
    // Reuse the same Boot -> Loaded -> Checked prefix for invariant parity.
    let readme = io.read(README);
    let patterns_raw = io.read(PATTERNS);
    let catalog_raw = io.read_opt(CATALOG);
    let upstream_raw = io.read_opt(UPSTREAM);
    let model = markdown::parse_readme(&readme);
    let patterns_index = markdown::parse_patterns(&patterns_raw);
    let ws = Workspace { model, patterns_index, patterns_raw, catalog_raw, upstream_raw };
    let checked = fsm::run_checks(ws);

    let ws = match checked {
        State::Checked(ws, _) => ws,
        _ => unreachable!("run_checks always returns Checked"),
    };

    let mut slugs: Vec<String> = ws.model.entries.iter().filter_map(|e| upstream::repo_slug(&e.url)).collect();
    slugs.sort();
    slugs.dedup();

    let mut rows: Vec<UpstreamRow> = Vec::new();
    let mut missing: Vec<String> = Vec::new();
    for slug in &slugs {
        match upstream.latest_tag(slug) {
            TagState::Tag(tag) => rows.push(UpstreamRow { repo: slug.clone(), tag }),
            // stable legitimate state - must not churn the weekly diff
            TagState::NoTags => rows.push(UpstreamRow { repo: slug.clone(), tag: "(no-tags)".into() }),
            TagState::Unreachable => missing.push(slug.clone()),
        }
    }

    let d = upstream::diff(ws.upstream_raw.as_deref(), &rows);

    // Strike ledger: consecutive failures per repo, reset on any success.
    let outcomes: Vec<(String, upstream::Outcome)> = slugs
        .iter()
        .map(|s| {
            let o = if missing.contains(s) {
                upstream::Outcome::Unreachable
            } else {
                upstream::Outcome::Ok
            };
            (s.clone(), o)
        })
        .collect();
    let strike_prev = upstream::parse_strike(&io.read_opt(STRIKE).unwrap_or_default());

    // Circuit breaker: a mass-failure run is an upstream incident - freeze
    // both the projection and the ledger so one bad day poisons nothing
    // (a half-empty projection would churn the next diff with fake churn).
    if upstream::incident(missing.len(), slugs.len()) {
        let mut failures = vec![format!(
            "INCIDENT {}/{} repos unreachable (> 3%) - projection and strike state frozen",
            missing.len(),
            slugs.len()
        )];
        failures.extend(missing.iter().map(|r| format!("UNREACHABLE {r}")));
        return Report {
            ok: true,
            summary: format!("upstream: scanned {} repos, incident - nothing written", slugs.len()),
            failures,
            artifacts_written: 0,
        };
    }

    let strike_new = upstream::apply(&strike_prev, &outcomes);
    io.write(UPSTREAM, &upstream::json(&rows));
    io.write(STRIKE, &upstream::strike_json(&strike_new));

    let mut bumped = 0;
    let mut recovered = 0;
    for (repo, n) in &strike_new.0 {
        match strike_prev.0.get(repo) {
            Some(old) if n > old => bumped += 1,
            Some(old) if n < old => recovered += 1,
            _ => {}
        }
    }

    let mut summary = format!(
        "upstream: scanned {} repos, {} changed, {} unfetchable; strikes +{bumped}/-{recovered}; wrote {UPSTREAM} + {STRIKE}",
        slugs.len(),
        d.changed.len(),
        missing.len()
    );
    let mut failures: Vec<String> = Vec::new();
    for line in &d.changed {
        summary.push_str(&format!("\nCHANGED {line}"));
    }
    for repo in &d.gone {
        failures.push(format!("GONE {repo} (was scanned, now unfetchable)"));
    }
    // Unfetchable repos are a soft condition: the scan still succeeded.
    for repo in upstream::tripwire(&strike_new) {
        failures.push(format!("TRIPWIRE {repo}: 3 strikes (enforcement is phase 2)"));
    }
    Report { ok: true, summary, failures, artifacts_written: 2 }
}
