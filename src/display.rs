//! Presentation layer for OCI tarball analysis output.

use crate::oci::ManifestInfo;

/// Maximum number of characters to display for manifest contents in summary mode.
const SUMMARY_MAX_LEN: usize = 600;

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
            crate::oci::snip_contents(&info.contents, SUMMARY_MAX_LEN)
        };
        println!("  Contents: {contents}");
    }
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_info_does_not_panic_on_single_manifest() {
        let info = vec![
            ManifestInfo {
                path: "index.json".into(),
                digest: "sha256:aaa".into(),
                media_type: "application/vnd.oci.image.index.v1+json".into(),
                contents: "{}".into(),
                platform: None,
                manifests: vec![],
            },
            ManifestInfo {
                path: "blobs/sha256/bbb".into(),
                digest: "sha256:bbb".into(),
                media_type: "application/vnd.oci.image.manifest.v1+json".into(),
                contents: r#"{"config":{}}"#.into(),
                platform: Some("linux/amd64".into()),
                manifests: vec![],
            },
        ];
        // Smoke test: no panic, exercises both full=true and full=false
        print_info(&info, false);
        print_info(&info, true);
    }
}
