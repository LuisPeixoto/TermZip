//! workspace.json is read as untrusted input: a layout that breaks a limit is an error, never a panic.
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(workspace) = termzip_core::Workspace::from_json(&String::from_utf8_lossy(data)) {
        assert!(workspace.tabs.len() <= 32);
    }
});
