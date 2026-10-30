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
    let args: Vec<String> = std::env::args().collect();
    let arg = args.get(1).cloned().unwrap_or_else(|| {
        eprintln!("usage: abr <catalog|patterns|llms|check|stats|upstream|signals|verify>");
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
    } else if cmd == Cmd::Verify {
        let mut opts = app::VerifyOpts::default();
        let mut i = 2;
        while i < args.len() {
            match args[i].as_str() {
                "--limit" => {
                    i += 1;
                    opts.limit = args.get(i).and_then(|v| v.parse().ok()).unwrap_or(0);
                }
                "--only" => {
                    i += 1;
                    opts.only = args.get(i).cloned();
                }
                "--entail" => opts.entail = true,
                other => {
                    eprintln!("abr verify: unknown option '{other}'");
                    exit(2);
                }
            }
            i += 1;
        }
        app::run_verify(
            &io,
            &adapters::page::WebPage,
            &adapters::subq::Subq::from_env(),
            &opts,
        )
    } else if cmd == Cmd::Signals {
        // Signals reach the network through three ports, so the command takes
        // options of its own: --triage, --limit N, --only owner/name.
        let mut opts = app::SignalOpts::default();
        let mut i = 2;
        while i < args.len() {
            match args[i].as_str() {
                "--triage" => opts.triage = true,
                "--limit" => {
                    i += 1;
                    opts.limit = args.get(i).and_then(|v| v.parse().ok()).unwrap_or(0);
                }
                "--only" => {
                    i += 1;
                    opts.only = args.get(i).cloned();
                }
                "--eval" => {
                    i += 1;
                    opts.eval = args.get(i).cloned();
                }
                other => {
                    eprintln!("abr signals: unknown option '{other}'");
                    exit(2);
                }
            }
            i += 1;
        }
        let source = adapters::web::WebSignals::new();
        app::run_signals(&io, &source, &adapters::subq::Subq::from_env(), &opts)
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
