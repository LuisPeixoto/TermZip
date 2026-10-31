//! Links in the output, and nothing else: all of this plugin is in `plugin.toml`.
//!
//! The patterns are regular expressions and the app does the matching, so there is no code to run. The plugin is
//! still a WebAssembly file (that is what a plugin is), and this is all it takes to make one.

use termzip_plugin_sdk::prelude::*;

termzip_plugin_sdk::manifest!("../plugin.toml");

#[derive(Default)]
struct IssueLinks;

impl Plugin for IssueLinks {}

termzip_plugin_sdk::export!(IssueLinks);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_never_has_anything_to_say() {
        let mut plugin = IssueLinks;
        let call = Command {
            api: 1,
            command: "none".to_owned(),
            cwd: None,
        };

        assert!(plugin.run_command(&call).is_empty());
    }
}
