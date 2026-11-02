//! A snippet command and its values: parsing, checking and rendering must never panic, and what is rendered is
//! always free of control characters.
#![no_main]
use std::collections::BTreeMap;

use libfuzzer_sys::fuzz_target;
use termzip_core::{Snippet, check_command, parameters, render};

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    let middle = (0..=text.len() / 2).rev().find(|at| text.is_char_boundary(*at)).unwrap_or(0);
    let (command, value) = text.split_at(middle);
    let _ = check_command(command);
    let _ = parameters(command);
    let snippet = Snippet {
        name: "x".to_owned(),
        command: command.to_owned(),
        ..Snippet::default()
    };
    let values: BTreeMap<String, String> = ["a", "b", "c", "branch", "x"]
        .iter()
        .map(|name| ((*name).to_owned(), value.to_owned()))
        .collect();
    if let Ok(typed) = render(&snippet, &values) {
        assert!(!typed.chars().any(char::is_control), "a control character was typed: {typed:?}");
    }
});
