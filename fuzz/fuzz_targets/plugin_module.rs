//! A plugin file is untrusted: reading it (structure, manifest, imports, exports) must never panic.
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = termzip_plugins::read_module(data);
});
