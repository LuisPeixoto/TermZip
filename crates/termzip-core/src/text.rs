//! Text that TermZip types into a terminal on the user's behalf.
//!
//! Plugins, snippets and project tasks all put text at the prompt. It must never run anything by itself, so
//! every control character is removed (no line break, no Tab, no escape sequence) and so are the characters
//! that hide or reorder text, which would make what is typed differ from what was read.

/// Characters that are invisible or change how the text around them is laid out, which makes text lie:
/// zero-width spaces, direction marks and overrides, joiners and the byte order mark.
pub fn hides_or_reorders(c: char) -> bool {
    matches!(c,
        '\u{200B}'..='\u{200F}'
        | '\u{202A}'..='\u{202E}'
        | '\u{2060}'..='\u{2064}'
        | '\u{2066}'..='\u{2069}'
        | '\u{FEFF}')
}

/// Text to type: every control character goes, and so do the hidden and reordering ones. The result is at most
/// `max_bytes` long and is never cut in the middle of a character.
pub fn clean_typed(text: &str, max_bytes: usize) -> String {
    let mut out = String::new();
    for c in text.chars() {
        if c.is_control() || hides_or_reorders(c) {
            continue;
        }
        if out.len() + c.len_utf8() > max_bytes {
            break;
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_characters_are_removed() {
        assert_eq!(clean_typed("rm -rf /\n", 100), "rm -rf /");
        assert_eq!(clean_typed("a\tb\r\nc", 100), "abc");
        assert_eq!(clean_typed("\u{1b}[201~evil", 100), "[201~evil");
        assert_eq!(clean_typed("\u{3}\u{4}\u{1a}", 100), "");
    }

    #[test]
    fn hidden_and_reordering_characters_are_removed() {
        assert_eq!(clean_typed("ls\u{202E} -la\u{2066}", 100), "ls -la");
        assert_eq!(clean_typed("a\u{200B}b\u{FEFF}", 100), "ab");
    }

    #[test]
    fn the_limit_is_in_bytes_and_never_splits_a_character() {
        assert_eq!(clean_typed(&"é".repeat(10), 5), "éé");
        assert_eq!(clean_typed("abc", 0), "");
        assert_eq!(clean_typed(&"é".repeat(100), 20).len(), 20);
    }

    #[test]
    fn ordinary_text_is_kept() {
        assert_eq!(
            clean_typed("git commit -m \"x\"", 100),
            "git commit -m \"x\""
        );
    }
}
