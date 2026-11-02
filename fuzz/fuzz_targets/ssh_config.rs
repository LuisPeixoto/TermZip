//! ~/.ssh/config: only plain host names come out, and nothing panics.
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    for host in termzip_core::parse_hosts(&String::from_utf8_lossy(data)) {
        assert!(termzip_core::is_valid_host(&host), "an unusable host was listed: {host:?}");
    }
});
