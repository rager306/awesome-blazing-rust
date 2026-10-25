//! abr - the awesome-blazing-rust consistency CLI.
//!
//! Hexagonal/onion layout:
//!
//!   adapters/  - filesystem + markdown translation (edges)
//!   ports/     - Source / Sink traits (the hexagon's sides)
//!   domain/    - model, rules, FSM, projections (pure core, no I/O)
//!   app/       - FSM driver wiring ports to transitions
//!
//! The domain model is the single source of truth (the metaprompt): every
//! published artifact is a deterministic projection of it, and every command
//! is a path through the same finite state machine:
//!
//!   Boot -> Loaded -> Checked -> Planned -> Done | Failed
//!
//! Zero dependencies on purpose: a catalog about measured speed is written
//! in the same spirit it documents.

mod adapters;
mod app;
mod domain;
mod ports;

use adapters::fs_io::FsIo;
use domain::fsm::Cmd;
use std::process::exit;

fn main() {
    let arg = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: abr <catalog|patterns|llms|check|stats|upstream>");
        exit(2);
    });
    let cmd = match Cmd::from_arg(&arg) {
        Some(c) => c,
        None => {
            eprintln!("abr: unknown command '{arg}'");
            exit(2);
        }
    };
    let io = FsIo::discover();
    // Upstream needs the network: it bypasses the pure FSM terminal branches
    // and is never part of the CI determinism gate.
    let report = if cmd == Cmd::Upstream {
        app::run_upstream(&io, &adapters::gh_curl::GhCli)
    } else {
        app::run(cmd, &io)
    };
    println!("{}", report.summary);
    for f in &report.failures {
        eprintln!("{f}");
    }
    if !report.ok {
        exit(1);
    }
}
