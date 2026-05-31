//! Contract tests for RuntimeResolver port.

use repro::builder::{MockResolver, RuntimeResolver, WhichResolver};
use std::path::PathBuf;

// --- C2.1: resolve() returns matching name ---

#[test]
fn c2_1_mock_resolver_returns_requested_name() {
    let resolver = MockResolver::new(PathBuf::from("/usr/bin/docker"));
    let result = resolver.resolve("docker").unwrap();
    assert_eq!(result.name, "docker");
}

#[test]
fn c2_1_mock_resolver_returns_any_name_requested() {
    let resolver = MockResolver::new(PathBuf::from("/usr/bin/podman"));
    let result = resolver.resolve("podman").unwrap();
    assert_eq!(result.name, "podman");
}

#[test]
fn c2_1_which_resolver_returns_matching_name() {
    // `true` exists on all unix systems
    let resolver = WhichResolver;
    let result = resolver.resolve("true").unwrap();
    assert_eq!(result.name, "true");
}

// --- C2.2: resolve() returns an absolute path ---

#[test]
fn c2_2_mock_resolver_returns_absolute_path() {
    let resolver = MockResolver::new(PathBuf::from("/usr/bin/docker"));
    let result = resolver.resolve("docker").unwrap();
    assert!(result.path.is_absolute());
}

#[test]
fn c2_2_which_resolver_returns_absolute_path() {
    let resolver = WhichResolver;
    let result = resolver.resolve("true").unwrap();
    assert!(
        result.path.is_absolute(),
        "WhichResolver should return absolute path, got: {:?}",
        result.path
    );
}

// --- C2.3: resolve() errors on missing binary ---

#[test]
fn c2_3_which_resolver_errors_on_nonexistent() {
    let resolver = WhichResolver;
    let result = resolver.resolve("nonexistent-binary-xyz-12345");
    assert!(result.is_err());
}
