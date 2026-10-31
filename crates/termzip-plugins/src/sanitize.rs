//! What a plugin's actions become before TermZip acts on them.
//!
//! A plugin is untrusted, so each action is checked against the permissions it was given and against where it
//! came from, and the text in it is made safe to show or type: no control characters (an escape sequence in a
//! label or in typed text must never reach the terminal), no characters that reorder or hide text, and a limit
//! on the length. What does not pass is dropped, not repaired into something else.

use termzip_plugin_api::{
    Action, MAX_LABEL_CHARS, MAX_NOTIFY_CHARS, MAX_STATUS_CHARS, MAX_TYPE_BYTES, Permission,
};

const MAX_URL_BYTES: usize = 2048;

/// Characters that are invisible or change how the text around them is laid out, which makes text lie:
/// zero-width spaces, direction marks and overrides, joiners and the byte order mark.
fn hides_or_reorders(c: char) -> bool {
    matches!(c,
        '\u{200B}'..='\u{200F}'
        | '\u{202A}'..='\u{202E}'
        | '\u{2060}'..='\u{2064}'
        | '\u{2066}'..='\u{2069}'
        | '\u{FEFF}')
}

/// Text to show: one line, no control characters, no hidden or reordering ones, at most `limit` characters.
/// Whitespace runs (including line breaks) become a single space.
pub fn clean_line(text: &str, limit: usize) -> String {
    let mut out = String::new();
    let mut pending_space = false;
    for c in text.chars() {
        if c.is_whitespace() || c == '\t' {
            pending_space = !out.is_empty();
            continue;
        }
        if c.is_control() || hides_or_reorders(c) {
            continue;
        }
        if pending_space {
            out.push(' ');
            pending_space = false;
        }
        out.push(c);
    }
    if out.chars().count() > limit {
        let mut cut: String = out.chars().take(limit.saturating_sub(1)).collect();
        cut.truncate(cut.trim_end().len());
        cut.push('…');
        return cut;
    }
    out
}

/// Text to type: every control character goes (so no line break, no Tab, no escape sequence: nothing is
/// run and nothing is completed), and so do the hidden and reordering ones. At most `MAX_TYPE_BYTES`.
pub fn clean_typed(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        if c.is_control() || hides_or_reorders(c) {
            continue;
        }
        if out.len() + c.len_utf8() > MAX_TYPE_BYTES {
            break;
        }
        out.push(c);
    }
    out
}

/// An address that is safe to hand to the browser: `http` or `https`, plain ASCII in the host, no user name
/// in front of it (`https://good.example@evil.example` lies), no spaces or control characters.
pub fn is_safe_url(url: &str) -> bool {
    if url.len() > MAX_URL_BYTES
        || url
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || hides_or_reorders(c))
    {
        return false;
    }
    let Some(rest) = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
    else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    !authority.is_empty()
        && authority
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':' | '[' | ']'))
        && authority.chars().any(|c| c.is_ascii_alphanumeric())
}

/// What survived, and how many actions did not.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Checked {
    pub actions: Vec<Action>,
    pub dropped: usize,
}

/// Keeps the actions a plugin may do. `from_command` says the actions answer a command the user ran, which is
/// the only time a plugin may type or open anything; an answer to a finished command may label it, show a
/// message and update its status, nothing else.
pub fn check_actions(
    actions: Vec<Action>,
    permissions: &[Permission],
    from_command: bool,
) -> Checked {
    let allowed = |permission: Permission| permissions.contains(&permission);
    let mut checked = Checked::default();
    for action in actions {
        let kept = match action {
            Action::Label { text, tone } if !from_command => {
                let text = clean_line(&text, MAX_LABEL_CHARS);
                (!text.is_empty()).then_some(Action::Label { text, tone })
            }
            Action::Notify { text, tone } if allowed(Permission::Notify) => {
                let text = clean_line(&text, MAX_NOTIFY_CHARS);
                (!text.is_empty()).then_some(Action::Notify { text, tone })
            }
            // An empty text clears the item, so it is kept.
            Action::Status(text) => Some(Action::Status(clean_line(&text, MAX_STATUS_CHARS))),
            Action::Type(text) if from_command && allowed(Permission::Type) => {
                let text = clean_typed(&text);
                (!text.is_empty()).then_some(Action::Type(text))
            }
            Action::Open(url) if from_command && allowed(Permission::Open) => {
                is_safe_url(&url).then_some(Action::Open(url))
            }
            _ => None,
        };
        match kept {
            Some(action) => checked.actions.push(action),
            None => checked.dropped += 1,
        }
    }
    checked
}

#[cfg(test)]
mod tests {
    use super::*;
    use termzip_plugin_api::Tone;

    fn label(text: &str) -> Action {
        Action::Label {
            text: text.to_owned(),
            tone: Tone::Warn,
        }
    }

    #[test]
    fn a_line_has_no_control_characters_and_no_line_breaks() {
        assert_eq!(
            clean_line("slow\nbuild\t\u{1b}[31mred", 24),
            "slow build [31mred"
        );
        assert_eq!(clean_line("  spaced   out  ", 24), "spaced out");
        assert_eq!(clean_line("\u{7}\u{0}\u{9b}", 24), "");
        assert_eq!(clean_line("ok", 24), "ok");
    }

    #[test]
    fn text_cannot_hide_or_reorder_itself() {
        // Right-to-left override, zero-width space, byte order mark.
        assert_eq!(
            clean_line("pass\u{202E}ssap\u{200B}ed\u{FEFF}", 24),
            "passssaped"
        );
        assert_eq!(clean_typed("ls\u{202E} -la\u{2066}"), "ls -la");
    }

    #[test]
    fn a_long_line_is_cut_with_an_ellipsis_inside_the_limit() {
        let cut = clean_line(&"x".repeat(100), 24);
        assert_eq!(cut.chars().count(), 24);
        assert!(cut.ends_with('…'));
        assert_eq!(
            clean_line("exactly-twenty-four-chars", 24).chars().count(),
            24
        );
        // Multi-byte characters count as one.
        assert_eq!(clean_line(&"é".repeat(30), 10).chars().count(), 10);
    }

    #[test]
    fn typed_text_loses_everything_that_could_run_or_complete() {
        // A line break would run it, Tab would complete it, an escape could end a paste or move the cursor.
        assert_eq!(clean_typed("rm -rf /\n"), "rm -rf /");
        assert_eq!(clean_typed("a\tb\r\nc"), "abc");
        assert_eq!(clean_typed("\u{1b}[201~evil"), "[201~evil");
        assert_eq!(clean_typed("\u{3}\u{4}\u{1a}"), "");
        // Spaces and ordinary punctuation stay: they are what a snippet is made of.
        assert_eq!(clean_typed("git commit -m \""), "git commit -m \"");
        assert_eq!(
            clean_typed(&"é".repeat(MAX_TYPE_BYTES)).len(),
            MAX_TYPE_BYTES
        );
    }

    #[test]
    fn only_web_addresses_with_an_honest_host_are_safe_to_open() {
        for good in [
            "https://example.com",
            "http://localhost:8080/path?q=1#frag",
            "https://sub.example.co.uk/a/b",
            "https://[::1]/",
        ] {
            assert!(is_safe_url(good), "{good}");
        }
        for bad in [
            "javascript:alert(1)",
            "file:///etc/passwd",
            "data:text/html,x",
            "ftp://example.com",
            "https://",
            "https:///etc",
            "//example.com",
            "https://good.example@evil.example/",
            "https://exa mple.com",
            "https://example.com/\u{1b}",
            "https://exämple.com",
            "https://example.com/\n",
            "HTTPS://example.com",
            &format!("https://example.com/{}", "a".repeat(2100)),
        ] {
            assert!(!is_safe_url(bad), "{bad:?}");
        }
    }

    #[test]
    fn labels_are_for_finished_commands_only() {
        let all = [
            Permission::Blocks,
            Permission::Notify,
            Permission::Type,
            Permission::Open,
        ];

        let from_block = check_actions(vec![label("slow")], &all, false);
        assert_eq!(from_block.actions, [label("slow")]);

        let from_command = check_actions(vec![label("slow")], &all, true);
        assert!(from_command.actions.is_empty());
        assert_eq!(from_command.dropped, 1);
    }

    #[test]
    fn typing_and_opening_need_the_permission_and_a_command_the_user_ran() {
        let typing = vec![
            Action::Type("git status".to_owned()),
            Action::Open("https://example.com".to_owned()),
        ];

        // Reacting to a finished command can never type or open, whatever the permissions.
        let from_block =
            check_actions(typing.clone(), &[Permission::Type, Permission::Open], false);
        assert!(from_block.actions.is_empty());
        assert_eq!(from_block.dropped, 2);

        // From a command it needs the permission for each.
        let none = check_actions(typing.clone(), &[], true);
        assert!(none.actions.is_empty());
        let only_type = check_actions(typing.clone(), &[Permission::Type], true);
        assert_eq!(only_type.actions, [Action::Type("git status".to_owned())]);
        let both = check_actions(typing, &[Permission::Type, Permission::Open], true);
        assert_eq!(both.actions.len(), 2);
    }

    #[test]
    fn a_message_needs_the_notify_permission() {
        let note = vec![Action::Notify {
            text: "done".to_owned(),
            tone: Tone::Ok,
        }];

        assert!(
            check_actions(note.clone(), &[Permission::Blocks], false)
                .actions
                .is_empty()
        );
        assert_eq!(
            check_actions(note, &[Permission::Notify], false)
                .actions
                .len(),
            1
        );
    }

    #[test]
    fn a_status_needs_no_permission_and_an_empty_one_clears_the_item() {
        let status = vec![
            Action::Status("3 failed".to_owned()),
            Action::Status("\n\u{1b}".to_owned()),
        ];

        let checked = check_actions(status, &[], false);
        assert_eq!(
            checked.actions,
            [
                Action::Status("3 failed".to_owned()),
                Action::Status(String::new())
            ]
        );
    }

    #[test]
    fn text_in_actions_is_cleaned_and_empty_ones_are_dropped() {
        let checked = check_actions(
            vec![
                label("\u{1b}[31mred\nline"),
                label(" \n "),
                Action::Notify {
                    text: "x".repeat(500),
                    tone: Tone::Info,
                },
                Action::Type("\n\r\t".to_owned()),
                Action::Open("javascript:alert(1)".to_owned()),
            ],
            &[Permission::Notify, Permission::Type, Permission::Open],
            true,
        );

        // The label is dropped for a command, the empty typing and the bad address too.
        assert_eq!(checked.actions.len(), 1);
        assert!(
            matches!(&checked.actions[0], Action::Notify { text, .. } if text.chars().count() == MAX_NOTIFY_CHARS)
        );
        assert_eq!(checked.dropped, 4);
    }
}
