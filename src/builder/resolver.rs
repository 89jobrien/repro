//! Runtime discovery abstractions (ports) and adapters.

use anyhow::{Context, Result};
use std::path::PathBuf;

/// Resolved runtime: name + absolute path to the binary.
pub struct ResolvedRuntime {
    pub name: String,
    pub path: PathBuf,
}

/// Resolve a runtime name to an absolute binary path.
pub trait RuntimeResolver: Send + Sync {
    fn resolve(&self, name: &str) -> Result<ResolvedRuntime>;
}

/// Resolves runtimes by looking up binaries on PATH via `which`.
pub struct WhichResolver;

impl RuntimeResolver for WhichResolver {
    fn resolve(&self, name: &str) -> Result<ResolvedRuntime> {
        let path = which::which(name).with_context(|| format!("{name} not found on PATH"))?;
        Ok(ResolvedRuntime {
            name: name.to_string(),
            path,
        })
    }
}

/// Resolver that returns a fixed path for any runtime name.
#[cfg(any(test, feature = "testutil"))]
pub struct MockResolver {
    pub path: PathBuf,
}

#[cfg(any(test, feature = "testutil"))]
impl MockResolver {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

#[cfg(any(test, feature = "testutil"))]
impl RuntimeResolver for MockResolver {
    fn resolve(&self, name: &str) -> Result<ResolvedRuntime> {
        Ok(ResolvedRuntime {
            name: name.to_string(),
            path: self.path.clone(),
        })
    }
}
