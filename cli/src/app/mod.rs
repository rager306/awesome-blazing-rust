//! Application layer: drives the FSM from Boot to a terminal state.
//!
//! The only place that combines ports + domain transitions. Every command is
//! the same machine with a different terminal branch:

use crate::adapters::markdown;
use crate::domain::fsm::{self, Cmd, Report, State, Workspace};
use crate::ports::sink::Sink;
use crate::ports::source::Source;

pub const README: &str = "README.md";
pub const PATTERNS: &str = "PATTERNS.md";
pub const CATALOG: &str = "catalog/projects.json";

pub fn run(cmd: Cmd, io: &(impl Source + Sink)) -> Report {
    let mut state = State::Boot;
    loop {
        state = match state {
            // Boot -> Loaded: adapters translate raw texts into domain terms.
            State::Boot => {
                let readme = io.read(README);
                let patterns_raw = io.read(PATTERNS);
                let catalog_raw = io.read_opt(CATALOG);
                let model = markdown::parse_readme(&readme);
                let patterns_index = markdown::parse_patterns(&patterns_raw);
                State::Loaded(Workspace { model, patterns_index, patterns_raw, catalog_raw })
            }
            // Loaded -> Checked: domain invariants run on every command.
            State::Loaded(ws) => fsm::run_checks(ws),
            State::Checked(ws, violations) => match cmd {
                Cmd::Check => {
                    let r = fsm::check_report(&ws, &violations);
                    if r.ok { State::Done(r) } else { State::Failed(r) }
                }
                Cmd::Stats => State::Done(fsm::stats_report(&ws)),
                _ => {
                    if !violations.is_empty() {
                        State::Failed(fsm::check_report(&ws, &violations))
                    } else {
                        // Checked -> Planned -> (committed by driver) -> Done
                        match fsm::plan(cmd, ws) {
                            Ok(s) => s,
                            Err(e) => State::Failed(Report {
                                ok: false,
                                summary: format!("abr: {e}"),
                                failures: vec![],
                                artifacts_written: 0,
                            }),
                        }
                    }
                }
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
