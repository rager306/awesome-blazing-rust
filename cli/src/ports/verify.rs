//! Page port: the text behind a claim's link.

pub trait PageSource {
    /// Page text with markup stripped, or `None` when it cannot be fetched.
    fn page(&self, url: &str) -> Option<String>;
}

/// A judgement, not a measurement: "does this passage support this claim".
///
/// Kept apart from `PageSource` because it is non-deterministic. Nothing in the
/// gate may depend on it; the report shows it next to the mechanical result so
/// a reader can see where the two disagree.
pub trait EntailmentJudge {
    fn supports(&self, claim: &str, passage: &str) -> Option<f64>;
}
