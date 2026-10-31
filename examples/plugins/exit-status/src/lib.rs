//! A red label with the exit code on every command that failed, and "N failed" in the status bar.
//!
//! It only needs the `blocks` permission: labels and the status item show what the plugin returns, so they
//! need none. The count lives in the plugin and starts again from zero each time TermZip starts.

use termzip_plugin_sdk::prelude::*;

termzip_plugin_sdk::manifest!("../plugin.toml");

/// What a shell reports for a command stopped with Ctrl+C (128 + SIGINT). It is not a failure of the command.
const INTERRUPTED: i32 = 130;

#[derive(Default)]
struct ExitStatus {
    failed: u32,
}

impl Plugin for ExitStatus {
    fn on_block(&mut self, block: &Block) -> Vec<Action> {
        match block.exit_code {
            None | Some(0) => Vec::new(),
            Some(INTERRUPTED) => vec![label("interrupted", Tone::Warn)],
            Some(code) => {
                self.failed = self.failed.saturating_add(1);
                vec![
                    label(format!("exit {code}"), Tone::Err),
                    status(format!("{} failed", self.failed)),
                ]
            }
        }
    }
}

termzip_plugin_sdk::export!(ExitStatus);

#[cfg(test)]
mod tests {
    use super::*;

    fn finished(exit_code: Option<i32>) -> Block {
        Block {
            api: 1,
            index: 0,
            command: Some("make".to_owned()),
            cwd: None,
            exit_code,
            duration_ms: Some(5),
            output: None,
        }
    }

    #[test]
    fn a_command_that_worked_is_left_alone() {
        let mut plugin = ExitStatus::default();

        assert!(plugin.on_block(&finished(Some(0))).is_empty());
        assert!(plugin.on_block(&finished(None)).is_empty());
    }

    #[test]
    fn a_failure_gets_its_exit_code_and_is_counted() {
        let mut plugin = ExitStatus::default();

        assert_eq!(
            plugin.on_block(&finished(Some(2))),
            [label("exit 2", Tone::Err), status("1 failed")]
        );
        assert_eq!(
            plugin.on_block(&finished(Some(1))),
            [label("exit 1", Tone::Err), status("2 failed")]
        );
    }

    #[test]
    fn a_command_stopped_with_ctrl_c_is_marked_but_not_counted() {
        let mut plugin = ExitStatus::default();

        assert_eq!(
            plugin.on_block(&finished(Some(INTERRUPTED))),
            [label("interrupted", Tone::Warn)]
        );
        assert_eq!(plugin.failed, 0);
    }

    #[test]
    fn a_label_fits_the_limit_for_any_exit_code() {
        let mut plugin = ExitStatus::default();

        for code in [i32::MIN, -1, 255, i32::MAX] {
            let actions = plugin.on_block(&finished(Some(code)));
            let Action::Label { text, .. } = &actions[0] else {
                panic!("a label first")
            };
            assert!(
                text.chars().count() <= termzip_plugin_sdk::MAX_LABEL_CHARS,
                "{text}"
            );
        }
    }
}
