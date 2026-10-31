//! The manifest a plugin carries in the custom section named [`MANIFEST_SECTION`] of its `.wasm` file,
//! written as TOML.

use serde::{Deserialize, Serialize};

/// The version of the API this crate describes. A plugin names the one it was written for.
pub const API_VERSION: u32 = 1;

/// The custom section of the WebAssembly module that holds the manifest.
pub const MANIFEST_SECTION: &str = "termzip-plugin";

/// A plugin file is at most this big.
pub const MAX_PLUGIN_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_MANIFEST_BYTES: usize = 16 * 1024;
pub const MAX_COMMANDS: usize = 32;
pub const MAX_LINKS: usize = 16;
/// Applies to names, titles and the other short texts of a manifest.
pub const MAX_NAME_CHARS: usize = 64;
pub const MAX_PATTERN_CHARS: usize = 256;

/// What a plugin may ask for. There is nothing else: no files, no network, no processes, no clock.
/// Commands in the palette, labels on blocks, status items and link patterns need no permission,
/// because they only show what the plugin returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Permission {
    /// Receive each finished command: its text, exit code, duration and folder.
    Blocks,
    /// Also receive what the command printed. Needs [`Permission::Blocks`].
    Output,
    /// Type text into the active terminal when the user runs one of the plugin's commands.
    Type,
    /// Show the user short messages.
    Notify,
}

impl Permission {
    /// The sentence shown to the user before a plugin is enabled.
    pub fn describe(self) -> &'static str {
        match self {
            Self::Blocks => "See each command you run: its text, exit code, duration and folder.",
            Self::Output => "Also see what each command printed.",
            Self::Type => {
                "Type text into the terminal when you run one of its commands. It never presses Enter."
            }
            Self::Notify => "Show you short messages.",
        }
    }
}

/// A command the plugin adds to the command palette.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandSpec {
    /// Passed back to the plugin when the user runs the command (`a-z`, `0-9`, `-`).
    pub id: String,
    pub title: String,
}

/// A pattern in the terminal output that becomes a link.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkSpec {
    /// A regular expression (the `regex` crate's syntax; no look-around, no back-references).
    pub pattern: String,
    /// Where the link goes: `http` or `https`, with `{0}` for the whole match and `{1}`, `{2}`… for groups.
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    /// The API version the plugin was written for.
    pub api: u32,
    /// Stable identifier (`a-z`, `0-9`, `-`); it names the plugin's file and its approval.
    pub id: String,
    pub name: String,
    /// `major.minor.patch`.
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub permissions: Vec<Permission>,
    #[serde(default)]
    pub commands: Vec<CommandSpec>,
    #[serde(default)]
    pub links: Vec<LinkSpec>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = r##"
api = 1
id = "long-command-notify"
name = "Long command notify"
version = "1.2.0"
description = "Tells you when a long command ends."
author = "Someone"
license = "MIT"
homepage = "https://example.com/plugin"
permissions = ["blocks", "notify"]

[[commands]]
id = "say-hello"
title = "Say hello"

[[links]]
pattern = "#([0-9]+)"
url = "https://github.com/me/repo/issues/{1}"
"##;

    #[test]
    fn a_manifest_is_read_with_everything_it_can_carry() {
        let manifest: Manifest = toml::from_str(EXAMPLE).unwrap();

        assert_eq!(manifest.api, API_VERSION);
        assert_eq!(manifest.id, "long-command-notify");
        assert_eq!(
            manifest.permissions,
            [Permission::Blocks, Permission::Notify]
        );
        assert_eq!(manifest.commands[0].title, "Say hello");
        assert_eq!(
            manifest.links[0].url,
            "https://github.com/me/repo/issues/{1}"
        );
        assert_eq!(manifest.author.as_deref(), Some("Someone"));
    }

    #[test]
    fn only_the_identity_is_required() {
        let manifest: Manifest =
            toml::from_str("api = 1\nid = \"tiny\"\nname = \"Tiny\"\nversion = \"0.1.0\"\n")
                .unwrap();

        assert!(manifest.permissions.is_empty());
        assert!(manifest.commands.is_empty() && manifest.links.is_empty());
        assert_eq!(manifest.description, "");
        assert_eq!(manifest.author, None);
    }

    #[test]
    fn a_manifest_that_misses_its_identity_or_names_an_unknown_permission_is_refused() {
        assert!(
            toml::from_str::<Manifest>("api = 1\nname = \"x\"\nversion = \"1.0.0\"\n").is_err()
        );
        assert!(
            toml::from_str::<Manifest>("id = \"x\"\nname = \"x\"\nversion = \"1.0.0\"\n").is_err()
        );
        let unknown = EXAMPLE.replace("\"notify\"", "\"files\"");
        assert!(toml::from_str::<Manifest>(&unknown).is_err());
    }

    #[test]
    fn a_manifest_survives_a_round_trip_through_toml() {
        let manifest: Manifest = toml::from_str(EXAMPLE).unwrap();
        let text = toml::to_string_pretty(&manifest).unwrap();

        assert_eq!(toml::from_str::<Manifest>(&text).unwrap(), manifest);
    }

    #[test]
    fn every_permission_has_a_sentence_for_the_confirmation() {
        for permission in [
            Permission::Blocks,
            Permission::Output,
            Permission::Type,
            Permission::Notify,
        ] {
            assert!(permission.describe().ends_with('.'), "{permission:?}");
        }
        // Typing is the sensitive one, and says it never runs anything.
        assert!(Permission::Type.describe().contains("never presses Enter"));
    }

    #[test]
    fn the_section_name_and_the_limits_are_part_of_the_contract() {
        assert_eq!(MANIFEST_SECTION, "termzip-plugin");
        assert_eq!(API_VERSION, 1);
        assert_eq!(MAX_PLUGIN_BYTES, 4 * 1024 * 1024);
        assert_eq!(MAX_MANIFEST_BYTES, 16 * 1024);
    }
}
