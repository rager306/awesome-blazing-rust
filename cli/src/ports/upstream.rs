//! Upstream port: the latest published version of a cataloged repository.

/// Outcome of an upstream lookup. `NoTags` is a legitimate stable state
/// (repos without releases/tags must not churn the diff every week);
/// `Unreachable` (404, API failure) is the signal worth attention.
pub enum TagState {
    Tag(String),
    NoTags,
    Unreachable,
}

pub trait UpstreamSource {
    fn latest_tag(&self, repo: &str) -> TagState;
}
