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
        out.push_str(&format!("  {{\"repo\": \"{}\", \"tag\": \"{}\"}}{}\n", r.repo, r.tag, comma));
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
        let inner = line.trim_start_matches("{\"repo\": \"").trim_end_matches('}');
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
    let prev_map: std::collections::BTreeMap<&str, &str> =
        prev.iter().map(|r| (r.repo.as_str(), r.tag.as_str())).collect();
    let now_map: std::collections::BTreeMap<&str, &str> =
        rows.iter().map(|r| (r.repo.as_str(), r.tag.as_str())).collect();

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_extracts_owner_repo() {
        assert_eq!(repo_slug("https://github.com/RoaringBitmap/roaring-rs"), Some("RoaringBitmap/roaring-rs".into()));
        assert_eq!(repo_slug("https://github.com/o/r/blob/main/x.md"), Some("o/r".into()));
        assert_eq!(repo_slug("https://rust-lang.org"), None);
    }

    #[test]
    fn json_roundtrip() {
        let rows = vec![
            UpstreamRow { repo: "b/c".into(), tag: "v2".into() },
            UpstreamRow { repo: "a/b".into(), tag: "v1".into() },
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
            UpstreamRow { repo: "a/b".into(), tag: "v1".into() },
            UpstreamRow { repo: "x/y".into(), tag: "v9".into() },
        ]);
        let now = vec![
            UpstreamRow { repo: "a/b".into(), tag: "v2".into() },
            UpstreamRow { repo: "n/m".into(), tag: "v0".into() },
        ];
        let d = diff(Some(&prev), &now);
        assert_eq!(d.changed, vec!["a/b: v1 -> v2".to_string(), "n/m: v0 (first scan)".to_string()]);
        assert_eq!(d.gone, vec!["x/y".to_string()]);
    }
}
