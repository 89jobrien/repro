#![no_main]
use libfuzzer_sys::fuzz_target;
use std::io::Write;

// Fuzz the tarball parser with completely arbitrary bytes as the tar file.
// Tests resilience to malformed tar headers, truncated entries, and corrupted archives.
fuzz_target!(|data: &[u8]| {
    if let Ok(mut tmpfile) = tempfile::NamedTempFile::new() {
        if tmpfile.write_all(data).is_ok() {
            let _ = repro::oci::parse_tarball(tmpfile.path());
        }
    }
});
