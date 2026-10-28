//! Signals domain: third-party context about a cataloged repository.
//!
//! DATA, NOT EVIDENCE. Nothing here can raise a card's `**Type:**`; machine
//! flags are facts (an advisory inside the window, an archived repository) and
//! a `verdict` is a human decision that survives every recollect. The
//! asymmetry is enforced in `rules::check_signals`.

/// The closed vocabulary a flag may use. `security` and `unmaintained` are
/// machine facts; the rest require judgement and belong in a human verdict.
pub const ALL_FLAGS: &[&str] = &["security", "unmaintained", "contested", "hype-suspect", "astroturf-suspected", "stale-claim"];

/// RUSTSEC keeps every advisory forever; only the last two years are a current
/// risk. The advisory id encodes the year (`RUSTSEC-2026-0041`).
pub const SECURITY_WINDOW_YEARS: i32 = 2;
/// Signals expire: an old wave must not hold a project forever.
pub const STALE_AFTER_DAYS: i64 = 365;
/// Triage buckets: accept, review, drop. The docs' advice - route a noul near
/// 0.5 to a person - is why there are three and not two, and why the review
/// band is [0.5, 0.65): below 0.5 the model already leans "no", and the
/// fixture's noise sits at 0.34-0.42.
pub const TRIAGE_ACCEPT: f64 = 0.65;
pub const TRIAGE_REVIEW: f64 = 0.5;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Crate {
    pub name: String,
    pub downloads: u64,
    pub recent_downloads: u64,
    pub versions: u64,
    pub yanked: u64,
    pub updated_at: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Attention {
    pub stories: u64,
    pub comments: u64,
    /// Stars gained per day since the repository was created. Attention, not a
    /// score: fake stars have a promotion effect under two months and become a
    /// liability after (StarScout, ICSE 2026). Never shown on a card.
    pub stars_per_day: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Dispute {
    pub source: String,
    pub url: String,
    pub created_at: String,
    /// `machine-triaged` | `needs-review` | `unverified`
    pub status: String,
    /// triage probability, percent-free (0..1) when the model was asked
    pub p: Option<f64>,
}

#[derive(Clone, Debug, Default)]
pub struct SignalRow {
    pub repo: String,
    pub observed_at: String,
    pub archived: Option<bool>,
    pub pushed_at: Option<String>,
    pub stars: Option<u64>,
    pub license: Option<String>,
    pub krate: Option<Crate>,
    pub attention: Attention,
    pub advisories: Vec<String>,
    pub disputes: Vec<Dispute>,
    pub flags: Vec<String>,
    pub verdict: Option<String>,
    pub notes: Option<String>,
}

pub fn stars_per_day(stars: Option<u64>, created_at: Option<&str>, today: i64) -> Option<f64> {
    let stars = stars? as f64;
    let created = parse_date_days(created_at?)?;
    let days = (today - created).max(1) as f64;
    Some((stars / days * 10.0).round() / 10.0)
}

/// Days since 1970-01-01 for a civil date (Howard Hinnant's algorithm).
pub fn days_from_civil(y: i32, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y } as i64;
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = ((m as i64) + 9) % 12;
    let doy = (153 * mp + 2) / 5 + (d as i64) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

pub fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    ((if m <= 2 { y + 1 } else { y }) as i32, m, d)
}

pub fn today_days() -> i64 {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    secs / 86_400
}

pub fn today_utc() -> String {
    let (y, m, d) = civil_from_days(today_days());
    format!("{y:04}-{m:02}-{d:02}")
}

/// `2026-10-21T11:37:38Z` or `2026-10-21` -> days since epoch.
pub fn parse_date_days(s: &str) -> Option<i64> {
    let date = s.split('T').next()?;
    let mut parts = date.split('-');
    let y: i32 = parts.next()?.parse().ok()?;
    let m: u32 = parts.next()?.parse().ok()?;
    let d: u32 = parts.next()?.parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some(days_from_civil(y, m, d))
}

fn advisory_year(id: &str) -> Option<i32> {
    id.split('-').nth(1)?.parse().ok()
}

/// Objective flags. `today` is a parameter, not a clock read, so the rule is
/// pure and testable.
pub fn machine_flags(archived: Option<bool>, pushed_at: Option<&str>, advisories: &[String], today: i64) -> Vec<String> {
    let mut flags = Vec::new();
    let (current_year, _, _) = civil_from_days(today);
    if advisories
        .iter()
        .filter_map(|id| advisory_year(id))
        .any(|y| y > current_year - SECURITY_WINDOW_YEARS)
    {
        flags.push("security".to_string());
    }
    let dead = archived == Some(true)
        || pushed_at
            .and_then(parse_date_days)
            .map(|d| today - d > STALE_AFTER_DAYS)
            .unwrap_or(false);
    if dead {
        flags.push("unmaintained".to_string());
    }
    flags
}

/// Which bucket a triage probability lands in.
pub enum Bucket {
    Accept,
    Review,
    Drop,
}

pub fn bucket(p: Option<f64>) -> Bucket {
    match p {
        Some(p) if p >= TRIAGE_ACCEPT => Bucket::Accept,
        Some(p) if p >= TRIAGE_REVIEW => Bucket::Review,
        _ => Bucket::Drop,
    }
}

/// Verdicts and notes a human wrote are preserved across recollects: a
/// scheduled job must not overwrite a maintainer's decision.
pub fn parse_previous(raw: &str) -> (std::collections::BTreeMap<String, String>, std::collections::BTreeMap<String, String>) {
    let mut verdicts = std::collections::BTreeMap::new();
    let mut notes = std::collections::BTreeMap::new();
    let mut repo: Option<String> = None;
    for line in raw.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("\"repo\": \"") {
            repo = rest.split('"').next().map(str::to_string);
            continue;
        }
        let Some(current) = repo.as_ref() else { continue };
        if let Some(rest) = t.strip_prefix("\"verdict\": \"") {
            if let Some(v) = rest.split('"').next() {
                if !v.is_empty() {
                    verdicts.insert(current.clone(), v.to_string());
                }
            }
        } else if let Some(rest) = t.strip_prefix("\"notes\": \"") {
            // The writer emits `"notes": "..."` on one line, optionally followed
            // by a comma; escaped quotes inside a note are not re-read.
            let v = rest.trim_end().trim_end_matches(',').trim_end_matches('"');
            if !v.is_empty() {
                notes.insert(current.clone(), v.to_string());
            }
        }
    }
    (verdicts, notes)
}

fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
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

fn opt_str(v: &Option<String>, indent: &str) -> String {
    match v {
        Some(s) => format!("\"{}\"", esc(s)),
        None => "null".to_string(),
    }
    .replace('\n', indent)
}

fn crate_block(c: &Option<Crate>, indent: &str) -> String {
    let i2 = format!("{indent}  ");
    match c {
        None => "null".to_string(),
        Some(c) => {
            let mut out = String::from("{\n");
            out.push_str(&format!("{i2}\"name\": \"{}\",\n", esc(&c.name)));
            out.push_str(&format!("{i2}\"downloads\": {},\n", c.downloads));
            out.push_str(&format!("{i2}\"recent_downloads\": {},\n", c.recent_downloads));
            out.push_str(&format!("{i2}\"versions\": {},\n", c.versions));
            out.push_str(&format!("{i2}\"yanked\": {},\n", c.yanked));
            out.push_str(&format!("{i2}\"updated_at\": \"{}\"\n", esc(&c.updated_at)));
            out.push_str(&format!("{indent}}}"));
            out
        }
    }
}

/// Deterministic projection: rows sorted by repo, fixed key order, two-space
/// indent - the shape `catalog/signals.schema.json` describes and the scanner
/// in `rules::parse_signals` reads.
pub fn json(rows: &[SignalRow], generated_at: &str, decision_enabled: bool) -> String {
    let mut sorted: Vec<&SignalRow> = rows.iter().collect();
    sorted.sort_by(|a, b| a.repo.cmp(&b.repo));
    let mut out = String::from("{\n  \"meta\": {\n");
    out.push_str("    \"format\": \"awesome-blazing-rust signals v1\",\n");
    out.push_str("    \"rule\": \"data, not evidence: signals can only hold or demote a card, never raise its Type\",\n");
    out.push_str("    \"sources\": [\"crates.io\", \"hn.algolia.com\", \"rustsec/advisory-db\", \"api.github.com via gh\"],\n");
    if decision_enabled {
        out.push_str("    \"sources_note\": \"triage via subq.ai decisions\",\n");
    }
    out.push_str("    \"flag_vocabulary\": [\"security\", \"unmaintained\", \"contested\", \"hype-suspect\", \"astroturf-suspected\", \"stale-claim\"],\n");
    out.push_str(&format!("    \"generated_at\": \"{}\",\n", esc(generated_at)));
    out.push_str(&format!("    \"repos\": {}\n  }},\n  \"repos\": [", sorted.len()));
    for (i, r) in sorted.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str("\n    {\n");
        out.push_str(&format!("      \"repo\": \"{}\",\n", esc(&r.repo)));
        out.push_str(&format!("      \"observed_at\": \"{}\",\n", esc(&r.observed_at)));
        out.push_str("      \"github\": {\n");
        let archived = match r.archived {
            Some(b) => b.to_string(),
            None => "null".to_string(),
        };
        out.push_str(&format!("        \"archived\": {archived},\n"));
        out.push_str(&format!("        \"pushed_at\": {},\n", opt_str(&r.pushed_at, "        ")));
        let stars = match r.stars {
            Some(s) => s.to_string(),
            None => "null".to_string(),
        };
        out.push_str(&format!("        \"stars\": {stars},\n"));
        out.push_str(&format!("        \"license\": {}\n", opt_str(&r.license, "        ")));
        out.push_str("      },\n");
        out.push_str(&format!("      \"crate\": {},\n", crate_block(&r.krate, "      ")));
        out.push_str(&format!(
            "      \"attention\": {{\n        \"stories\": {},\n        \"comments\": {},\n        \"stars_per_day\": {}\n      }},\n",
            r.attention.stories,
            r.attention.comments,
            match r.attention.stars_per_day {
                Some(v) => format!("{v:.1}"),
                None => "null".to_string(),
            }
        ));
        if r.advisories.is_empty() {
            out.push_str("      \"advisories\": [],\n");
        } else {
            out.push_str("      \"advisories\": [\n");
            for (j, a) in r.advisories.iter().enumerate() {
                let comma = if j + 1 == r.advisories.len() { "" } else { "," };
                out.push_str(&format!("        {{\n          \"id\": \"{}\"\n        }}{comma}\n", esc(a)));
            }
            out.push_str("      ],\n");
        }
        if r.disputes.is_empty() {
            out.push_str("      \"disputes\": [],\n");
        } else {
            out.push_str("      \"disputes\": [\n");
            for (j, d) in r.disputes.iter().enumerate() {
                let comma = if j + 1 == r.disputes.len() { "" } else { "," };
                out.push_str("        {\n");
                out.push_str(&format!("          \"source\": \"{}\",\n", esc(&d.source)));
                out.push_str(&format!("          \"url\": \"{}\",\n", esc(&d.url)));
                if !d.created_at.is_empty() {
                    out.push_str(&format!("          \"created_at\": \"{}\",\n", esc(&d.created_at)));
                }
                out.push_str(&format!("          \"status\": \"{}\"", esc(&d.status)));
                if let Some(p) = d.p {
                    out.push_str(&format!(",\n          \"p\": {p:.2}"));
                }
                out.push_str(&format!("\n        }}{comma}\n"));
            }
            out.push_str("      ],\n");
        }
        if r.flags.is_empty() {
            out.push_str("      \"flags\": []");
        } else {
            out.push_str("      \"flags\": [\n");
            for (j, f) in r.flags.iter().enumerate() {
                let comma = if j + 1 == r.flags.len() { "" } else { "," };
                out.push_str(&format!("        \"{}\"{comma}\n", esc(f)));
            }
            out.push_str("      ]");
        }
        if let Some(v) = &r.verdict {
            out.push_str(&format!(",\n      \"verdict\": \"{}\"", esc(v)));
        }
        if let Some(n) = &r.notes {
            out.push_str(&format!(",\n      \"notes\": \"{}\"", esc(n)));
        }
        out.push_str("\n    }");
    }
    out.push_str("\n  ]\n}\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_roundtrip() {
        for days in [0i64, 19_000, 20_000, 20_744] {
            let (y, m, d) = civil_from_days(days);
            assert_eq!(days_from_civil(y, m, d), days);
        }
        assert_eq!(civil_from_days(0), (1970, 1, 1));
    }

    #[test]
    fn security_flag_uses_a_two_year_window() {
        let today = days_from_civil(2026, 10, 27);
        let old = vec!["RUSTSEC-2019-0008".to_string()];
        let recent = vec!["RUSTSEC-2026-0041".to_string()];
        assert!(machine_flags(Some(false), Some("2026-10-01"), &old, today).is_empty());
        assert_eq!(machine_flags(Some(false), Some("2026-10-01"), &recent, today), vec!["security".to_string()]);
    }

    #[test]
    fn unmaintained_covers_archive_and_staleness() {
        let today = days_from_civil(2026, 10, 27);
        assert_eq!(machine_flags(Some(true), Some("2026-10-01"), &[], today), vec!["unmaintained".to_string()]);
        assert_eq!(machine_flags(Some(false), Some("2024-01-01"), &[], today), vec!["unmaintained".to_string()]);
        assert!(machine_flags(Some(false), Some("2026-09-01"), &[], today).is_empty());
    }

    #[test]
    fn stars_per_day_is_attention_not_a_score() {
        let today = days_from_civil(2026, 10, 27);
        // 316 stars in 8 days: a burst, recorded and never shown on a card
        assert_eq!(stars_per_day(Some(316), Some("2026-10-19T03:05:36Z"), today), Some(39.5));
        assert_eq!(stars_per_day(Some(0), Some("2026-10-19"), today), Some(0.0));
        // a same-day repository divides by one, not by zero
        assert_eq!(stars_per_day(Some(5), Some("2026-10-27"), today), Some(5.0));
        assert_eq!(stars_per_day(None, Some("2026-10-19"), today), None);
        assert_eq!(stars_per_day(Some(5), None, today), None);
    }

    #[test]
    fn triage_buckets_follow_the_documented_cutoffs() {
        assert!(matches!(bucket(Some(0.96)), Bucket::Accept));
        assert!(matches!(bucket(Some(0.61)), Bucket::Review));
        assert!(matches!(bucket(Some(0.50)), Bucket::Review));
        // the fixture's noise tails at 0.42 - below the review band
        assert!(matches!(bucket(Some(0.42)), Bucket::Drop));
        assert!(matches!(bucket(Some(0.10)), Bucket::Drop));
        assert!(matches!(bucket(None), Bucket::Drop));
    }

    #[test]
    fn projection_is_deterministic_and_sorted() {
        let row = |repo: &str| SignalRow {
            repo: repo.to_string(),
            observed_at: "2026-10-27".to_string(),
            flags: vec![],
            ..Default::default()
        };
        let a = json(&[row("b/b"), row("a/a")], "2026-10-27T00:00:00Z", false);
        let b = json(&[row("a/a"), row("b/b")], "2026-10-27T00:00:00Z", false);
        assert_eq!(a, b);
        assert!(a.find("\"a/a\"").unwrap() < a.find("\"b/b\"").unwrap());
    }

    #[test]
    fn previous_verdicts_survive() {
        let raw = "{\n  \"repos\": [\n    {\n      \"repo\": \"a/b\",\n      \"flags\": [],\n      \"verdict\": \"contested\",\n      \"notes\": \"checked by hand\"\n    }\n  ]\n}";
        let (v, n) = parse_previous(raw);
        assert_eq!(v.get("a/b").map(String::as_str), Some("contested"));
        assert_eq!(n.get("a/b").map(String::as_str), Some("checked by hand"));
    }
}
