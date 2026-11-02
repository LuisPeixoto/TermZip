//! The terminal's output is untrusted: the marker parser must never panic or grow without bound.
#![no_main]
use libfuzzer_sys::fuzz_target;
use termzip_shell::{OscParser, strip_ansi};

fuzz_target!(|data: &[u8]| {
    let mut parser = OscParser::new();
    // In pieces, the way a PTY delivers it, so the buffering across calls is exercised too.
    for chunk in data.chunks(7) {
        let _ = parser.feed_ranges(chunk);
    }
    let mut whole = OscParser::new();
    let _ = whole.feed(data);
    let _ = strip_ansi(data);
});
