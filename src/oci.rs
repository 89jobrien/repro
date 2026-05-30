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
