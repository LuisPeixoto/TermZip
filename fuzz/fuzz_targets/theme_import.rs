//! Theme files a person picked: iTerm2, Windows Terminal, Ghostty, Kitty, Alacritty and TermZip's own.
#![no_main]
use libfuzzer_sys::fuzz_target;

const NAMES: [&str; 6] = ["a.toml", "a.itermcolors", "a.json", "a.conf", "ghostty", "a.txt"];

fuzz_target!(|data: &[u8]| {
    let Some((first, rest)) = data.split_first() else { return };
    let _ = termzip_core::import_theme(NAMES[*first as usize % NAMES.len()], rest);
});
