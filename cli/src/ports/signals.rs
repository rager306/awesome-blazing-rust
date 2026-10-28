//! Signals port: where third-party context about a repository comes from, and
//! where a judgement about it is asked for. The domain never touches the net.

pub struct Health {
    pub archived: Option<bool>,
    pub pushed_at: Option<String>,
    pub created_at: Option<String>,
    pub stars: Option<u64>,
    pub license: Option<String>,
}

pub struct CrateInfo {
    pub name: String,
    pub downloads: u64,
    pub recent_downloads: u64,
    pub versions: u64,
    pub yanked: u64,
    pub updated_at: String,
}

pub struct MentionCounts {
    pub stories: u64,
    pub comments: u64,
}

/// A keyword hit: cheap recall, deliberately over-inclusive.
pub struct Candidate {
    pub source: String,
    pub url: String,
    pub created_at: String,
    pub text: String,
}

pub trait SignalsSource {
    fn health(&self, repo: &str) -> Option<Health>;
    /// `name` is the crate to look up, `repo` the repository it must belong to.
    fn crate_info(&self, name: &str, repo: &str) -> Option<CrateInfo>;
    fn attention(&self, name: &str) -> MentionCounts;
    fn advisories(&mut self, name: &str) -> Vec<String>;
    fn dispute_candidates(&self, name: &str, repo: &str) -> Vec<Candidate>;
}

/// A typed-decision service. `None` means "no judgement available" - never a
/// guess, because a guessed signal is worse than a missing one.
pub trait DecisionSource {
    fn model_available(&self) -> bool;
    fn critique_p(&self, repo: &str, text: &str) -> Option<f64>;
}
