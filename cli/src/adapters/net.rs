//! Network adapter: `curl` for transport and `jq` for extraction.
//!
//! Subprocesses, not crate dependencies - the same choice `gh_curl` makes for
//! the GitHub API, and the reason `abr` stays dependency-free. Both tools ship
//! on every runner the workflows use, and neither is asked to do more than
//! fetch bytes or pick fields.

use std::io::Write;
use std::process::{Command, Stdio};

const UA: &str = "abr-signals/1.0 (+https://github.com/rager306/awesome-blazing-rust)";

fn run(mut cmd: Command, stdin: Option<&str>) -> Option<String> {
    let mut child = cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().ok()?;
    if let Some(input) = stdin {
        child.stdin.as_mut()?.write_all(input.as_bytes()).ok()?;
    }
    drop(child.stdin.take());
    let out = child.wait_with_output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).to_string())
}

/// GET a URL. `None` on a transport error or a non-2xx status.
pub fn get(url: &str, headers: &[(&str, &str)]) -> Option<String> {
    let mut cmd = Command::new("curl");
    cmd.args(["-sfL", "-m", "25", "-A", UA]);
    for (k, v) in headers {
        cmd.args(["-H", &format!("{k}: {v}")]);
    }
    cmd.arg(url);
    run(cmd, None)
}

/// POST a JSON body. Returns (status, body): 402 is a state the caller must
/// act on (out of credits), not a transport failure.
pub fn post(url: &str, headers: &[(&str, &str)], body: &str) -> Option<(u16, String)> {
    let mut cmd = Command::new("curl");
    cmd.args(["-sL", "-m", "30", "-A", UA, "-X", "POST", "-H", "Content-Type: application/json"]);
    for (k, v) in headers {
        cmd.args(["-H", &format!("{k}: {v}")]);
    }
    cmd.args(["--data-binary", "@-", "-w", "\n%{http_code}"]);
    cmd.arg(url);
    let out = run(cmd, Some(body))?;
    let (body, code) = out.rsplit_once('\n')?;
    Some((code.trim().parse().unwrap_or(0), body.to_string()))
}

/// Run a jq filter over a JSON document.
pub fn jq(doc: &str, filter: &str) -> Option<String> {
    let mut cmd = Command::new("jq");
    cmd.args(["-r", filter]);
    run(cmd, Some(doc))
}

/// `a\tb\nc` -> [a, b, c], `\t` and `\n` inside fields unescaped.
pub fn tsv_lines(s: &str) -> Vec<Vec<String>> {
    s.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| line.split('\t').map(unescape_tsv).collect())
        .collect()
}

fn unescape_tsv(field: &str) -> String {
    let mut out = String::with_capacity(field.len());
    let mut chars = field.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('t') => out.push('\t'),
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tsv_unescapes_tabs_and_newlines() {
        let rows = tsv_lines("1\t2026-01-01\ta\\tb\\nc\n");
        assert_eq!(rows[0][2], "a\tb\nc");
    }

    #[test]
    fn tsv_skips_blank_lines() {
        assert_eq!(tsv_lines("\n\n").len(), 0);
    }
}
