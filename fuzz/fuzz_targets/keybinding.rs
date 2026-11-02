//! A chord written by hand: it is either normalized to a form that normalizes to itself, or refused.
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    if let Some(chord) = termzip_core::normalize_chord(&text) {
        assert_eq!(termzip_core::normalize_chord(&chord).as_deref(), Some(chord.as_str()));
    }
});
