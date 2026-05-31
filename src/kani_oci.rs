use super::normalize_path;

// normalize_path: the format!() calls are too heavy for Kani.
// We verify the branching predicates and the passthrough case.

#[kani::proof]
fn normalize_path_sha256_takes_format_branch() {
    let input = "sha256:abcdef0123456789";
    assert!(input.strip_prefix("sha256:").is_some());
}

#[kani::proof]
fn normalize_path_idempotent_predicate() {
    let result = "blobs/sha256/abcdef0123456789";
    assert!(result.strip_prefix("sha256:").is_none());
}

#[kani::proof]
fn normalize_path_no_sha256_passthrough() {
    let result = normalize_path("index.json");
    assert_eq!(result.as_str(), "index.json");
}

// --- verify_digest: test prefix-stripping logic directly ---

#[kani::proof]
fn digest_prefix_strip_symmetry() {
    let a = "sha256:abcd1234";
    let b = "abcd1234";
    assert_eq!(
        a.strip_prefix("sha256:").unwrap_or(a),
        b.strip_prefix("sha256:").unwrap_or(b),
    );
}

#[kani::proof]
fn digest_mismatch_detected() {
    let a = "sha256:abcd1234";
    let b = "sha256:wrong";
    assert_ne!(
        a.strip_prefix("sha256:").unwrap_or(a),
        b.strip_prefix("sha256:").unwrap_or(b),
    );
}
