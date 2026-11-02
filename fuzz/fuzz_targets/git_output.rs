//! What git prints (and the address of a remote) comes from a repository that may not be trusted.
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = termzip_git::parse_status(data);
    if let Some(page) = termzip_git::remote_to_https(&String::from_utf8_lossy(data)) {
        assert!(page.starts_with("https://"));
        assert!(!page.contains('@'), "credentials were kept: {page:?}");
        assert!(!page.chars().any(|c| c.is_control() || c.is_whitespace()));
    }
});
