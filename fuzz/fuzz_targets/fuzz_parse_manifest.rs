#![no_main]
use libfuzzer_sys::fuzz_target;
use std::io::Write;

fuzz_target!(|data: &[u8]| {
    // Build a tar archive containing the fuzzed data as index.json
    let mut buf = Vec::new();
    {
        let mut builder = tar::Builder::new(&mut buf);
        let mut header = tar::Header::new_gnu();
        header.set_size(data.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        let _ = builder.append_data(&mut header, "index.json", data);
        let _ = builder.finish();
    }

    // Write to a temp file and attempt to parse
    if let Ok(mut tmpfile) = tempfile::NamedTempFile::new() {
        if tmpfile.write_all(&buf).is_ok() {
            let _ = repro::oci::parse_tarball(tmpfile.path());
        }
    }
});
