//! Signals domain: third-party context about a cataloged repository.
//!
//! DATA, NOT EVIDENCE. Nothing here can raise a card's `**Type:**`; machine
//! flags are facts (an advisory inside the window, an archived repository) and
//! a `verdict` is a human decision that survives every recollect. The
//! asymmetry is enforced in `rules::check_signals`.

/// The closed vocabulary a flag may use. `security` and `unmaintained` are
/// machine facts; the rest require judgement and belong in a human verdict.
pub const ALL_FLAGS: &[&str] = &[
    "security",
    "unmaintained",
    "contested",
    "hype-suspect",
    "astroturf-suspected",
    "stale-claim",
];

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
    /// Share of lifetime downloads that arrived in the recent window
    /// (crates.io reports 90 days). A live project keeps this roughly flat; a
    /// fading one drops while its repository still shows commits.
    pub download_share: Option<f64>,
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

/// What `abr verify` found for a measured card: how many of the claim's
/// numbers are on the linked page, and (optionally) whether the page supports
/// the claim at all. Mechanical part is deterministic; `support` is a judgement
/// and is stored as one.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ClaimCheck {
    pub numbers_found: usize,
    pub numbers_total: usize,
    pub support: Option<f64>,
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
    /// Which sources did not answer. `None` alone cannot say whether a field is
    /// absent because it does not apply or because the lookup failed.
    pub unavailable: Vec<String>,
    pub flags: Vec<String>,
    /// Present only when the collector was asked to check the claim.
    pub claim: Option<ClaimCheck>,
    pub verdict: Option<String>,
    pub notes: Option<String>,
}

/// `recent_downloads / downloads`. Adoption that has stopped growing shows up
/// here while the repository is still being pushed to.
pub fn download_share(downloads: u64, recent: u64) -> Option<f64> {
    if downloads == 0 {
        return None;
    }
    Some((recent as f64 / downloads as f64 * 1000.0).round() / 1000.0)
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
pub fn machine_flags(
    archived: Option<bool>,
    pushed_at: Option<&str>,
    advisories: &[String],
    today: i64,
) -> Vec<String> {
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

pub fn parse_previous(
    raw: &str,
) -> (
    std::collections::BTreeMap<String, String>,
    std::collections::BTreeMap<String, String>,
) {
    let mut verdicts = std::collections::BTreeMap::new();
    let mut notes = std::collections::BTreeMap::new();
    let mut repo: Option<String> = None;
    for line in raw.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("\"repo\": \"") {
            repo = rest.split('"').next().map(str::to_string);
            continue;
        }
        let Some(current) = repo.as_ref() else {
            continue;
        };
        if let Some(rest) = trimmed.strip_prefix("\"verdict\": \"") {
            if let Some(v) = rest.split('"').next() {
                if !v.is_empty() {
                    verdicts.insert(current.clone(), v.to_string());
                }
            }
        } else if let Some(rest) = trimmed.strip_prefix("\"notes\": \"") {
            let v = rest.trim_end().trim_end_matches(',').trim_end_matches('"');
            if !v.is_empty() {
                notes.insert(current.clone(), v.to_string());
            }
        }
    }
    (verdicts, notes)
}

/// Parse claim check data alongside verdicts and notes.
/// Returns (verdicts, notes, claims) - each keyed by repo slug.
pub fn parse_previous_with_claims(
    raw: &str,
) -> (
    std::collections::BTreeMap<String, String>,
    std::collections::BTreeMap<String, String>,
    std::collections::BTreeMap<String, ClaimCheck>,
) {
    let (v, n) = parse_previous(raw);
    // Re-scan for claim blocks: "claim": { "numbers_found": N, "numbers_total": N, "support": P }
    let mut claims = std::collections::BTreeMap::new();
    let mut repo: Option<String> = None;
    let mut found_nf = None;
    let mut found_nt = None;
    let mut found_support = None;
    for line in raw.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("\"repo\": \"") {
            if let Some(slug) = rest.split('"').next() {
                repo = Some(slug.to_string());
            }
            continue;
        }
        let Some(current) = repo.as_ref() else {
            continue;
        };
        if let Some(rest) = trimmed.strip_prefix("\"claim\": {") {
            if let Some(nf) = rest
                .strip_prefix("{\"numbers_found\": ")
                .and_then(|r| r.split(", ").next())
            {
                found_nf = nf.parse::<usize>().ok();
            }
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("\"numbers_found\": ") {
            if let Ok(n) = rest.trim_end_matches(',').parse::<usize>() {
                found_nf = Some(n);
            }
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("\"numbers_total\": ") {
            if let Ok(n) = rest.trim_end_matches(',').parse::<usize>() {
                found_nt = Some(n);
            }
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("\"support\": ") {
            if let Ok(p) = rest.trim_end_matches(',').parse::<f64>() {
                found_support = Some(p);
            }
            continue;
        }
        // Object closing - emit if we have enough data
        if trimmed.starts_with('}') || trimmed.starts_with("},") || trimmed.starts_with('}') {
            if let (Some(nf), Some(nt)) = (found_nf, found_nt) {
                claims.insert(
                    current.clone(),
                    ClaimCheck {
                        numbers_found: nf,
                        numbers_total: nt,
                        support: found_support,
                    },
                );
            }
            found_nf = None;
            found_nt = None;
            found_support = None;
        }
    }
    (v, n, claims)
}

/// Apply claim checks to a signals document: each named repo's claim is
/// replaced in place, or inserted right after its "repo" field when the row
/// had none. Repos without a check are left untouched. The document is
/// parsed, modified as data, rendered by the shared writer, and re-parsed as
/// a validity gate before the caller may write it.
pub fn apply_claims(raw: &str, checks: &[(String, ClaimCheck)]) -> Result<String, String> {
    use super::json::Value;
    let by_slug: std::collections::BTreeMap<&str, &ClaimCheck> =
        checks.iter().map(|(s, c)| (s.as_str(), c)).collect();
    let mut doc = super::json::parse(raw)?;
    if let Value::Obj(top) = &mut doc {
        if let Some(Value::Block(rows)) = top.iter_mut().find(|(k, _)| k == "repos").map(|(_, v)| v)
        {
            for row in rows.iter_mut() {
                let Value::Obj(fields) = row else {
                    continue;
                };
                let slug = fields.iter().find_map(|(k, v)| match (k.as_str(), v) {
                    ("repo", Value::Str(s)) => Some(s.clone()),
                    _ => None,
                });
                let Some(slug) = slug else {
                    continue;
                };
                let Some(check) = by_slug.get(slug.as_str()) else {
                    continue;
                };
                let claim = claim_value(check);
                match fields.iter().position(|(k, _)| k == "claim") {
                    Some(i) => fields[i].1 = claim,
                    None => {
                        let at = fields
                            .iter()
                            .position(|(k, _)| k == "repo")
                            .map(|i| i + 1)
                            .unwrap_or(0);
                        fields.insert(at, ("claim".to_string(), claim));
                    }
                }
            }
        }
    }
    let mut out = doc.render();
    out.push('\n');
    super::json::parse(&out)
        .map_err(|e| format!("rendered signals document does not re-parse: {e}"))?;
    Ok(out)
}

fn claim_value(c: &ClaimCheck) -> super::json::Value {
    use super::json::Value;
    let mut f = vec![
        ("numbers_found", Value::num(c.numbers_found)),
        ("numbers_total", Value::num(c.numbers_total)),
    ];
    if let Some(p) = c.support {
        f.push(("support", Value::Num(format!("{p:.2}"))));
    }
    Value::obj(f)
}

/// Carry over rows from `previous` whose repo is absent from `doc`, so a
/// partial collection (`--only`, `--limit`) cannot erase repos it did not
/// fetch. The carried rows keep their last-known data, claim, and verdict;
/// the row count in the meta block is corrected to match.
pub fn carry_absent(doc: &str, previous: &str) -> Result<String, String> {
    use super::json::Value;
    let mut d = super::json::parse(doc)?;
    let prev = super::json::parse(previous)?;
    let Value::Obj(top) = &mut d else {
        return Err("signals document is not an object".to_string());
    };
    let Some(ri) = top.iter().position(|(k, _)| k == "repos") else {
        return Err("signals document has no repos block".to_string());
    };
    let meta_i = top.iter().position(|(k, _)| k == "meta");
    let Value::Block(rows) = &mut top[ri].1 else {
        return Err("signals document has no repos block".to_string());
    };
    let Value::Obj(ptop) = prev else {
        return Err("previous signals document is not an object".to_string());
    };
    let Some(Value::Block(prows)) = ptop.iter().find(|(k, _)| k == "repos").map(|(_, v)| v) else {
        return Err("previous signals document has no repos block".to_string());
    };
    let have: std::collections::BTreeSet<String> = rows
        .iter()
        .filter_map(|r| match r {
            Value::Obj(f) => f.iter().find_map(|(k, v)| match (k.as_str(), v) {
                ("repo", Value::Str(s)) => Some(s.clone()),
                _ => None,
            }),
            _ => None,
        })
        .collect();
    let mut carried = 0usize;
    for row in prows {
        let slug = match row {
            Value::Obj(f) => f.iter().find_map(|(k, v)| match (k.as_str(), v) {
                ("repo", Value::Str(s)) => Some(s.clone()),
                _ => None,
            }),
            _ => None,
        };
        if let Some(s) = slug {
            if !have.contains(&s) {
                rows.push(row.clone());
                carried += 1;
            }
        }
    }
    if let Some(mi) = meta_i {
        if let Value::Obj(meta) = &mut top[mi].1 {
            if let Some((_, count)) = meta.iter_mut().find(|(k, _)| k == "repos") {
                *count = Value::num(have.len() + carried);
            }
        }
    }
    let mut out = d.render();
    out.push('\n');
    super::json::parse(&out)
        .map_err(|e| format!("rendered signals document does not re-parse: {e}"))?;
    Ok(out)
}

/// Deterministic projection: rows sorted by repo, fixed key order, two-space
/// indent - the shape `catalog/signals.schema.json` describes and the scanner
/// in `rules::parse_signals` reads.
pub fn json(rows: &[SignalRow], generated_at: &str, decision_enabled: bool) -> String {
    use super::json::Value;
    let mut sorted: Vec<&SignalRow> = rows.iter().collect();
    sorted.sort_by(|a, b| a.repo.cmp(&b.repo));

    let mut meta = vec![
        (
            "format",
            Value::Str("awesome-blazing-rust signals v1".into()),
        ),
        (
            "rule",
            Value::Str(
                "data, not evidence: signals can only hold or demote a card, never raise its Type"
                    .into(),
            ),
        ),
        (
            "sources",
            Value::strs(&[
                "crates.io",
                "hn.algolia.com",
                "rustsec/advisory-db",
                "api.github.com via gh",
            ]),
        ),
    ];
    if decision_enabled {
        meta.push((
            "sources_note",
            Value::Str("triage via subq.ai decisions".into()),
        ));
    }
    meta.push((
        "flag_vocabulary",
        Value::strs(&[
            "security",
            "unmaintained",
            "contested",
            "hype-suspect",
            "astroturf-suspected",
            "stale-claim",
        ]),
    ));
    meta.push(("generated_at", Value::Str(generated_at.to_string())));
    meta.push(("repos", Value::num(sorted.len())));

    let rows_json: Vec<Value> = sorted
        .iter()
        .map(|r| {
            let mut fields = vec![
                ("repo", Value::Str(r.repo.clone())),
                ("observed_at", Value::Str(r.observed_at.clone())),
                (
                    "github",
                    Value::obj(vec![
                        (
                            "archived",
                            match r.archived {
                                Some(b) => Value::Bool(b),
                                None => Value::Null,
                            },
                        ),
                        ("pushed_at", opt(&r.pushed_at)),
                        (
                            "stars",
                            match r.stars {
                                Some(s) => Value::num(s),
                                None => Value::Null,
                            },
                        ),
                        ("license", opt(&r.license)),
                    ]),
                ),
                ("crate", crate_value(&r.krate)),
                (
                    "attention",
                    Value::obj(vec![
                        ("stories", Value::num(r.attention.stories)),
                        ("comments", Value::num(r.attention.comments)),
                        (
                            "stars_per_day",
                            match r.attention.stars_per_day {
                                Some(v) => Value::Num(format!("{v:.1}")),
                                None => Value::Null,
                            },
                        ),
                        (
                            "download_share",
                            match r.attention.download_share {
                                Some(v) => Value::Num(format!("{v:.3}")),
                                None => Value::Null,
                            },
                        ),
                    ]),
                ),
                (
                    "advisories",
                    Value::Block(
                        r.advisories
                            .iter()
                            .map(|a| Value::obj(vec![("id", Value::Str(a.clone()))]))
                            .collect(),
                    ),
                ),
                (
                    "disputes",
                    Value::Block(
                        r.disputes
                            .iter()
                            .map(|d| {
                                let mut f = vec![
                                    ("source", Value::Str(d.source.clone())),
                                    ("url", Value::Str(d.url.clone())),
                                ];
                                if !d.created_at.is_empty() {
                                    f.push(("created_at", Value::Str(d.created_at.clone())));
                                }
                                f.push(("status", Value::Str(d.status.clone())));
                                if let Some(p) = d.p {
                                    f.push(("p", Value::Num(format!("{p:.2}"))));
                                }
                                Value::obj(f)
                            })
                            .collect(),
                    ),
                ),
                ("flags", Value::strs(&r.flags)),
            ];
            if let Some(c) = &r.claim {
                let mut f = vec![
                    ("numbers_found", Value::num(c.numbers_found)),
                    ("numbers_total", Value::num(c.numbers_total)),
                ];
                if let Some(p) = c.support {
                    f.push(("support", Value::Num(format!("{p:.2}"))));
                }
                fields.push(("claim", Value::obj(f)));
            }
            if !r.unavailable.is_empty() {
                fields.push(("unavailable", Value::strs(&r.unavailable)));
            }
            if let Some(v) = &r.verdict {
                fields.push(("verdict", Value::Str(v.clone())));
            }
            if let Some(n) = &r.notes {
                fields.push(("notes", Value::Str(n.clone())));
            }
            Value::obj(fields)
        })
        .collect();

    let doc = Value::obj(vec![
        ("meta", Value::obj(meta)),
        ("repos", Value::Block(rows_json)),
    ]);
    let mut out = doc.render();
    out.push('\n');
    out
}

fn opt(v: &Option<String>) -> super::json::Value {
    match v {
        Some(s) => super::json::Value::Str(s.clone()),
        None => super::json::Value::Null,
    }
}

fn crate_value(c: &Option<Crate>) -> super::json::Value {
    use super::json::Value;
    match c {
        None => Value::Null,
        Some(c) => Value::obj(vec![
            ("name", Value::Str(c.name.clone())),
            ("downloads", Value::num(c.downloads)),
            ("recent_downloads", Value::num(c.recent_downloads)),
            ("versions", Value::num(c.versions)),
            ("yanked", Value::num(c.yanked)),
            ("updated_at", Value::Str(c.updated_at.clone())),
        ]),
    }
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
        assert_eq!(
            machine_flags(Some(false), Some("2026-10-01"), &recent, today),
            vec!["security".to_string()]
        );
    }

    #[test]
    fn unmaintained_covers_archive_and_staleness() {
        let today = days_from_civil(2026, 10, 27);
        assert_eq!(
            machine_flags(Some(true), Some("2026-10-01"), &[], today),
            vec!["unmaintained".to_string()]
        );
        assert_eq!(
            machine_flags(Some(false), Some("2024-01-01"), &[], today),
            vec!["unmaintained".to_string()]
        );
        assert!(machine_flags(Some(false), Some("2026-09-01"), &[], today).is_empty());
    }

    #[test]
    fn stars_per_day_is_attention_not_a_score() {
        let today = days_from_civil(2026, 10, 27);
        // 316 stars in 8 days: a burst, recorded and never shown on a card
        assert_eq!(
            stars_per_day(Some(316), Some("2026-10-19T03:05:36Z"), today),
            Some(39.5)
        );
        assert_eq!(stars_per_day(Some(0), Some("2026-10-19"), today), Some(0.0));
        // a same-day repository divides by one, not by zero
        assert_eq!(stars_per_day(Some(5), Some("2026-10-27"), today), Some(5.0));
        assert_eq!(stars_per_day(None, Some("2026-10-19"), today), None);
        assert_eq!(stars_per_day(Some(5), None, today), None);
    }

    #[test]
    fn download_share_reads_as_a_fraction_of_lifetime() {
        // ripgrep: 124970 recent of 1635037 lifetime
        assert_eq!(download_share(1635037, 124970), Some(0.076));
        // a dead project: no recent downloads at all
        assert_eq!(download_share(1000, 0), Some(0.0));
        // nothing to divide by
        assert_eq!(download_share(0, 0), None);
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
    fn a_claim_check_round_trips_through_the_writer() {
        let row = SignalRow {
            repo: "a/b".into(),
            observed_at: "2026-10-31".into(),
            claim: Some(ClaimCheck {
                numbers_found: 9,
                numbers_total: 11,
                support: Some(0.96),
            }),
            ..Default::default()
        };
        let doc = json(&[row], "2026-10-31T00:00:00Z", false);
        assert!(doc.contains("\"claim\""), "{doc}");
        assert!(doc.contains("\"numbers_found\": 9"));
        assert!(doc.contains("\"support\": 0.96"));
    }

    #[test]
    fn the_writer_and_the_readers_agree() {
        // Round trip over the one format three places encode: the writer below,
        // rules::parse_signals, and parse_previous. A drift between them is the
        // bug class this test exists to catch.
        let row = SignalRow {
            repo: "owner/repo".to_string(),
            observed_at: "2026-10-28".to_string(),
            flags: vec!["security".to_string()],
            verdict: Some("contested".to_string()),
            notes: Some("checked by hand".to_string()),
            unavailable: vec!["crate".to_string()],
            ..Default::default()
        };
        let doc = json(&[row], "2026-10-28T00:00:00Z", false);
        assert!(doc.contains("\"unavailable\": [\"crate\"]"), "{doc}");
        let (verdicts, notes) = parse_previous(&doc);
        assert_eq!(
            verdicts.get("owner/repo").map(String::as_str),
            Some("contested")
        );
        assert_eq!(
            notes.get("owner/repo").map(String::as_str),
            Some("checked by hand")
        );
    }

    #[test]
    fn verify_update_replaces_a_claim_the_general_writer_wrote() {
        let row = SignalRow {
            repo: "a/b".into(),
            observed_at: "2026-10-31".into(),
            claim: Some(ClaimCheck {
                numbers_found: 9,
                numbers_total: 11,
                support: Some(0.96),
            }),
            ..Default::default()
        };
        let old = json(&[row], "2026-10-31T00:00:00Z", false);
        let doc = apply_claims(
            &old,
            &[(
                "a/b".into(),
                ClaimCheck {
                    numbers_found: 1,
                    numbers_total: 2,
                    support: None,
                },
            )],
        )
        .expect("apply succeeds");
        assert_eq!(
            doc.matches("\"claim\"").count(),
            1,
            "duplicate claim keys:\n{doc}"
        );
        assert!(doc.contains("\"numbers_found\": 1"), "{doc}");
        assert!(!doc.contains("\"numbers_found\": 9"), "{doc}");
        assert!(!doc.contains("0.96"), "stale support survived:\n{doc}");
    }

    #[test]
    fn claims_update_touches_only_the_named_repo() {
        let mk = |repo: &str, nf: usize, nt: usize| SignalRow {
            repo: repo.into(),
            observed_at: "2026-10-31".into(),
            claim: Some(ClaimCheck {
                numbers_found: nf,
                numbers_total: nt,
                support: None,
            }),
            ..Default::default()
        };
        let old = json(
            &[mk("a/aa", 1, 1), mk("b/bb", 2, 2)],
            "2026-10-31T00:00:00Z",
            false,
        );
        let doc = apply_claims(
            &old,
            &[(
                "b/bb".into(),
                ClaimCheck {
                    numbers_found: 7,
                    numbers_total: 9,
                    support: Some(0.5),
                },
            )],
        )
        .expect("apply succeeds");
        let a =
            &doc[doc.find("\"repo\": \"a/aa\"").unwrap()..doc.find("\"repo\": \"b/bb\"").unwrap()];
        assert!(
            a.contains("\"numbers_found\": 1"),
            "neighbor changed:\n{doc}"
        );
        assert!(
            !a.contains("\"numbers_found\": 7"),
            "check leaked into neighbor:\n{doc}"
        );
        assert!(doc.contains("\"support\": 0.50"), "{doc}");
    }

    #[test]
    fn partial_collection_carries_absent_repos() {
        let mk = |repo: &str, ts: &str, nf: usize, nt: usize| SignalRow {
            repo: repo.into(),
            observed_at: ts.into(),
            claim: Some(ClaimCheck {
                numbers_found: nf,
                numbers_total: nt,
                support: None,
            }),
            ..Default::default()
        };
        let full = json(
            &[
                mk("a/aa", "2026-10-30", 1, 1),
                mk("b/bb", "2026-10-30", 2, 2),
            ],
            "2026-10-30T00:00:00Z",
            false,
        );
        let partial = json(
            &[mk("a/aa", "2026-11-19", 1, 1)],
            "2026-11-19T00:00:00Z",
            false,
        );
        let merged = carry_absent(&partial, &full).expect("merge succeeds");
        assert!(
            merged.contains("\"repo\": \"b/bb\""),
            "absent repo dropped:\n{merged}"
        );
        assert!(merged.contains("2026-10-30"), "carried row lost its date");
        assert!(
            merged.contains("\"numbers_found\": 2"),
            "carried row lost its claim"
        );
        let fresh = &merged[merged.find("\"repo\": \"a/aa\"").unwrap()
            ..merged.find("\"repo\": \"b/bb\"").unwrap()];
        assert!(
            fresh.contains("2026-11-19"),
            "fetched row was overwritten by the stale one:\n{merged}"
        );
        assert!(
            merged.contains("\"repos\": 2"),
            "row count not updated:\n{merged}"
        );
    }

    #[test]
    fn previous_verdicts_survive() {
        let raw = "{\n  \"repos\": [\n    {\n      \"repo\": \"a/b\",\n      \"flags\": [],\n      \"verdict\": \"contested\",\n      \"notes\": \"checked by hand\"\n    }\n  ]\n}";
        let (v, n) = parse_previous(raw);
        assert_eq!(v.get("a/b").map(String::as_str), Some("contested"));
        assert_eq!(n.get("a/b").map(String::as_str), Some("checked by hand"));
    }
}
