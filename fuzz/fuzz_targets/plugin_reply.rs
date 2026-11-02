//! What a plugin answers is untrusted: bounded JSON that becomes actions or a fault, never a panic.
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = termzip_plugins::parse_reply(data);
});
