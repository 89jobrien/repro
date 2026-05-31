#![no_main]
use libfuzzer_sys::fuzz_target;
use std::io::Write;

// Fuzz multi-manifest traversal: a valid index.json referencing a fuzzed
// child manifest blob. Tests DFS traversal, digest computation, and
// nested manifest parsing with adversarial content.
fuzz_target!(|data: &[u8]| {
    // Compute the digest of the fuzzed data to build a structurally valid index
    use sha2::{Digest, Sha256};
    let digest = format!("sha256:{}", hex::encode(Sha256::digest(data)));
    let blob_path = format!("blobs/sha256/{}", digest.strip_prefix("sha256:").unwrap());

    let index_content = format!(
        r#"{{"mediaType":"application/vnd.oci.image.index.v1+json","manifests":[{{"digest":"{digest}","mediaType":"application/vnd.oci.image.manifest.v1+json"}}]}}"#
    );

    let mut buf = Vec::new();
    {
        let mut builder = tar::Builder::new(&mut buf);

        // Add index.json
        let index_bytes = index_content.as_bytes();
        let mut header = tar::Header::new_gnu();
        header.set_size(index_bytes.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        let _ = builder.append_data(&mut header, "index.json", index_bytes);

        // Add the fuzzed manifest blob at the correct path
        let mut header = tar::Header::new_gnu();
        header.set_size(data.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        let _ = builder.append_data(&mut header, &blob_path, data);

        let _ = builder.finish();
    }

    if let Ok(mut tmpfile) = tempfile::NamedTempFile::new() {
        if tmpfile.write_all(&buf).is_ok() {
            let _ = repro::oci::parse_tarball(tmpfile.path());
        }
    }
});
