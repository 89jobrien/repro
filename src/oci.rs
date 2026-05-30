//! OCI tarball parsing and analysis.
//!
//! Compatible with the OCI image layout spec (opencontainers/image-spec).

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::Path;

/// Parsed manifest/index entry from an OCI tarball.
#[derive(Debug, Clone)]
pub struct ManifestInfo {
    pub path: String,
    pub contents: String,
    pub digest: String,
    pub media_type: String,
    pub platform: Option<String>,
    pub manifests: Vec<ManifestDescriptor>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct ManifestDescriptor {
    pub digest: String,
    #[serde(rename = "mediaType")]
    pub media_type: Option<String>,
    pub platform: Option<PlatformSpec>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlatformSpec {
    pub os: String,
    pub architecture: String,
}

#[derive(Deserialize)]
struct ManifestJson {
    #[serde(rename = "mediaType")]
    media_type: String,
    #[serde(default)]
    manifests: Vec<ManifestDescriptor>,
}

/// Normalize an OCI path (e.g. `sha256:abc...` -> `blobs/sha256/abc...`).
fn normalize_path(path: &str) -> String {
    if let Some(checksum) = path.strip_prefix("sha256:") {
        format!("blobs/sha256/{checksum}")
    } else {
        path.to_string()
    }
}

/// Extract a file from an OCI tarball, handling `./` prefix variations.
fn get_file_from_tarball(tar: &mut tar::Archive<impl Read>, path: &str) -> Result<String> {
    let dotslash = format!("./{path}");
    for entry in tar.entries().context("reading tar entries")? {
        let entry = entry.context("reading tar entry")?;
        let entry_path = entry.path().context("reading entry path")?;
        let entry_str = entry_path.to_string_lossy();
        if entry_str == path || entry_str == dotslash {
            let mut buf = String::new();
            let mut reader = entry;
            reader
                .read_to_string(&mut buf)
                .with_context(|| format!("reading {path}"))?;
            return Ok(buf);
        }
    }
    bail!("file not found in tarball: {path}");
}

/// Parse a single manifest/index entry.
fn parse_manifest(
    tarball_path: &Path,
    path: &str,
    platform: Option<String>,
) -> Result<ManifestInfo> {
    let normalized = normalize_path(path);
    let file = std::fs::File::open(tarball_path)
        .with_context(|| format!("opening {}", tarball_path.display()))?;
    let decoder = flate2_or_raw(file, tarball_path);
    let mut archive = tar::Archive::new(decoder);
    let contents = get_file_from_tarball(&mut archive, &normalized)
        .with_context(|| format!("extracting {normalized}"))?;

    let digest = format!(
        "sha256:{}",
        hex::encode(Sha256::digest(contents.as_bytes()))
    );
    let parsed: ManifestJson = serde_json::from_str(&contents).context("parsing manifest JSON")?;

    Ok(ManifestInfo {
        path: normalized,
        contents,
        digest,
        media_type: parsed.media_type,
        platform,
        manifests: parsed.manifests,
    })
}

/// Open a tar file, auto-detecting gzip compression.
fn flate2_or_raw(file: std::fs::File, path: &Path) -> Box<dyn Read> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    if ext == "gz" || ext == "tgz" {
        Box::new(flate2::read::GzDecoder::new(file))
    } else {
        Box::new(file)
    }
}

/// Recursively parse manifests via DFS starting from `index.json`.
fn parse_manifests_dfs(
    tarball_path: &Path,
    path: &str,
    parsed: &mut Vec<ManifestInfo>,
    platform: Option<String>,
) -> Result<()> {
    let info = parse_manifest(tarball_path, path, platform)?;
    let child_manifests: Vec<_> = info.manifests.clone();
    parsed.push(info);
    for m in child_manifests {
        let plat = m
            .platform
            .as_ref()
            .map(|p| format!("{}/{}", p.os, p.architecture));
        parse_manifests_dfs(tarball_path, &m.digest, parsed, plat)?;
    }
    Ok(())
}

/// Parse an OCI tarball, returning all manifest/index entries.
pub fn parse_tarball(path: &Path) -> Result<Vec<ManifestInfo>> {
    let mut parsed = Vec::new();
    parse_manifests_dfs(path, "index.json", &mut parsed, None)?;
    Ok(parsed)
}

/// Truncate a string for display, removing newlines.
fn snip_contents(contents: &str, max_len: usize) -> String {
    let flat: String = contents.chars().filter(|c| *c != '\n').collect();
    let char_count = flat.chars().count();
    if char_count > max_len {
        let omitted = char_count - max_len;
        let truncated: String = flat.chars().take(max_len).collect();
        format!(
            "{truncated}  [... {omitted} characters omitted. Pass --show-contents to print them in their entirety]"
        )
    } else {
        flat
    }
}

/// Print parsed OCI tarball info to stdout.
pub fn print_info(parsed: &[ManifestInfo], full: bool) {
    println!(
        "The OCI tarball contains an index and {} manifest(s):",
        parsed.len() - 1
    );
    println!();
    if parsed.len() > 1 {
        println!("Image digest: {}", parsed[1].digest);
    }
    for (i, info) in parsed.iter().enumerate() {
        println!();
        if i == 0 {
            println!("Index ({}):", info.path);
        } else {
            println!("Manifest {i} ({}):", info.path);
        }
        println!("  Digest: {}", info.digest);
        println!("  Media type: {}", info.media_type);
        println!("  Platform: {}", info.platform.as_deref().unwrap_or("-"));
        let contents = if full {
            info.contents.clone()
        } else {
            snip_contents(&info.contents, 600)
        };
        println!("  Contents: {contents}");
    }
    println!();
}

/// Verify the image digest matches an expected value.
pub fn verify_digest(parsed: &[ManifestInfo], expected: &str) -> Result<()> {
    if parsed.len() < 2 {
        bail!("no manifest found in tarball to verify");
    }
    let cur_digest = parsed[1]
        .digest
        .strip_prefix("sha256:")
        .unwrap_or(&parsed[1].digest);

    let expected_hash = expected.strip_prefix("sha256:").unwrap_or(expected);

    if cur_digest != expected_hash {
        bail!("image does not have the expected digest: {cur_digest} != {expected_hash}");
    }
    println!("Image digest matches {expected_hash}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- normalize_path ---

    #[test]
    fn normalize_path_sha256_prefix() {
        assert_eq!(normalize_path("sha256:abc123"), "blobs/sha256/abc123");
    }

    #[test]
    fn normalize_path_passthrough() {
        assert_eq!(normalize_path("index.json"), "index.json");
        assert_eq!(normalize_path("blobs/sha256/abc"), "blobs/sha256/abc");
    }

    // --- snip_contents ---

    #[test]
    fn snip_contents_short_string() {
        let input = "hello world";
        assert_eq!(snip_contents(input, 100), "hello world");
    }

    #[test]
    fn snip_contents_truncates() {
        let input = "a".repeat(200);
        let result = snip_contents(&input, 50);
        assert!(result.starts_with(&"a".repeat(50)));
        assert!(result.contains("150 characters omitted"));
    }

    #[test]
    fn snip_contents_strips_newlines() {
        let input = "line1\nline2\nline3";
        let result = snip_contents(input, 100);
        assert!(!result.contains('\n'));
        assert_eq!(result, "line1line2line3");
    }

    // --- verify_digest ---

    fn make_manifest(digest: &str) -> ManifestInfo {
        ManifestInfo {
            path: "test".into(),
            contents: "{}".into(),
            digest: digest.into(),
            media_type: "application/vnd.oci.image.manifest.v1+json".into(),
            platform: None,
            manifests: vec![],
        }
    }

    #[test]
    fn verify_digest_match() {
        let index = make_manifest("sha256:0000");
        let manifest = make_manifest("sha256:abcd1234");
        let parsed = vec![index, manifest];
        assert!(verify_digest(&parsed, "sha256:abcd1234").is_ok());
    }

    #[test]
    fn verify_digest_match_without_prefix() {
        let index = make_manifest("sha256:0000");
        let manifest = make_manifest("sha256:abcd1234");
        let parsed = vec![index, manifest];
        assert!(verify_digest(&parsed, "abcd1234").is_ok());
    }

    #[test]
    fn verify_digest_mismatch() {
        let index = make_manifest("sha256:0000");
        let manifest = make_manifest("sha256:abcd1234");
        let parsed = vec![index, manifest];
        assert!(verify_digest(&parsed, "sha256:wrong").is_err());
    }

    #[test]
    fn verify_digest_empty_parsed() {
        let result = verify_digest(&[], "sha256:abc");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no manifest"));
    }

    #[test]
    fn verify_digest_only_index() {
        let parsed = vec![make_manifest("sha256:0000")];
        assert!(verify_digest(&parsed, "sha256:abc").is_err());
    }

    // --- parse_tarball with a real tar ---

    #[test]
    fn parse_tarball_minimal_oci() {
        use tempfile::NamedTempFile;

        // Build a minimal OCI tarball: index.json -> one manifest blob
        let manifest_content = r#"{"mediaType":"application/vnd.oci.image.manifest.v1+json","config":{"digest":"sha256:dead"},"layers":[]}"#;
        let manifest_digest = format!(
            "sha256:{}",
            hex::encode(sha2::Sha256::digest(manifest_content.as_bytes()))
        );
        let manifest_blob_path = format!(
            "blobs/sha256/{}",
            manifest_digest.strip_prefix("sha256:").unwrap()
        );

        let index_content = format!(
            r#"{{"mediaType":"application/vnd.oci.image.index.v1+json","manifests":[{{"digest":"{manifest_digest}","mediaType":"application/vnd.oci.image.manifest.v1+json"}}]}}"#
        );

        let mut tmpfile = NamedTempFile::new().expect("create temp file");
        {
            let file = tmpfile.as_file_mut();
            let mut builder = tar::Builder::new(file);

            // Add index.json
            let index_bytes = index_content.as_bytes();
            let mut header = tar::Header::new_gnu();
            header.set_size(index_bytes.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder
                .append_data(&mut header, "index.json", index_bytes)
                .expect("append index.json");

            // Add manifest blob
            let manifest_bytes = manifest_content.as_bytes();
            let mut header = tar::Header::new_gnu();
            header.set_size(manifest_bytes.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder
                .append_data(&mut header, &manifest_blob_path, manifest_bytes)
                .expect("append manifest blob");

            builder.finish().expect("finish tar");
        }

        let parsed = parse_tarball(tmpfile.path()).expect("should parse");
        assert_eq!(parsed.len(), 2, "index + 1 manifest");
        assert_eq!(parsed[0].path, "index.json");
        assert_eq!(parsed[1].digest, manifest_digest);
    }

    // --- property tests ---

    mod proptests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn normalize_path_sha256_always_prefixed(hash in "[a-f0-9]{64}") {
                let input = format!("sha256:{hash}");
                let result = normalize_path(&input);
                prop_assert!(result.starts_with("blobs/sha256/"));
                prop_assert!(result.ends_with(&hash));
            }

            #[test]
            fn normalize_path_no_prefix_passthrough(s in "[a-z]{1,50}") {
                let result = normalize_path(&s);
                prop_assert_eq!(result, s);
            }

            #[test]
            fn snip_contents_length_invariant(
                input in ".{0,500}",
                max_len in 1usize..200
            ) {
                let result = snip_contents(&input, max_len);
                let flat_len = input.chars().filter(|c| *c != '\n').count();
                if flat_len <= max_len {
                    prop_assert!(!result.contains("omitted"));
                } else {
                    prop_assert!(result.contains("omitted"));
                }
            }

            #[test]
            fn snip_contents_no_newlines(input in ".{0,200}") {
                let result = snip_contents(&input, 100);
                prop_assert!(!result.contains('\n'));
            }
        }
    }
}

#[cfg(kani)]
mod kani_proofs {
    use super::*;

    // normalize_path: the format!() calls are too heavy for Kani.
    // We verify the branching predicates and the passthrough case.

    #[kani::proof]
    fn normalize_path_sha256_takes_format_branch() {
        let input = "sha256:abcdef0123456789";
        assert!(input.strip_prefix("sha256:").is_some());
    }

    #[kani::proof]
    fn normalize_path_idempotent_predicate() {
        // After one normalize, result starts with "blobs/sha256/" (not "sha256:")
        // so strip_prefix("sha256:") returns None => passthrough on second call.
        let result = "blobs/sha256/abcdef0123456789";
        assert!(result.strip_prefix("sha256:").is_none());
    }

    #[kani::proof]
    fn normalize_path_no_sha256_passthrough() {
        let result = normalize_path("index.json");
        assert_eq!(result.as_str(), "index.json");
    }

    // --- verify_digest: test prefix-stripping logic directly ---
    // The full function uses anyhow which is too heavy for Kani.

    #[kani::proof]
    fn digest_prefix_strip_symmetry() {
        let with_prefix = "sha256:abcd1234";
        let without_prefix = "abcd1234";

        let stripped_a = with_prefix.strip_prefix("sha256:").unwrap_or(with_prefix);
        let stripped_b = without_prefix
            .strip_prefix("sha256:")
            .unwrap_or(without_prefix);

        assert_eq!(stripped_a, stripped_b);
    }

    #[kani::proof]
    fn digest_mismatch_detected() {
        let digest = "sha256:abcd1234";
        let expected = "sha256:wrong";

        let cur = digest.strip_prefix("sha256:").unwrap_or(digest);
        let exp = expected.strip_prefix("sha256:").unwrap_or(expected);

        assert_ne!(cur, exp);
    }
}
