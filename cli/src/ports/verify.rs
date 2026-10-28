//! Page port: the text behind a claim's link.

pub trait PageSource {
    /// Page text with markup stripped, or `None` when it cannot be fetched.
    fn page(&self, url: &str) -> Option<String>;
}
