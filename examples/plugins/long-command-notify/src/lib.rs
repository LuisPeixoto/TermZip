//! Tells you when a long command has finished: a "slow" label on its block and a message with how long it took.
//!
//! It needs two permissions: `blocks` to hear about finished commands and `notify` to show the message.

use termzip_plugin_sdk::prelude::*;

termzip_plugin_sdk::manifest!("../plugin.toml");

/// A command is long when it ran for more than this many milliseconds.
const LONG_MS: u64 = 10_000;
/// How much of the command is repeated in the message.
const COMMAND_CHARS: usize = 60;

#[derive(Default)]
struct LongCommands;

impl Plugin for LongCommands {
    fn on_block(&mut self, block: &Block) -> Vec<Action> {
        let Some(ms) = block.duration_ms.filter(|ms| *ms > LONG_MS) else {
            return Vec::new();
        };
        let command = shorten(block.command.as_deref().unwrap_or("A command"));
        let took = duration(ms);
        let (tone, message) = match block.exit_code {
            Some(code) if code != 0 => (
                Tone::Err,
                format!("{command} failed (exit {code}) after {took}"),
            ),
            _ => (Tone::Ok, format!("{command} finished in {took}")),
        };
        vec![label("slow", Tone::Warn), notify(message, tone)]
    }
}

termzip_plugin_sdk::export!(LongCommands);

/// The command on one line, cut with an ellipsis if it is long.
fn shorten(command: &str) -> String {
    let command = command.split_whitespace().collect::<Vec<_>>().join(" ");
    if command.chars().count() <= COMMAND_CHARS {
        return command;
    }
    let cut: String = command.chars().take(COMMAND_CHARS - 1).collect();
    format!("{}…", cut.trim_end())
}

/// `45 s`, `2 min 5 s`, `1 h 5 min`.
fn duration(ms: u64) -> String {
    let seconds = ms / 1000;
    let (hours, minutes, seconds) = (seconds / 3600, seconds / 60 % 60, seconds % 60);
    match (hours, minutes, seconds) {
        (0, 0, s) => format!("{s} s"),
        (0, m, 0) => format!("{m} min"),
        (0, m, s) => format!("{m} min {s} s"),
        (h, 0, _) => format!("{h} h"),
        (h, m, _) => format!("{h} h {m} min"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(command: Option<&str>, exit_code: Option<i32>, duration_ms: Option<u64>) -> Block {
        Block {
            api: 1,
            index: 0,
            command: command.map(str::to_owned),
            cwd: None,
            exit_code,
            duration_ms,
            output: None,
        }
    }

    #[test]
    fn a_short_command_is_left_alone() {
        let mut plugin = LongCommands;

        assert!(
            plugin
                .on_block(&block(Some("ls"), Some(0), Some(10_000)))
                .is_empty()
        );
        assert!(
            plugin
                .on_block(&block(Some("ls"), Some(0), Some(3)))
                .is_empty()
        );
        assert!(
            plugin
                .on_block(&block(Some("ls"), Some(0), None))
                .is_empty()
        );
    }

    #[test]
    fn a_long_command_gets_a_label_and_a_message() {
        let mut plugin = LongCommands;

        let actions = plugin.on_block(&block(Some("cargo build"), Some(0), Some(12_400)));

        assert_eq!(
            actions,
            [
                label("slow", Tone::Warn),
                notify("cargo build finished in 12 s", Tone::Ok)
            ]
        );
    }

    #[test]
    fn a_long_command_that_failed_says_so() {
        let mut plugin = LongCommands;

        let actions = plugin.on_block(&block(Some("make"), Some(2), Some(75_000)));

        assert_eq!(
            actions[1],
            notify("make failed (exit 2) after 1 min 15 s", Tone::Err)
        );
    }

    #[test]
    fn a_command_without_a_text_is_still_reported() {
        let mut plugin = LongCommands;

        let actions = plugin.on_block(&block(None, None, Some(30_000)));

        assert_eq!(actions[1], notify("A command finished in 30 s", Tone::Ok));
    }

    #[test]
    fn durations_read_naturally() {
        assert_eq!(duration(10_001), "10 s");
        assert_eq!(duration(59_999), "59 s");
        assert_eq!(duration(60_000), "1 min");
        assert_eq!(duration(125_000), "2 min 5 s");
        assert_eq!(duration(3_600_000), "1 h");
        assert_eq!(duration(3_900_000), "1 h 5 min");
    }

    #[test]
    fn a_long_command_is_cut_and_put_on_one_line() {
        assert_eq!(shorten("echo   a\n b"), "echo a b");
        let long = "x".repeat(200);
        let cut = shorten(&long);
        assert_eq!(cut.chars().count(), COMMAND_CHARS);
        assert!(cut.ends_with('…'));
    }
}
