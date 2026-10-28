//! Page adapter: fetch a URL and reduce it to text.
//!
//! `curl` for transport, then a tag-stripper - no parser crate, and no attempt
//! to render tables from images, which is exactly why the check is advisory.

use crate::adapters::net;
use crate::ports::verify::PageSource;

pub struct WebPage;

impl PageSource for WebPage {
    fn page(&self, url: &str) -> Option<String> {
        let body = net::get(url, &[])?;
        Some(to_text(&body))
    }
}

/// Drop tags, decode the five entities that matter, collapse whitespace.
pub fn to_text(html: &str) -> String {
    let mut out = String::with_capacity(html.len() / 2);
    let mut in_tag = false;
    let mut in_script = false;
    let lower = html.to_lowercase();
    let mut i = 0;
    let bytes: Vec<char> = html.chars().collect();
    let _ = &lower;
    while i < bytes.len() {
        let c = bytes[i];
        if !in_tag && c == '<' {
            // skip <script>/<style> bodies entirely
            let rest: String = bytes[i..bytes.len().min(i + 8)].iter().collect();
            let r = rest.to_lowercase();
            if r.starts_with("<script") || r.starts_with("<style") {
                in_script = true;
            }
            in_tag = true;
            i += 1;
            continue;
        }
        if in_tag {
            if c == '>' {
                in_tag = false;
                if in_script {
                    // find the closing tag
                    let rest: String = bytes[i..bytes.len().min(i + 10)].iter().collect();
                    if rest.to_lowercase().contains("/script") || rest.to_lowercase().contains("/style") {
                        in_script = false;
                    }
                } else {
                    out.push(' ');
                }
            } else if in_script {
                let rest: String = bytes[i..bytes.len().min(i + 10)].iter().collect();
                if rest.to_lowercase().starts_with("/script") || rest.to_lowercase().starts_with("/style") {
                    in_script = false;
                    in_tag = false;
                }
            }
            i += 1;
            continue;
        }
        if in_script {
            i += 1;
            continue;
        }
        match c {
            '&' => {
                let rest: String = bytes[i..bytes.len().min(i + 8)].iter().collect();
                if let Some((enc, len)) = decode_entity(&rest) {
                    out.push(enc);
                    i += len;
                    continue;
                }
                out.push(c);
            }
            c if c.is_whitespace() => out.push(' '),
            c => out.push(c),
        }
        i += 1;
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn decode_entity(rest: &str) -> Option<(char, usize)> {
    for (name, ch) in [("&amp;", '&'), ("&lt;", '<'), ("&gt;", '>'), ("&quot;", '"'), ("&#39;", '\'')] {
        if rest.starts_with(name) {
            return Some((ch, name.len()));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_tags_and_collapses_space() {
        assert_eq!(to_text("<p>2,5&nbsp;ms</p><b>1879</b>"), "2,5&nbsp;ms 1879");
    }

    #[test]
    fn skips_script_bodies() {
        let t = to_text("<script>var x = 42;</script><p>7 ms</p>");
        assert!(!t.contains("42"), "{t}");
        assert!(t.contains("7 ms"));
    }
}
