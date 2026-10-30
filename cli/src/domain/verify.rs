//! Provenance check for claim numbers: does the number a card states appear in
//! the page the card cites?
//!
//! This is the mechanical half of verification. It answers "is this figure
//! present at the source", never "does the source support the claim" - those
//! are separate jobs (AI Citation Verification, 2026). A pass here means the
//! number was not invented or transposed; it does not mean it was read
//! correctly. Entailment needs a reader, and a reader is not deterministic.
//!
//! Deliberately not a gate: tables rendered as images, PDFs and values written
//! in prose ("two and a half") all fail the text match without being wrong.

/// Numbers worth checking: at least two digits, normalized so `1,879` and
/// `1879` and `1879.0` are the same token, and `10×` matches `10x`.
pub fn numbers(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if !chars[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let start = i;
        let mut buf = String::new();
        let mut digits = 0;
        while i < chars.len() {
            let c = chars[i];
            if c.is_ascii_digit() {
                buf.push(c);
                digits += 1;
                i += 1;
            } else if (c == ',' || c == '_' || c == '\u{202f}') && digits > 0 {
                // A thousands separator, only if a digit follows. A space is
                // deliberately NOT one: "version 3. 28.7 ns" must not become
                // 328.7, and English prose separates numbers with spaces.
                if chars.get(i + 1).is_some_and(|n| n.is_ascii_digit()) {
                    i += 1;
                } else {
                    break;
                }
            } else if c == '.' && chars.get(i + 1).is_some_and(|n| n.is_ascii_digit()) {
                buf.push('.');
                i += 1;
            } else {
                break;
            }
        }
        if digits >= 2 && i > start {
            let mut token = buf.trim_end_matches('.').to_string();
            while token.ends_with('0') && token.contains('.') {
                token.pop();
            }
            token = token.trim_end_matches('.').to_string();
            // "1.0" normalises to "1": too weak to verify anything, and it
            // matches every page that happens to contain a one.
            if token.chars().filter(|c| c.is_ascii_digit()).count() >= 2 && !token.is_empty() {
                out.push(token);
            }
        } else if i == start {
            i += 1;
        }
    }
    out
}

/// (present, missing) for every checkable number in the claim.
pub fn coverage(claim: &str, page: &str) -> (Vec<String>, Vec<String>) {
    let have = numbers(page);
    let mut present = Vec::new();
    let mut missing = Vec::new();
    for n in numbers(claim) {
        if have.contains(&n) {
            present.push(n);
        } else {
            missing.push(n);
        }
    }
    present.dedup();
    missing.dedup();
    (present, missing)
}

/// The first link in a card's claim: the methodology the number should live in.
pub fn first_link(claim: &str) -> Option<&str> {
    let open = claim.find("](")?;
    let rest = &claim[open + 2..];
    let close = rest.find(')')?;
    let url = &rest[..close];
    url.starts_with("http").then_some(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_normalise_thousands_and_trailing_zeros() {
        assert_eq!(
            numbers("1,879 ms and 1879 and 12.50"),
            vec!["1879", "1879", "12.5"]
        );
        assert_eq!(numbers("2.5 ms"), vec!["2.5"]);
        // a space is not a thousands separator: "3. 28.7" is two tokens
        assert_eq!(numbers("version 3. 28.7 ns"), vec!["28.7"]);
    }

    #[test]
    fn single_digit_tokens_are_ignored() {
        assert!(numbers("median of 7 runs, 5 MB").is_empty());
        // "1.0" collapses to "1" and must not survive as a checkable token
        assert!(numbers("Oxlint 1.0 table").is_empty());
    }

    #[test]
    fn coverage_splits_present_from_missing() {
        let (p, m) = coverage(
            "2.5 ms vs 1879 ms for 755x",
            "HashSet 1879 ms; roaring 2.5 ms",
        );
        assert!(p.contains(&"2.5".to_string()) && p.contains(&"1879".to_string()));
        assert!(m.contains(&"755".to_string()));
    }

    #[test]
    fn link_extraction_takes_the_first_url() {
        assert_eq!(
            first_link("see [run](https://example.com/r) and [x](https://example.com/y)"),
            Some("https://example.com/r")
        );
        assert_eq!(first_link("no link here"), None);
    }
}
