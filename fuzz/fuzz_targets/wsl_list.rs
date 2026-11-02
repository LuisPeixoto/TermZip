//! The output of `wsl.exe -l -q`: plain names or nothing.
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    for name in termzip_core::parse_distributions(data) {
        assert!(termzip_core::is_valid_remote_name(&name), "an unusable name was listed: {name:?}");
    }
});
