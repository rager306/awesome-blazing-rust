//! Upstream domain: pure slug extraction, projection, and diffing for the
//! weekly upstream scan. No I/O - the driver supplies fetched rows.

/// Extract `owner/repo` from a github.com URL; None for non-repo links.
pub fn repo_slug(url: &str) -> Option<String> {
    let rest = url.strip_prefix("https://github.com/")?;
    let mut parts = rest.split('/');
    let owner = parts.next()?;
    let repo = parts.next()?;
    if owner.is_empty() || repo.is_empty() || repo == "blob" && parts.next().is_none() {
        return None;
    }
    Some(format!("{owner}/{repo}"))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpstreamRow {
    pub repo: String,
    pub tag: String,
}

/// Deterministic projection: one-line objects, sorted by repo, valid JSON
/// (no trailing comma). The strict format is the parser's contract for the
/// previous-state scan.
pub fn json(rows: &[UpstreamRow]) -> String {
    let mut sorted: Vec<&UpstreamRow> = rows.iter().collect();
    sorted.sort_by(|a, b| a.repo.cmp(&b.repo));
    let mut out = String::from("[\n");
    for (i, r) in sorted.iter().enumerate() {
        let comma = if i + 1 == sorted.len() { "" } else { "," };
        out.push_str(&format!(
            "  {{\"repo\": \"{}\", \"tag\": \"{}\"}}{}\n",
            r.repo, r.tag, comma
        ));
    }
    out.push_str("]\n");
    out
}

/// Tolerant scan of the exact format `json` emits (nothing else).
pub fn parse_prev(raw: &str) -> Vec<UpstreamRow> {
    let mut rows = Vec::new();
    for line in raw.lines() {
        let line = line.trim().trim_end_matches(',');
        if !line.starts_with("{\"repo\": \"") {
            continue;
        }
        let inner = line
            .trim_start_matches("{\"repo\": \"")
            .trim_end_matches('}');
        if let Some((repo, tag)) = inner.split_once("\", \"tag\": \"") {
            rows.push(UpstreamRow {
                repo: repo.to_string(),
                tag: tag.trim_end_matches('"').to_string(),
            });
        }
    }
    rows
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Diff {
    pub changed: Vec<String>, // "repo: old -> new" or "repo: new (first scan)"
    pub gone: Vec<String>,    // previously scanned, now unfetchable
}

/// Compare fetched rows against the previous projection (pure).
pub fn diff(prev_raw: Option<&str>, rows: &[UpstreamRow]) -> Diff {
    let prev = prev_raw.map(parse_prev).unwrap_or_default();
    let prev_map: std::collections::BTreeMap<&str, &str> = prev
        .iter()
        .map(|r| (r.repo.as_str(), r.tag.as_str()))
        .collect();
    let now_map: std::collections::BTreeMap<&str, &str> = rows
        .iter()
        .map(|r| (r.repo.as_str(), r.tag.as_str()))
        .collect();

    let mut d = Diff::default();
    for (repo, tag) in &now_map {
        match prev_map.get(*repo) {
            None => d.changed.push(format!("{repo}: {tag} (first scan)")),
            Some(old) if *old != *tag => d.changed.push(format!("{repo}: {old} -> {tag}")),
            Some(_) => {}
        }
    }
    for repo in prev_map.keys() {
        if !now_map.contains_key(repo) {
            d.gone.push(repo.to_string());
        }
    }
    d
}

/// Per-repo scan outcome, decoupled from the ports layer: the driver maps
/// TagState::Unreachable to Unreachable, Tag/NoTags to Ok.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Ok,
    Unreachable,
}

/// Consecutive-failure ledger: repo -> strikes (0 = healthy/observed).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Strike(pub std::collections::BTreeMap<String, u32>);

/// Success resets a repo's streak, Unreachable increments it. Repos new to
/// the ledger start at 0 (a first scan is not a strike).
pub fn apply(strike: &Strike, outcomes: &[(String, Outcome)]) -> Strike {
    let mut next = Strike(strike.0.clone());
    for (repo, outcome) in outcomes {
        let n = next.0.entry(repo.clone()).or_insert(0);
        match outcome {
            Outcome::Ok => *n = 0,
            Outcome::Unreachable => *n += 1,
        }
    }
    next
}

/// Circuit breaker: more than 3% of scanned repos unreachable in one run
/// means something upstream broke - the run is an incident, not a signal.
pub fn incident(unreachable: usize, scanned: usize) -> bool {
    scanned > 0 && (unreachable as f64) / (scanned as f64) > 0.03
}

/// Repos at or past the enforcement threshold (phase 2 removes these).
pub fn tripwire(strike: &Strike) -> Vec<String> {
    strike
        .0
        .iter()
        .filter(|(_, n)| **n >= 3)
        .map(|(r, _)| r.clone())
        .collect()
}

/// Deterministic one-line-object projection (same contract style as `json`).
pub fn strike_json(strike: &Strike) -> String {
    let mut out = String::from("[\n");
    for (i, (repo, n)) in strike.0.iter().enumerate() {
        let comma = if i + 1 == strike.0.len() { "" } else { "," };
        out.push_str(&format!(
            "  {{\"repo\": \"{repo}\", \"strikes\": {n}}}{comma}\n"
        ));
    }
    out.push_str("]\n");
    out
}

/// Tolerant scan of the exact format `strike_json` emits.
pub fn parse_strike(raw: &str) -> Strike {
    let mut s = Strike::default();
    for line in raw.lines() {
        let line = line.trim().trim_end_matches(',');
        if !line.starts_with("{\"repo\": \"") {
            continue;
        }
        let inner = line
            .trim_start_matches("{\"repo\": \"")
            .trim_end_matches('}');
        if let Some((repo, n)) = inner.split_once("\", \"strikes\": ") {
            if let Ok(n) = n.parse::<u32>() {
                s.0.insert(repo.to_string(), n);
            }
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_extracts_owner_repo() {
        assert_eq!(
            repo_slug("https://github.com/RoaringBitmap/roaring-rs"),
            Some("RoaringBitmap/roaring-rs".into())
        );
        assert_eq!(
            repo_slug("https://github.com/o/r/blob/main/x.md"),
            Some("o/r".into())
        );
        assert_eq!(repo_slug("https://rust-lang.org"), None);
    }

    #[test]
    fn json_roundtrip() {
        let rows = vec![
            UpstreamRow {
                repo: "b/c".into(),
                tag: "v2".into(),
            },
            UpstreamRow {
                repo: "a/b".into(),
                tag: "v1".into(),
            },
        ];
        let text = json(&rows);
        assert!(text.find("\"repo\": \"a/b\"").unwrap() < text.find("\"repo\": \"b/c\"").unwrap());
        let back = parse_prev(&text);
        assert_eq!(back.len(), 2);
        assert_eq!(back[0].repo, "a/b");
        assert_eq!(back[0].tag, "v1");
    }

    #[test]
    fn diff_reports_changed_new_gone() {
        let prev = json(&[
            UpstreamRow {
                repo: "a/b".into(),
                tag: "v1".into(),
            },
            UpstreamRow {
                repo: "x/y".into(),
                tag: "v9".into(),
            },
        ]);
        let now = vec![
            UpstreamRow {
                repo: "a/b".into(),
                tag: "v2".into(),
            },
            UpstreamRow {
                repo: "n/m".into(),
                tag: "v0".into(),
            },
        ];
        let d = diff(Some(&prev), &now);
        assert_eq!(
            d.changed,
            vec![
                "a/b: v1 -> v2".to_string(),
                "n/m: v0 (first scan)".to_string()
            ]
        );
        assert_eq!(d.gone, vec!["x/y".to_string()]);
    }

    #[test]
    fn strike_increments_on_unreachable_and_resets_on_success() {
        let mut s = Strike::default();
        let out = |unreachable: bool| {
            vec![(
                "a/b".to_string(),
                if unreachable {
                    Outcome::Unreachable
                } else {
                    Outcome::Ok
                },
            )]
        };
        s = apply(&s, &out(true));
        s = apply(&s, &out(true));
        assert_eq!(s.0.get("a/b"), Some(&2));
        s = apply(&s, &out(false));
        assert_eq!(s.0.get("a/b"), Some(&0));
    }

    #[test]
    fn strike_tracks_repos_independently() {
        let mut s = Strike::default();
        s = apply(
            &s,
            &[
                ("a/b".to_string(), Outcome::Unreachable),
                ("c/d".to_string(), Outcome::Ok),
            ],
        );
        s = apply(
            &s,
            &[
                ("a/b".to_string(), Outcome::Unreachable),
                ("c/d".to_string(), Outcome::Ok),
            ],
        );
        assert_eq!(s.0.get("a/b"), Some(&2));
        assert_eq!(s.0.get("c/d"), Some(&0));
    }

    #[test]
    fn incident_trips_only_above_three_percent() {
        // 1 of 30 = 3.3% > 3%: incident.
        assert!(incident(1, 30));
        // 1 of 40 = 2.5%: no incident.
        assert!(!incident(1, 40));
        // 0 of 0: no incident (nothing scanned, nothing to conclude).
        assert!(!incident(0, 0));
        // All reachable: no incident.
        assert!(!incident(0, 1));
    }

    #[test]
    fn tripwire_reports_three_strikes() {
        let mut s = Strike::default();
        for _ in 0..3 {
            s = apply(
                &s,
                &[
                    ("a/b".to_string(), Outcome::Unreachable),
                    ("c/d".to_string(), Outcome::Ok),
                ],
            );
        }
        assert_eq!(tripwire(&s), vec!["a/b".to_string()]);
    }

    #[test]
    fn strike_state_roundtrips_through_lines() {
        let mut s = Strike::default();
        s = apply(
            &s,
            &[
                ("a/b".to_string(), Outcome::Unreachable),
                ("c/d".to_string(), Outcome::Ok),
            ],
        );
        s = apply(
            &s,
            &[
                ("a/b".to_string(), Outcome::Unreachable),
                ("c/d".to_string(), Outcome::Ok),
            ],
        );
        let text = strike_json(&s);
        let back = parse_strike(&text);
        assert_eq!(back.0.get("a/b"), Some(&2));
        assert_eq!(back.0.get("c/d"), Some(&0));
    }
}
