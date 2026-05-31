#![no_main]
use libfuzzer_sys::fuzz_target;

use repro::oci::ManifestInfo;

// Fuzz verify_digest with arbitrary digest strings and manifest data.
// Tests prefix stripping, comparison logic, and edge cases.
fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Split fuzzed input into two parts: manifest digest and expected digest
        let parts: Vec<&str> = s.splitn(2, '\n').collect();
        if parts.len() < 2 {
            return;
        }
        let manifest_digest = parts[0];
        let expected = parts[1];

        let parsed = vec![
            ManifestInfo {
                path: "index.json".into(),
                contents: "{}".into(),
                digest: "sha256:0000".into(),
                media_type: "application/vnd.oci.image.index.v1+json".into(),
                platform: None,
                manifests: vec![],
            },
            ManifestInfo {
                path: "blobs/sha256/test".into(),
                contents: "{}".into(),
                digest: manifest_digest.to_string(),
                media_type: "application/vnd.oci.image.manifest.v1+json".into(),
                platform: None,
                manifests: vec![],
            },
        ];

        // Should never panic, only return Ok or Err
        let _ = repro::oci::verify_digest(&parsed, expected);
    }
});
