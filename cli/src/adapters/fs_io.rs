//! Filesystem adapters implementing the Source and Sink ports.
//!
//! Paths are always relative to the repository root. The root is discovered
//! by walking up from the current directory until a directory containing
//! both README.md and PATTERNS.md is found - so `abr` works from anywhere
//! inside the repository, and no absolute path exists anywhere in the code.

use crate::ports::{sink::Sink, source::Source};
use std::fs;
#[cfg(test)]
use std::path::Path;
use std::path::PathBuf;
use std::process::exit;

pub struct FsIo {
    root: PathBuf,
}

impl FsIo {
    pub fn discover() -> FsIo {
        let mut dir = std::env::current_dir().unwrap_or_else(|e| {
            eprintln!("abr: cannot determine current directory: {e}");
            exit(1);
        });
        loop {
            if dir.join("README.md").is_file() && dir.join("PATTERNS.md").is_file() {
                return FsIo { root: dir };
            }
            if !dir.pop() {
                eprintln!("abr: repository root not found (walked up from cwd, no README.md + PATTERNS.md pair)");
                exit(1);
            }
        }
    }

    fn resolve(&self, path: &str) -> PathBuf {
        self.root.join(path)
    }

    #[cfg(test)]
    pub fn root(&self) -> &Path {
        &self.root
    }
}

impl Source for FsIo {
    fn read(&self, path: &str) -> String {
        let p = self.resolve(path);
        fs::read_to_string(&p).unwrap_or_else(|e| {
            eprintln!("abr: cannot read {} (root {}): {e}", p.display(), self.root.display());
            exit(1);
        })
    }

    fn read_opt(&self, path: &str) -> Option<String> {
        fs::read_to_string(self.resolve(path)).ok()
    }
}

impl Sink for FsIo {
    fn write(&self, path: &str, content: &str) {
        let p = self.resolve(path);
        if let Some(parent) = p.parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(&p, content).unwrap_or_else(|e| {
            eprintln!("abr: cannot write {}: {e}", p.display());
            exit(1);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discover_finds_repo_root_from_subdirectory() {
        // tests run with cwd = cli/, so the walk-up must find the repo root
        let io = FsIo::discover();
        assert!(io.root().join("README.md").is_file());
        assert!(io.root().join("PATTERNS.md").is_file());
        assert!(io.read_opt("catalog/projects.json").is_some());
    }
}
