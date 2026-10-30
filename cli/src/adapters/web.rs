//! The signals source over the public, keyless APIs: crates.io, HN Algolia,
//! RUSTSEC's advisory-db, and GitHub through the authenticated `gh` CLI.

use crate::adapters::net;
use crate::ports::signals::{Candidate, CrateInfo, Health, MentionCounts, SignalsSource};
use std::cell::RefCell;

pub struct WebSignals {
    /// Cache lives in the adapter: the port has no reason to demand &mut.
    rustsec_index: RefCell<Option<Vec<String>>>,
}

impl WebSignals {
    pub fn new() -> WebSignals {
        WebSignals {
            rustsec_index: RefCell::new(None),
        }
    }

    fn rustsec_index(&self) -> std::cell::Ref<'_, Option<Vec<String>>> {
        if self.rustsec_index.borrow().is_none() {
            let loaded = net::get(
                "https://api.github.com/repos/rustsec/advisory-db/contents/crates",
                &[("Accept", "application/vnd.github+json")],
            )
            .and_then(|body| net::jq(&body, ".[].name"))
            .map(|names| names.lines().map(str::to_string).collect());
            *self.rustsec_index.borrow_mut() = loaded;
        }
        self.rustsec_index.borrow()
    }
}

fn norm_repo(url: &str) -> String {
    let u = url
        .trim()
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .to_lowercase();
    u.replace("https://", "")
        .replace("http://", "")
        .replace("www.", "")
}

impl SignalsSource for WebSignals {
    fn health(&self, repo: &str) -> Option<Health> {
        let out = std::process::Command::new("gh")
            .args([
                "api",
                &format!("repos/{repo}"),
                "--jq",
                "[.archived, .pushed_at, .stargazers_count, .license.spdx_id, .created_at] | @tsv",
            ])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&out.stdout);
        let f: Vec<&str> = text.trim().split('\t').collect();
        if f.len() < 4 {
            return None;
        }
        Some(Health {
            archived: match f[0] {
                "true" => Some(true),
                "false" => Some(false),
                _ => None,
            },
            pushed_at: (f[1] != "null").then(|| f[1].to_string()),
            stars: f[2].parse().ok(),
            license: (f[3] != "null").then(|| f[3].to_string()),
            created_at: f.get(4).filter(|v| **v != "null").map(|v| v.to_string()),
        })
    }

    fn crate_info(&self, name: &str, repo: &str) -> Option<CrateInfo> {
        // crates.io asks for at most one request per second.
        std::thread::sleep(std::time::Duration::from_millis(1050));
        let body = net::get(&format!("https://crates.io/api/v1/crates/{name}"), &[])?;
        let filter =
            "[.crate.name, .crate.downloads, .crate.recent_downloads, .crate.num_versions, \
                      .crate.yanked, .crate.updated_at, .crate.repository] | @tsv";
        let row = net::jq(&body, filter)?;
        let f: Vec<String> = net::tsv_lines(&row).into_iter().next()?;
        if f.len() < 7 {
            return None;
        }
        // A name collision must not attach someone else's downloads.
        if norm_repo(&f[6]) != norm_repo(&format!("https://github.com/{repo}")) {
            return None;
        }
        Some(CrateInfo {
            name: f[0].clone(),
            downloads: f[1].parse().unwrap_or(0),
            recent_downloads: f[2].parse().unwrap_or(0),
            versions: f[3].parse().unwrap_or(0),
            yanked: f[4].parse().unwrap_or(0),
            updated_at: f[5].clone(),
        })
    }

    fn attention(&self, name: &str) -> MentionCounts {
        let q = urlencode(&format!("\"{name}\""));
        let count = |tags: &str| -> u64 {
            net::get(
                &format!(
                    "https://hn.algolia.com/api/v1/search?query={q}&tags={tags}&hitsPerPage=1"
                ),
                &[],
            )
            .and_then(|b| net::jq(&b, ".nbHits"))
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(0)
        };
        MentionCounts {
            stories: count("story"),
            comments: count("comment"),
        }
    }

    fn advisories(&self, name: &str) -> Vec<String> {
        let indexed = self
            .rustsec_index()
            .as_ref()
            .map(|v| v.iter().any(|n| n == name))
            .unwrap_or(false);
        if !indexed {
            return Vec::new();
        }
        net::get(
            &format!("https://api.github.com/repos/rustsec/advisory-db/contents/crates/{name}"),
            &[("Accept", "application/vnd.github+json")],
        )
        .and_then(|b| net::jq(&b, ".[].name"))
        .map(|names| {
            names
                .lines()
                .filter_map(|n| n.strip_suffix(".md").map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
    }

    fn dispute_candidates(&self, name: &str, repo: &str) -> Vec<Candidate> {
        let q = urlencode(&format!("\"{name}\" benchmark"));
        let Some(body) = net::get(
            &format!("https://hn.algolia.com/api/v1/search?query={q}&tags=comment&hitsPerPage=20"),
            &[],
        ) else {
            return Vec::new();
        };
        let Some(rows) = net::jq(
            &body,
            ".hits[] | [.objectID, .created_at, .comment_text] | @tsv",
        ) else {
            return Vec::new();
        };
        let slug = repo.to_lowercase();
        let mut out = Vec::new();
        for f in net::tsv_lines(&rows) {
            if f.len() < 3 {
                continue;
            }
            let (id, created, text) = (&f[0], &f[1], &f[2]);
            let low = text.to_lowercase();
            if !low.contains(&slug) && !low.contains(&name.to_lowercase()) {
                continue;
            }
            if created.get(..4).unwrap_or("0000") < "2020" {
                continue;
            }
            if !["bench", "fast", "speed", "perf", "slower", "faster"]
                .iter()
                .any(|w| low.contains(w))
            {
                continue;
            }
            out.push(Candidate {
                source: "hn".to_string(),
                url: format!("https://news.ycombinator.com/item?id={id}"),
                created_at: created.clone(),
                text: text.clone(),
            });
            if out.len() >= 3 {
                break;
            }
        }
        out
    }
}

fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_normalisation_ignores_case_and_suffixes() {
        assert_eq!(
            norm_repo("https://github.com/Apache/Datafusion/"),
            "github.com/apache/datafusion"
        );
        assert_eq!(
            norm_repo("https://github.com/apache/datafusion.git"),
            "github.com/apache/datafusion"
        );
    }

    #[test]
    fn url_encode_handles_quotes() {
        assert_eq!(urlencode("\"a b\""), "%22a%20b%22");
    }
}
