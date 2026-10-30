//! SubQ adapter: typed yes/no decisions from a small model.
//!
//! One noul question per candidate, probability in, no prose out. The key is
//! read from SUBQ_API_KEY; without it the adapter reports "no model" and every
//! caller degrades to "no signal" rather than a guess.

use crate::adapters::net;
use crate::ports::signals::DecisionSource;
use crate::ports::verify::EntailmentJudge;

pub const MODEL: &str = "subq-snap-1.1";
const BASE: &str = "https://api.subq.ai/v1";

pub struct Subq {
    key: String,
    available: Option<bool>,
}

impl Subq {
    pub fn from_env() -> Subq {
        Subq {
            key: std::env::var("SUBQ_API_KEY").unwrap_or_default(),
            available: None,
        }
    }

    fn question(repo: &str, text: &str) -> String {
        let instructions = format!(
            "Does this comment contradict or dispute the performance claims of the project github.com/{repo} \
             - its own benchmark numbers, its methodology, or the speed it advertises?"
        );
        let yes = "The comment says this project is slower, wrong or misleading about its speed - its own numbers, \
                   its benchmark method, or results on another public benchmark that contradict what the project claims";
        let no = "The comment accepts the project's speed (for example \"it is fast, but...\") and criticises \
                  something else - its API, ecosystem, adoption or design - or only mentions the project in \
                  passing, or criticises a different project or topic";
        // Built by concatenation rather than format!: the brace count of a JSON
        // literal is easy to get wrong and hard to read.
        // Two narrow judgements, one request (the docs' advice: one judgement
        // per question, and let code combine them). The subject question exists
        // because a bare crate name collides: "fst" is a variable in a Python
        // snippet, "blaze" is a C++ library.
        let subject_i = format!(
            "Is this comment about the software project github.com/{repo} - not a different project, \
             and not an unrelated identifier that happens to share its name?"
        );
        let subject_t =
            "It refers to that project: its code, its releases, its benchmarks, its speed";
        let subject_f = "It refers to another project with a similar name, to a variable or type named the same, \
                         or to a general topic without naming this project";

        let mut body = String::from("{\"model\":\"");
        body.push_str(MODEL);
        body.push_str("\",\"state\":");
        body.push_str(&json_str(text));
        body.push_str(",\"questions\":{\"subject\":{\"type\":\"noul\",\"instructions\":");
        body.push_str(&json_str(&subject_i));
        body.push_str(",\"criteria\":{\"true\":");
        body.push_str(&json_str(subject_t));
        body.push_str(",\"false\":");
        body.push_str(&json_str(subject_f));
        body.push_str("}},\"critique\":{\"type\":\"noul\",\"instructions\":");
        body.push_str(&json_str(&instructions));
        body.push_str(",\"criteria\":{\"true\":");
        body.push_str(&json_str(yes));
        body.push_str(",\"false\":");
        body.push_str(&json_str(no));
        body.push_str("}}}}");
        body
    }
}

fn json_str(s: &str) -> String {
    let mut out = String::from("\"");
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
    out.push('"');
    out
}

impl EntailmentJudge for Subq {
    fn supports(&self, claim: &str, passage: &str) -> Option<f64> {
        if self.key.is_empty() {
            return None;
        }
        let instructions =
            "Does this passage support the specific claim: same metric, same baseline, \
                            same units - not merely a similar number?";
        let yes =
            "The passage states the same measurement the claim states, against the same baseline";
        let no =
            "The passage mentions a different metric, a different baseline, or a number that only \
                  looks similar";
        let mut body = String::from("{\"model\":\"");
        body.push_str(MODEL);
        body.push_str("\",\"state\":");
        body.push_str(&json_str(&format!("CLAIM: {claim}\n\nPASSAGE: {passage}")));
        body.push_str(",\"questions\":{\"supports\":{\"type\":\"noul\",\"instructions\":");
        body.push_str(&json_str(instructions));
        body.push_str(",\"criteria\":{\"true\":");
        body.push_str(&json_str(yes));
        body.push_str(",\"false\":");
        body.push_str(&json_str(no));
        body.push_str("}}}}");
        let (status, resp) = net::post(
            &format!("{BASE}/systemone"),
            &[("Authorization", &format!("Bearer {}", self.key))],
            &body,
        )?;
        if status != 200 {
            return None;
        }
        net::jq(&resp, ".answers.supports.noul").and_then(|v| v.trim().parse().ok())
    }
}

impl DecisionSource for Subq {
    fn model_available(&self) -> bool {
        if let Some(v) = self.available {
            return v;
        }
        let v = if self.key.is_empty() {
            false
        } else {
            net::get(
                &format!("{BASE}/models"),
                &[("Authorization", &format!("Bearer {}", self.key))],
            )
            .and_then(|b| net::jq(&b, ".data[].id"))
            .map(|ids| ids.lines().any(|id| id.trim() == MODEL))
            .unwrap_or(false)
        };
        // the cache is only ever read through &self, so recompute rather than mutate
        v
    }

    fn critique_p(&self, repo: &str, text: &str) -> Option<f64> {
        if self.key.is_empty() {
            return None;
        }
        let body = Self::question(repo, text);
        let (status, resp) = net::post(
            &format!("{BASE}/systemone"),
            &[("Authorization", &format!("Bearer {}", self.key))],
            &body,
        )?;
        if status != 200 {
            return None;
        }
        // Both gates must pass: the comment is about this project, and it
        // disputes its numbers. Multiplying them is the cheapest honest
        // conjunction - a name collision cannot be rescued by a confident
        // critique judgement.
        let subject: f64 =
            net::jq(&resp, ".answers.subject.noul").and_then(|v| v.trim().parse().ok())?;
        let critique: f64 =
            net::jq(&resp, ".answers.critique.noul").and_then(|v| v.trim().parse().ok())?;
        Some(subject * critique)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn question_embeds_the_text_escaped() {
        let q = Subq::question("apache/datafusion", "he said \"hi\" \n and left");
        assert!(q.contains("\"type\":\"noul\""));
        assert!(q.contains("apache/datafusion"));
        assert!(q.contains("\\\"hi\\\""), "quotes must be escaped: {q}");
        assert!(q.contains("\\n"), "newlines must be escaped: {q}");
    }
}
