#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Exercise with various max_len values
        for max_len in [0, 1, 10, 50, 100, 500, usize::MAX] {
            if max_len == 0 {
                // snip_contents with 0 would be degenerate; skip
                continue;
            }
            let result = repro::oci::snip_contents(s, max_len);
            // Invariant: no newlines in output
            assert!(!result.contains('\n'), "newline found in snip output");
        }
    }
});
