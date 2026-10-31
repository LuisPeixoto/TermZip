//! Palette commands that type a snippet into the terminal. The plugin never presses Enter and the app would not
//! let it: the text waits at the prompt for you to read, edit and run.
//!
//! It needs the `type` permission. Add a snippet by adding a `[[commands]]` entry to `plugin.toml` and a line to
//! `SNIPPETS`.

use termzip_plugin_sdk::prelude::*;

termzip_plugin_sdk::manifest!("../plugin.toml");

/// The id of each command in `plugin.toml` and the text it types.
const SNIPPETS: [(&str, &str); 4] = [
    ("git-status", "git status"),
    ("git-log", "git log --oneline --graph --decorate -20"),
    ("cargo-clippy", "cargo clippy --all-targets -- -D warnings"),
    ("list-files", "ls -lah"),
];

#[derive(Default)]
struct Snippets;

impl Plugin for Snippets {
    fn run_command(&mut self, call: &Command) -> Vec<Action> {
        SNIPPETS
            .iter()
            .find(|(id, _)| *id == call.command)
            .map(|(_, text)| vec![type_text(*text)])
            .unwrap_or_default()
    }
}

termzip_plugin_sdk::export!(Snippets);

#[cfg(test)]
mod tests {
    use super::*;

    fn call(command: &str) -> Command {
        Command {
            api: 1,
            command: command.to_owned(),
            cwd: Some("/w".to_owned()),
        }
    }

    #[test]
    fn every_command_types_its_snippet() {
        let mut plugin = Snippets;

        assert_eq!(
            plugin.run_command(&call("git-status")),
            [type_text("git status")]
        );
        assert_eq!(
            plugin.run_command(&call("list-files")),
            [type_text("ls -lah")]
        );
    }

    #[test]
    fn an_unknown_command_types_nothing() {
        assert!(Snippets.run_command(&call("rm-everything")).is_empty());
    }

    #[test]
    fn a_snippet_never_has_a_newline_that_would_run_it() {
        for (id, text) in SNIPPETS {
            assert!(!text.contains(['\n', '\r']), "{id}");
            assert!(text.len() <= termzip_plugin_sdk::MAX_TYPE_BYTES, "{id}");
        }
    }

    #[test]
    fn the_ids_are_not_repeated() {
        let mut ids: Vec<_> = SNIPPETS.iter().map(|(id, _)| *id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), SNIPPETS.len());
    }
}
