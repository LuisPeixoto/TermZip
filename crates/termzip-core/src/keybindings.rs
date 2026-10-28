//! The grammar of a keyboard shortcut as it is written in `[keybindings]`: `Ctrl+Alt+Shift+Cmd+Key`.
//!
//! Keys are named like the browser's `KeyboardEvent.code` without its `Key` and `Digit` prefixes, so a
//! shortcut means the same physical key on every layout. The file is hand-editable input: anything
//! that is not a well-formed shortcut is dropped on load, and explained when it comes from the
//! preferences dialog.

use std::collections::BTreeMap;

pub const MAX_KEYBINDINGS: usize = 64;
const MAX_ACTION_CHARS: usize = 32;
const MAX_CHORD_CHARS: usize = 32;

const NAMED_KEYS: [&str; 25] = [
    "Tab",
    "Enter",
    "Space",
    "Backspace",
    "Delete",
    "Insert",
    "Left",
    "Right",
    "Up",
    "Down",
    "Home",
    "End",
    "PageUp",
    "PageDown",
    "Comma",
    "Period",
    "Slash",
    "Semicolon",
    "Quote",
    "BracketLeft",
    "BracketRight",
    "Backslash",
    "Minus",
    "Equal",
    "Backquote",
];

fn canonical_key(token: &str) -> Option<String> {
    let lower = token.to_ascii_lowercase();
    if lower.len() == 1 {
        let c = lower.as_bytes()[0];
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            return Some(lower.to_ascii_uppercase());
        }
    }
    if let Some(number) = lower.strip_prefix('f')
        && let Ok(number) = number.parse::<u8>()
        && (1..=24).contains(&number)
    {
        return Some(format!("F{number}"));
    }
    NAMED_KEYS
        .iter()
        .find(|name| name.eq_ignore_ascii_case(token))
        .map(|name| (*name).to_owned())
}

/// The shortcut in its canonical spelling (`cmd+shift+d` becomes `Shift+Cmd+D`), or `None` when it is not
/// one TermZip accepts. A shortcut needs Ctrl, Alt or Cmd, so typing a plain letter or Shift+letter is
/// never taken over; function keys may stand alone.
pub fn normalize_chord(text: &str) -> Option<String> {
    let text = text.trim();
    if text.is_empty() || text.len() > MAX_CHORD_CHARS {
        return None;
    }
    let (mut ctrl, mut alt, mut shift, mut cmd) = (false, false, false, false);
    let mut key = None;
    for token in text.split('+') {
        let token = token.trim();
        let modifier = match token.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => Some(&mut ctrl),
            "alt" | "option" | "opt" => Some(&mut alt),
            "shift" => Some(&mut shift),
            "cmd" | "command" | "meta" | "super" | "win" => Some(&mut cmd),
            _ => None,
        };
        match modifier {
            Some(flag) => {
                if *flag {
                    return None; // Ctrl+Ctrl+A
                }
                *flag = true;
            }
            None => {
                if key.replace(canonical_key(token)?).is_some() {
                    return None; // two keys
                }
            }
        }
    }
    let key = key?;
    let function_key = key.starts_with('F') && key[1..].parse::<u8>().is_ok();
    if !(ctrl || alt || cmd || function_key) {
        return None;
    }
    let mut parts = Vec::new();
    for (on, name) in [(ctrl, "Ctrl"), (alt, "Alt"), (shift, "Shift"), (cmd, "Cmd")] {
        if on {
            parts.push(name.to_owned());
        }
    }
    parts.push(key);
    Some(parts.join("+"))
}

fn valid_action(action: &str) -> bool {
    !action.is_empty()
        && action.len() <= MAX_ACTION_CHARS
        && action.chars().all(|c| c.is_ascii_alphanumeric())
}

/// The first problem found, as a sentence the preferences dialog can show as it is. An empty chord
/// means "no shortcut" and is fine.
pub fn validate_keybindings(bindings: &BTreeMap<String, String>) -> Result<(), String> {
    if bindings.len() > MAX_KEYBINDINGS {
        return Err(format!(
            "At most {MAX_KEYBINDINGS} shortcuts can be customized."
        ));
    }
    for (action, chord) in bindings {
        if !valid_action(action) {
            return Err(format!(
                "“{action}” is not a name TermZip can use for an action."
            ));
        }
        if !chord.trim().is_empty() && normalize_chord(chord).is_none() {
            return Err(format!(
                "“{chord}” cannot be the shortcut for {action}. Use Ctrl, Alt or Cmd with a key, such as Cmd+Shift+D."
            ));
        }
    }
    Ok(())
}

/// Keeps what is usable, in canonical spelling. Never fails; `validate_keybindings` explains first.
pub(crate) fn normalize_keybindings(
    bindings: BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    bindings
        .into_iter()
        .filter(|(action, _)| valid_action(action))
        .filter_map(|(action, chord)| {
            if chord.trim().is_empty() {
                Some((action, String::new()))
            } else {
                normalize_chord(&chord).map(|chord| (action, chord))
            }
        })
        .take(MAX_KEYBINDINGS)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(a, c)| ((*a).to_owned(), (*c).to_owned()))
            .collect()
    }

    #[test]
    fn chords_are_written_in_one_canonical_order() {
        assert_eq!(
            normalize_chord("cmd+shift+d").as_deref(),
            Some("Shift+Cmd+D")
        );
        assert_eq!(
            normalize_chord("Shift + Ctrl + t").as_deref(),
            Some("Ctrl+Shift+T")
        );
        assert_eq!(
            normalize_chord("Option+Command+Left").as_deref(),
            Some("Alt+Cmd+Left")
        );
        assert_eq!(normalize_chord("ctrl+tab").as_deref(), Some("Ctrl+Tab"));
        assert_eq!(
            normalize_chord("Cmd+bracketleft").as_deref(),
            Some("Cmd+BracketLeft")
        );
        assert_eq!(
            normalize_chord("Ctrl+Shift+3").as_deref(),
            Some("Ctrl+Shift+3")
        );
        assert_eq!(
            normalize_chord("Cmd+,"),
            None,
            "punctuation is spelled out: Comma"
        );
        assert_eq!(normalize_chord("Cmd+Comma").as_deref(), Some("Cmd+Comma"));
    }

    #[test]
    fn a_chord_that_would_break_typing_is_refused() {
        for bad in ["D", "Shift+D", "Shift+3", "Space", "Shift+Tab", "Enter"] {
            assert_eq!(normalize_chord(bad), None, "{bad}");
        }
        assert_eq!(normalize_chord("F5").as_deref(), Some("F5"));
        assert_eq!(normalize_chord("shift+f12").as_deref(), Some("Shift+F12"));
    }

    #[test]
    fn malformed_chords_are_refused() {
        for bad in [
            "",
            " ",
            "+",
            "Ctrl+",
            "+D",
            "Ctrl++D",
            "Ctrl+Ctrl+D",
            "Ctrl+D+E",
            "Ctrl+Hyper+D",
            "Ctrl+Escape",
            "Ctrl+F25",
            "Ctrl+F0",
            "Ctrl+Numpad",
            "Ctrl+Plus",
            "Ctrl+é",
            "Cmd+Shift+Option+Control+Alt+Super+AAAAAAAAAAAA",
        ] {
            assert_eq!(normalize_chord(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn validation_explains_the_first_problem() {
        assert!(validate_keybindings(&map(&[("splitRight", "Cmd+Shift+E"), ("find", "")])).is_ok());

        let chord = validate_keybindings(&map(&[("splitRight", "Shift+E")])).unwrap_err();
        assert!(
            chord.contains("Shift+E") && chord.contains("splitRight"),
            "{chord}"
        );

        let action = validate_keybindings(&map(&[("split right", "Cmd+E")])).unwrap_err();
        assert!(action.contains("split right"), "{action}");

        let many: BTreeMap<_, _> = (0..=MAX_KEYBINDINGS)
            .map(|n| (format!("a{n}"), "Cmd+E".to_owned()))
            .collect();
        assert!(validate_keybindings(&many).is_err());
    }

    #[test]
    fn cleaning_keeps_the_usable_entries_and_the_unbound_ones() {
        let cleaned = normalize_keybindings(map(&[
            ("splitRight", "cmd+shift+e"),
            ("find", ""),
            ("closePane", "W"),
            ("not an action", "Cmd+E"),
            ("newTab", "  Ctrl+T "),
        ]));

        assert_eq!(
            cleaned,
            map(&[
                ("splitRight", "Shift+Cmd+E"),
                ("find", ""),
                ("newTab", "Ctrl+T")
            ])
        );
    }
}
