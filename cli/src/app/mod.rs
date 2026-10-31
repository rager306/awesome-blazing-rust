//! Application layer: drives the FSM from Boot to a terminal state.
//!
//! The only place that combines ports + domain transitions. Every command is
//! the same machine with a different terminal branch:

use crate::adapters::markdown;
use crate::domain::fsm::{self, Cmd, Report, State, Workspace};
use crate::domain::signals::{self, Bucket, Crate, Dispute, SignalRow};
use crate::domain::upstream::{self, UpstreamRow};
use crate::domain::verify;
use crate::ports::signals::DecisionSource;
use crate::ports::sink::Sink;
use crate::ports::source::Source;
use crate::ports::upstream::{TagState, UpstreamSource};

/// Read or stop. The decision to exit belongs to the application, not to the
/// adapter that meets the filesystem.
fn load(io: &impl Source, path: &str) -> String {
    io.read(path).unwrap_or_else(|e| {
        eprintln!("abr: {e}");
        std::process::exit(1);
    })
}

pub const README: &str = "README.md";
pub const PATTERNS: &str = "PATTERNS.md";
pub const CATALOG: &str = "catalog/projects.json";
pub const UPSTREAM: &str = "catalog/upstream.json";
pub const STRIKE: &str = "catalog/upstream-state.json";
pub const SIGNALS: &str = "catalog/signals.json";
pub const PATTERN_SPECS: &str = "catalog/patterns.json";

/// Options for `abr signals`.
#[derive(Default)]
pub struct VerifyOpts {
    /// Take only the first N cards (smoke test).
    pub limit: usize,
    /// Write numbers_found / support into catalog/signals.json.
    pub write: bool,
    /// Take one card by name.
    pub only: Option<String>,
    /// Ask the decision model whether the source supports the claim. Off by
    /// default: it costs a request per card and it is not deterministic.
    pub entail: bool,
}

#[derive(Default)]
pub struct SignalOpts {
    /// Score a labelled fixture instead of collecting: `--eval <path>`.
    pub eval: Option<String>,
    /// Ask the decision model which keyword candidates are real critiques.
    pub triage: bool,
    /// Take only the first N repositories (smoke test).
    pub limit: usize,
    /// Take one repository by `owner/name`.
    pub only: Option<String>,
}

pub fn run(cmd: Cmd, io: &(impl Source + Sink)) -> Report {
    let mut state = State::Boot;
    loop {
        state = match state {
            // Boot -> Loaded: adapters translate raw texts into domain terms.
            State::Boot => {
                let readme = load(io, README);
                let patterns_raw = load(io, PATTERNS);
                let catalog_raw = io.read_opt(CATALOG);
                let upstream_raw = io.read_opt(UPSTREAM);
                let model = markdown::parse_readme(&readme);
                let patterns_index = markdown::parse_patterns(&patterns_raw);
                let signals_raw = io.read_opt(SIGNALS);
                let pattern_specs = io
                    .read_opt(PATTERN_SPECS)
                    .map(|r| markdown::parse_pattern_specs(&r))
                    .unwrap_or_default();
                let _ = &pattern_specs;
                State::Loaded(Workspace {
                    model,
                    patterns_index,
                    patterns_raw,
                    catalog_raw,
                    upstream_raw,
                    signals_raw,
                    pattern_specs,
                })
            }
            // Loaded -> Checked: domain invariants run on every command.
            State::Loaded(ws) => fsm::run_checks(ws),
            State::Checked(ws, violations) => match cmd {
                Cmd::Check => {
                    let r = fsm::check_report(&ws, &violations);
                    if r.ok {
                        State::Done(r)
                    } else {
                        State::Failed(r)
                    }
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
                    Cmd::Catalog => format!(
                        "catalog: wrote {CATALOG} ({} entries)",
                        ws.model.entries.len()
                    ),
                    Cmd::Patterns => {
                        format!("patterns: rebuilt index ({} tags)", ws.model.usage().len())
                    }
                    Cmd::Llms => "llms: wrote llms.txt".to_string(),
                    _ => String::new(),
                };
                State::Done(Report {
                    ok: true,
                    summary,
                    failures: vec![],
                    artifacts_written: n,
                })
            }
            State::Done(r) | State::Failed(r) => return r,
        };
    }
}

/// Merge this run's claim checks into catalog/signals.json, leaving every other
/// field untouched: the file is the collector's, and a verify run must not
/// rewrite data it did not measure.
fn record_claims(io: &(impl Source + Sink), checks: &[(String, signals::ClaimCheck)]) {
    let raw = io.read_opt(SIGNALS).unwrap_or_default();
    let by_name: std::collections::BTreeMap<&str, &signals::ClaimCheck> =
        checks.iter().map(|(n, c)| (n.as_str(), c)).collect();
    let model = markdown::parse_readme(&load(io, README));
    let mut updated = 0usize;
    let mut text = String::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        let repo = trimmed
            .strip_prefix("\"repo\": \"")
            .and_then(|r| r.split('"').next());
        if let Some(repo) = repo {
            if let Some(entry) = model
                .entries
                .iter()
                .find(|e| crate::domain::upstream::repo_slug(&e.url).as_deref() == Some(repo))
            {
                if let Some(check) = by_name.get(entry.name.as_str()) {
                    text.push_str(line);
                    text.push('\n');
                    text.push_str(&format!(
                        "      \"claim\": {{\"numbers_found\": {}, \"numbers_total\": {}{}}},\n",
                        check.numbers_found,
                        check.numbers_total,
                        match check.support {
                            Some(p) => format!(", \"support\": {p:.2}"),
                            None => String::new(),
                        }
                    ));
                    updated += 1;
                    continue;
                }
            }
        }
        text.push_str(line);
        text.push('\n');
    }
    io.write(SIGNALS, &text);
    eprintln!("verify: recorded claim checks for {updated} repos in {SIGNALS}");
}

/// Provenance check: does each measured card's number appear at its source?
///
/// Advisory by design. A pass means the figure was not invented or transposed;
/// a miss means the text match failed, which also happens for charts, PDFs and
/// prose. Entailment - whether the source supports the claim - is a separate
/// question and is not answered here.
pub fn run_verify(
    io: &(impl Source + Sink),
    page: &impl crate::ports::verify::PageSource,
    judge: &impl crate::ports::verify::EntailmentJudge,
    opts: &VerifyOpts,
) -> Report {
    let readme = load(io, README);
    let model = markdown::parse_readme(&readme);
    let mut cards: Vec<_> = model
        .entries
        .iter()
        .filter(|e| e.kind() == crate::domain::model::Kind::Measured)
        .collect();
    if let Some(only) = &opts.only {
        cards.retain(|e| e.name.eq_ignore_ascii_case(only));
    }
    if opts.limit > 0 {
        cards.truncate(opts.limit);
    }

    let mut checks: Vec<(String, signals::ClaimCheck)> = Vec::new();
    let mut lines = Vec::new();
    let mut checked = 0usize;
    let mut clean = 0usize;
    let mut unlinked = 0usize;
    let mut unfetched = 0usize;
    for e in &cards {
        let Some(url) = verify::first_link(&e.blazing) else {
            unlinked += 1;
            lines.push(format!("NOLINK {:<22} claim cites no URL", e.name));
            continue;
        };
        let Some(text) = page.page(url) else {
            unfetched += 1;
            lines.push(format!("NOFETCH {:<21} {}", e.name, url));
            continue;
        };
        checked += 1;
        let (present, missing) = verify::coverage(&e.blazing, &text);
        let support_p = if opts.entail {
            judge.supports(&e.blazing, &text.chars().take(6000).collect::<String>())
        } else {
            None
        };
        let entail = match (opts.entail, support_p) {
            (true, Some(p)) => format!(" | support {p:.2}"),
            (true, None) => " | support n/a".to_string(),
            _ => String::new(),
        };
        checks.push((
            e.name.clone(),
            signals::ClaimCheck {
                numbers_found: present.len(),
                numbers_total: present.len() + missing.len(),
                support: support_p,
            },
        ));
        if missing.is_empty() {
            clean += 1;
            lines.push(format!(
                "OK {:<26} {}/{} numbers found{entail}",
                e.name,
                present.len(),
                present.len()
            ));
        } else {
            lines.push(format!(
                "MISS {:<24} {}/{} found; missing: {}{entail}",
                e.name,
                present.len(),
                present.len() + missing.len(),
                missing.join(", ")
            ));
        }
    }

    if opts.write {
        record_claims(io, &checks);
    }

    let summary = format!(
        "verify: {} measured cards, {} checked against their source, {} fully present, {} unlinked, {} unfetched\n{}",
        cards.len(),
        checked,
        clean,
        unlinked,
        unfetched,
        lines.join("\n")
    );
    Report {
        ok: true,
        summary,
        failures: Vec::new(),
        artifacts_written: 0,
    }
}

/// Signals: third-party context about every cataloged repository.
///
/// Same shape as `run_upstream` - reuse the Boot -> Loaded -> Checked prefix so
/// the invariants run, then do the I/O behind ports and commit one projection.
pub fn run_signals(
    io: &(impl Source + Sink),
    source: &impl crate::ports::signals::SignalsSource,
    decisions: &impl DecisionSource,
    opts: &SignalOpts,
) -> Report {
    if let Some(path) = &opts.eval {
        return eval_triage(io, decisions, path);
    }
    let readme = load(io, README);
    let model = markdown::parse_readme(&readme);

    let mut slugs: Vec<String> = model
        .entries
        .iter()
        .filter_map(|e| upstream::repo_slug(&e.url))
        .collect();
    slugs.sort();
    slugs.dedup();
    if let Some(only) = &opts.only {
        slugs.retain(|s| s.eq_ignore_ascii_case(only));
    }
    if opts.limit > 0 {
        slugs.truncate(opts.limit);
    }

    let previous = io.read_opt(SIGNALS).unwrap_or_default();
    let (verdicts, notes) = signals::parse_previous(&previous);
    let triage_on = opts.triage && decisions.model_available();
    let today = signals::today_days();
    let observed = signals::today_utc();

    let mut rows = Vec::new();
    let mut triaged = 0usize;
    let mut review = 0usize;
    for (i, slug) in slugs.iter().enumerate() {
        let name = slug.split('/').nth(1).unwrap_or(slug.as_str()).to_string();
        let health = source.health(slug);
        let krate = source.crate_info(&name, slug);
        let crate_missing = krate.is_none();
        let mentions = source.attention(&name);
        let attention = signals::Attention {
            stories: mentions.stories,
            comments: mentions.comments,
            download_share: krate
                .as_ref()
                .and_then(|c| signals::download_share(c.downloads, c.recent_downloads)),
            stars_per_day: signals::stars_per_day(
                health.as_ref().and_then(|h| h.stars),
                health.as_ref().and_then(|h| h.created_at.as_deref()),
                today,
            ),
        };
        let advisories = source.advisories(&name);

        let (archived, pushed_at) = match &health {
            Some(h) => (h.archived, h.pushed_at.clone()),
            None => (None, None),
        };
        let flags = signals::machine_flags(archived, pushed_at.as_deref(), &advisories, today);

        let mut disputes = Vec::new();
        if triage_on {
            for c in source.dispute_candidates(&name, slug) {
                let p = decisions.critique_p(slug, &c.text);
                match signals::bucket(p) {
                    Bucket::Accept => triaged += 1,
                    Bucket::Review => review += 1,
                    Bucket::Drop => continue,
                }
                disputes.push(Dispute {
                    source: c.source,
                    url: c.url,
                    created_at: c.created_at,
                    status: match signals::bucket(p) {
                        Bucket::Accept => "machine-triaged".to_string(),
                        _ => "needs-review".to_string(),
                    },
                    p,
                });
            }
        }

        eprintln!(
            "[{}/{}] {}: crate={} flags={:?} disputes={}",
            i + 1,
            slugs.len(),
            slug,
            if krate.is_some() { "yes" } else { "no" },
            flags,
            disputes.len()
        );

        rows.push(SignalRow {
            repo: slug.clone(),
            observed_at: observed.clone(),
            archived,
            pushed_at,
            stars: health.as_ref().and_then(|h| h.stars),
            license: health.as_ref().and_then(|h| h.license.clone()),
            krate: krate.map(|c| Crate {
                name: c.name,
                downloads: c.downloads,
                recent_downloads: c.recent_downloads,
                versions: c.versions,
                yanked: c.yanked,
                updated_at: c.updated_at,
            }),
            attention,
            advisories,
            disputes,
            unavailable: {
                let mut u = Vec::new();
                if crate_missing {
                    u.push("crate".to_string());
                }
                if health.is_none() {
                    u.push("github".to_string());
                }
                u
            },
            flags,
            claim: None, // filled by `run_verify` when it writes the report
            verdict: verdicts.get(slug).cloned(),
            notes: notes.get(slug).cloned(),
        });
    }

    let generated_at = format!("{}T00:00:00Z", observed);
    let doc = signals::json(&rows, &generated_at, triage_on);
    io.write(SIGNALS, &doc);

    let flagged = rows.iter().filter(|r| !r.flags.is_empty()).count();
    let with_crate = rows.iter().filter(|r| r.krate.is_some()).count();
    let mut summary = format!(
        "signals: {} repos, {} with a crate, {} flagged; triage {}; wrote {SIGNALS}",
        rows.len(),
        with_crate,
        flagged,
        if triage_on {
            format!("on ({triaged} accepted, {review} for review)")
        } else {
            "off".to_string()
        }
    );
    let failures = rows
        .iter()
        .filter(|r| !r.flags.is_empty())
        .map(|r| format!("FLAG {} {:?}", r.repo, r.flags))
        .collect();
    summary.push_str(&format!(
        "\n{} flag lines follow",
        rows.iter().filter(|r| !r.flags.is_empty()).count()
    ));
    Report {
        ok: true,
        summary,
        failures,
        artifacts_written: 1,
    }
}

/// Score triage against a labelled fixture: `items[] = {repo, label, text}`.
///
/// The measurement the threshold rests on. False positives (noise the model
/// keeps) fail the command; false negatives (a real critique dropped without
/// review) are reported, because they are the failure that lets a contradicted
/// claim through.
fn eval_triage(io: &impl Source, decisions: &impl DecisionSource, path: &str) -> Report {
    if !decisions.model_available() {
        return Report {
            ok: false,
            summary: "signals --eval: no decision model available (set SUBQ_API_KEY)".to_string(),
            failures: vec![],
            artifacts_written: 0,
        };
    }
    let raw = load(io, path);
    let rows = crate::adapters::net::jq(&raw, ".items[] | [.repo, .label, .text] | @tsv")
        .map(|out| crate::adapters::net::tsv_lines(&out))
        .unwrap_or_default();
    let mut kept = 0usize;
    let mut review = 0usize;
    let mut fp = 0usize;
    let mut lost = 0usize;
    let mut lines = Vec::new();
    for f in &rows {
        if f.len() < 3 {
            continue;
        }
        let (repo, label, text) = (&f[0], &f[1], &f[2]);
        let p = decisions.critique_p(repo, text);
        let truth = label == "critique";
        let bucket = signals::bucket(p);
        let mark = match bucket {
            Bucket::Accept => {
                kept += 1;
                "KEEP"
            }
            Bucket::Review => {
                review += 1;
                "review"
            }
            Bucket::Drop => "drop",
        };
        if mark == "KEEP" && !truth {
            fp += 1;
        }
        if truth && mark == "drop" {
            lost += 1;
        }
        let shown = match p {
            Some(p) => format!("{p:.2}"),
            None => "-".to_string(),
        };
        lines.push(format!(
            "{mark:6} p={shown:5} [{}] {repo}",
            if truth { "critique" } else { "noise" }
        ));
    }
    let report = format!(
        "signals --eval: {} items, {kept} accepted, {review} in review, {fp} false positives, {lost} lost\n{}",
        lines.len(),
        lines.join("\n")
    );
    Report {
        ok: fp == 0 && lost == 0,
        summary: report,
        failures: Vec::new(),
        artifacts_written: 0,
    }
}

/// Upstream scan: fetch latest tags through the port, diff against the
/// previous projection, commit the new one. Runs from Checked; the report
/// carries the changed-repo lines so the pipeline can act on them.
pub fn run_upstream(io: &(impl Source + Sink), upstream: &impl UpstreamSource) -> Report {
    // Reuse the same Boot -> Loaded -> Checked prefix for invariant parity.
    let readme = load(io, README);
    let patterns_raw = load(io, PATTERNS);
    let catalog_raw = io.read_opt(CATALOG);
    let upstream_raw = io.read_opt(UPSTREAM);
    let model = markdown::parse_readme(&readme);
    let patterns_index = markdown::parse_patterns(&patterns_raw);
    let signals_raw = io.read_opt(SIGNALS);
    let pattern_specs = io
        .read_opt(PATTERN_SPECS)
        .map(|r| markdown::parse_pattern_specs(&r))
        .unwrap_or_default();
    let ws = Workspace {
        model,
        patterns_index,
        patterns_raw,
        catalog_raw,
        upstream_raw,
        signals_raw,
        pattern_specs,
    };
    let checked = fsm::run_checks(ws);

    let ws = match checked {
        State::Checked(ws, _) => ws,
        _ => unreachable!("run_checks always returns Checked"),
    };

    let mut slugs: Vec<String> = ws
        .model
        .entries
        .iter()
        .filter_map(|e| upstream::repo_slug(&e.url))
        .collect();
    slugs.sort();
    slugs.dedup();

    let mut rows: Vec<UpstreamRow> = Vec::new();
    let mut missing: Vec<String> = Vec::new();
    for slug in &slugs {
        match upstream.latest_tag(slug) {
            TagState::Tag(tag) => rows.push(UpstreamRow {
                repo: slug.clone(),
                tag,
            }),
            // stable legitimate state - must not churn the weekly diff
            TagState::NoTags => rows.push(UpstreamRow {
                repo: slug.clone(),
                tag: "(no-tags)".into(),
            }),
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
            summary: format!(
                "upstream: scanned {} repos, incident - nothing written",
                slugs.len()
            ),
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
        failures.push(format!(
            "TRIPWIRE {repo}: 3 strikes (enforcement is phase 2)"
        ));
    }
    Report {
        ok: true,
        summary,
        failures,
        artifacts_written: 2,
    }
}
