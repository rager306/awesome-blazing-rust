//! GitHub adapter: resolves the latest published tag via the authenticated
//! `gh` CLI (releases/latest with a tags fallback). A subprocess, not a
//! crate dependency - the zero-dependency ethos stays intact.

use crate::ports::upstream::{TagState, UpstreamSource};

pub struct GhCli;

impl GhCli {
    fn first_tag(&self, path: &str, jq: &str) -> Option<Result<String, bool>> {
        let out = std::process::Command::new("gh")
            .args(["api", path, "--jq", jq])
            .output()
            .ok()?;
        Some(if out.status.success() {
            let tag = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if tag.is_empty() {
                Err(false)
            } else {
                Ok(tag)
            }
        } else {
            Err(true) // HTTP error (404 / API failure)
        })
    }
}

impl UpstreamSource for GhCli {
    fn latest_tag(&self, repo: &str) -> TagState {
        // releases/latest: 200+tag | 404 (no releases OR no repo)
        match self.first_tag(&format!("repos/{repo}/releases/latest"), ".tag_name") {
            Some(Ok(tag)) => return TagState::Tag(tag),
            Some(Err(false)) => return TagState::NoTags, // 200, no releases
            _ => {}
        }
        // tags fallback: distinguishes "repo exists, no tags" from 404
        match self.first_tag(&format!("repos/{repo}/tags"), ".[0].name") {
            Some(Ok(tag)) => TagState::Tag(tag),
            Some(Err(false)) => TagState::NoTags,
            Some(Err(true)) => TagState::Unreachable,
            None => TagState::Unreachable,
        }
    }
}
