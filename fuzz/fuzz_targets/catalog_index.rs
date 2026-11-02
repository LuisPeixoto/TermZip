//! The catalog index and its signature come from the network. The signature is checked before anything is read;
//! with arbitrary bytes it must refuse without a panic, and the parser must cope with whatever it is given.
#![no_main]
use libfuzzer_sys::fuzz_target;

// The public key built into TermZip's tests (any valid key does for this).
const KEY: &str = "RWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3";

fuzz_target!(|data: &[u8]| {
    let split = data.len() / 2;
    let (index, signature) = data.split_at(split);
    let _ = termzip_plugins::parse_index(index);
    let _ = termzip_plugins::verify_and_parse(index, &String::from_utf8_lossy(signature), KEY);
});
