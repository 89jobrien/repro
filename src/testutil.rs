//! Shared test fixtures for OCI tarball construction.
//!
//! Available in tests via `repro::testutil` when the `testutil` feature is enabled,
//! or in-crate via `#[cfg(test)]`.

use sha2::{Digest, Sha256};
use tempfile::NamedTempFile;

/// Standard file permission mode for tar entries (0o644 = 420 decimal).
const TAR_FILE_MODE: u32 = 0o644;

/// Builder for synthetic OCI tarballs with configurable manifests.
pub struct OciTarballFixture {
    manifests: Vec<FixtureManifest>,
    use_dot_slash_prefix: bool,
    gzip: bool,
}

struct FixtureManifest {
    content: String,
    platform: Option<(String, String)>,
}

impl Default for OciTarballFixture {
    fn default() -> Self {
        Self {
            manifests: vec![FixtureManifest {
                content: r#"{"mediaType":"application/vnd.oci.image.manifest.v1+json","config":{"digest":"sha256:dead"},"layers":[]}"#.into(),
                platform: None,
            }],
            use_dot_slash_prefix: false,
            gzip: false,
        }
    }
}

impl OciTarballFixture {
    /// Use `./` prefix on tar entry paths (tests path normalization).
    pub fn with_dot_slash_prefix(mut self) -> Self {
        self.use_dot_slash_prefix = true;
        self
    }

    /// Wrap the tarball in gzip (tests auto-detection).
    pub fn with_gzip(mut self) -> Self {
        self.gzip = true;
        self
    }

    /// Replace the default manifest with custom content.
    pub fn with_manifest(mut self, content: &str) -> Self {
        self.manifests = vec![FixtureManifest {
            content: content.into(),
            platform: None,
        }];
        self
    }

    /// Add an additional manifest with a platform.
    pub fn add_manifest(mut self, content: &str, os: &str, arch: &str) -> Self {
        self.manifests.push(FixtureManifest {
            content: content.into(),
            platform: Some((os.into(), arch.into())),
        });
        self
    }

    /// Build the tarball and return the temp file + expected manifest digests.
    pub fn build(self) -> (NamedTempFile, Vec<String>) {
        let mut digests = Vec::new();
        let mut manifest_entries: Vec<(String, String, Option<String>)> = Vec::new();

        for m in &self.manifests {
            let digest = format!(
                "sha256:{}",
                hex::encode(Sha256::digest(m.content.as_bytes()))
            );
            let blob_path = format!("blobs/sha256/{}", digest.strip_prefix("sha256:").unwrap());
            let platform_json = m.platform.as_ref().map(|(os, arch)| {
                format!(r#","platform":{{"os":"{os}","architecture":"{arch}"}}"#)
            });
            manifest_entries.push((digest.clone(), blob_path, platform_json));
            digests.push(digest);
        }

        // Build index.json referencing all manifests
        let manifest_refs: Vec<String> = manifest_entries
            .iter()
            .map(|(digest, _, plat)| {
                let plat_str = plat.as_deref().unwrap_or("");
                format!(
                    r#"{{"digest":"{digest}","mediaType":"application/vnd.oci.image.manifest.v1+json"{plat_str}}}"#
                )
            })
            .collect();
        let index_content = format!(
            r#"{{"mediaType":"application/vnd.oci.image.index.v1+json","manifests":[{}]}}"#,
            manifest_refs.join(",")
        );

        let mut tmpfile = NamedTempFile::new().expect("create temp file");

        if self.gzip {
            let gz =
                flate2::write::GzEncoder::new(tmpfile.as_file_mut(), flate2::Compression::fast());
            self.write_tar(gz, &index_content, &manifest_entries);
            // rename to .tar.gz so flate2_or_raw detects it
            // NamedTempFile doesn't support rename, so we write a new file
            let mut gz_file = NamedTempFile::with_suffix(".tar.gz").expect("create gz temp file");
            let gz_enc =
                flate2::write::GzEncoder::new(gz_file.as_file_mut(), flate2::Compression::fast());
            self.write_tar_inner(gz_enc, &index_content, &manifest_entries);
            return (gz_file, digests);
        }

        self.write_tar(tmpfile.as_file_mut(), &index_content, &manifest_entries);
        (tmpfile, digests)
    }

    fn write_tar<W: std::io::Write>(
        &self,
        writer: W,
        index_content: &str,
        entries: &[(String, String, Option<String>)],
    ) {
        self.write_tar_inner(writer, index_content, entries);
    }

    fn write_tar_inner<W: std::io::Write>(
        &self,
        writer: W,
        index_content: &str,
        entries: &[(String, String, Option<String>)],
    ) {
        let mut builder = tar::Builder::new(writer);
        let prefix = if self.use_dot_slash_prefix { "./" } else { "" };

        // Add index.json
        let index_bytes = index_content.as_bytes();
        let mut header = tar::Header::new_gnu();
        header.set_size(index_bytes.len() as u64);
        header.set_mode(TAR_FILE_MODE);
        header.set_cksum();
        builder
            .append_data(&mut header, format!("{prefix}index.json"), index_bytes)
            .expect("append index.json");

        // Add manifest blobs
        for (i, (_, blob_path, _)) in entries.iter().enumerate() {
            let content = self.manifests[i].content.as_bytes();
            let mut header = tar::Header::new_gnu();
            header.set_size(content.len() as u64);
            header.set_mode(TAR_FILE_MODE);
            header.set_cksum();
            builder
                .append_data(&mut header, format!("{prefix}{blob_path}"), content)
                .expect("append manifest blob");
        }

        builder.finish().expect("finish tar");
    }
}
