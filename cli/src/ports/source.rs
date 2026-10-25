//! Source port: raw document texts for fixed well-known paths.

pub trait Source {
    /// Must return the file content, or exit with a clear error (boot input).
    fn read(&self, path: &str) -> String;
    /// Optional input: absence is a domain fact (e.g. missing catalog).
    fn read_opt(&self, path: &str) -> Option<String>;
}
