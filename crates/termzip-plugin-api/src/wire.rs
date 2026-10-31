//! What is exchanged with a plugin: JSON in, JSON out, as a pure function.
//!
//! The host calls an export with an event and gets back a [`Reply`]: a list of [`Action`]s. The names of
//! the fields are `snake_case` and part of the API; a plugin written in any language reads and writes
//! them. New optional fields may appear within an API version, so a reader must ignore what it does not
//! know.

use serde::{Deserialize, Serialize};

/// The longest label a plugin may put on a block, in characters.
pub const MAX_LABEL_CHARS: usize = 24;
pub const MAX_STATUS_CHARS: usize = 48;
pub const MAX_NOTIFY_CHARS: usize = 200;
/// The most a plugin may type into the terminal for one command, in bytes.
pub const MAX_TYPE_BYTES: usize = 4 * 1024;
/// A reply is dropped whole when it is bigger than this.
pub const MAX_REPLY_BYTES: usize = 64 * 1024;
/// How much of a command's output a plugin that asked for it receives, in bytes.
pub const MAX_OUTPUT_BYTES: usize = 64 * 1024;

/// A finished command, sent to `tz_on_block` for a plugin with the `blocks` permission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockEvent {
    pub api: u32,
    /// Counts the commands of one terminal from 0; it also tells the block apart from the others.
    pub index: usize,
    pub command: Option<String>,
    pub cwd: Option<String>,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<u64>,
    /// Only for a plugin with the `output` permission, cut to [`MAX_OUTPUT_BYTES`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
}

/// The user ran one of the plugin's palette commands, sent to `tz_run_command`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandCall {
    pub api: u32,
    /// The `id` of the command in the manifest.
    pub command: String,
    /// The folder of the terminal the command was run from.
    pub cwd: Option<String>,
}

/// How a label or a message looks.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tone {
    #[default]
    Info,
    Ok,
    Warn,
    Err,
}

/// Something a plugin asks the host to do. The host checks every one against the plugin's permissions
/// and limits, and drops what is not allowed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// A small badge on the block the event was about. Plain text, up to [`MAX_LABEL_CHARS`].
    Label {
        text: String,
        #[serde(default)]
        tone: Tone,
    },
    /// A short message for the user. Needs the `notify` permission. Plain text, up to [`MAX_NOTIFY_CHARS`].
    Notify {
        text: String,
        #[serde(default)]
        tone: Tone,
    },
    /// The text of the plugin's item in the status bar, up to [`MAX_STATUS_CHARS`]; an empty text removes it.
    Status(String),
    /// Text typed into the terminal, without Enter. Only as the answer to a command the user ran, and
    /// needs the `type` permission. Control characters and line breaks are removed.
    Type(String),
    /// Opens an `http` or `https` address in the browser. Only as the answer to a command the user ran,
    /// and needs the `open` permission.
    Open(String),
}

/// What a plugin answers.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reply {
    #[serde(default)]
    pub actions: Vec<Action>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn json<T: Serialize>(value: &T) -> String {
        serde_json::to_string(value).unwrap()
    }

    // These strings are the ABI: a plugin in any language writes and reads exactly this.

    #[test]
    fn a_finished_command_is_sent_like_this() {
        let event = BlockEvent {
            api: 1,
            index: 12,
            command: Some("cargo build".to_owned()),
            cwd: Some("/work/app".to_owned()),
            exit_code: Some(101),
            duration_ms: Some(48_200),
            output: None,
        };

        assert_eq!(
            json(&event),
            r#"{"api":1,"index":12,"command":"cargo build","cwd":"/work/app","exit_code":101,"duration_ms":48200}"#
        );
        let with_output = BlockEvent {
            output: Some("error[E0308]".to_owned()),
            ..event
        };
        assert!(json(&with_output).ends_with(r#","output":"error[E0308]"}"#));
    }

    #[test]
    fn a_command_the_user_ran_is_sent_like_this() {
        let call = CommandCall {
            api: 1,
            command: "say-hello".to_owned(),
            cwd: None,
        };

        assert_eq!(json(&call), r#"{"api":1,"command":"say-hello","cwd":null}"#);
    }

    #[test]
    fn every_action_has_one_shape() {
        let reply = Reply {
            actions: vec![
                Action::Label {
                    text: "slow".to_owned(),
                    tone: Tone::Warn,
                },
                Action::Notify {
                    text: "Done".to_owned(),
                    tone: Tone::Ok,
                },
                Action::Status("3 failed".to_owned()),
                Action::Type("git status".to_owned()),
                Action::Open("https://example.com".to_owned()),
            ],
        };

        let text = json(&reply);
        assert_eq!(
            text,
            r#"{"actions":[{"label":{"text":"slow","tone":"warn"}},{"notify":{"text":"Done","tone":"ok"}},{"status":"3 failed"},{"type":"git status"},{"open":"https://example.com"}]}"#
        );
        assert_eq!(serde_json::from_str::<Reply>(&text).unwrap(), reply);
    }

    #[test]
    fn a_tone_may_be_left_out_and_unknown_fields_are_ignored() {
        let reply: Reply = serde_json::from_str(
            r#"{"actions":[{"label":{"text":"x"}},{"notify":{"text":"y","icon":"bell"}}],"future":true}"#,
        )
        .unwrap();

        assert_eq!(
            reply.actions,
            [
                Action::Label {
                    text: "x".to_owned(),
                    tone: Tone::Info
                },
                Action::Notify {
                    text: "y".to_owned(),
                    tone: Tone::Info
                },
            ]
        );
        assert_eq!(
            serde_json::from_str::<Reply>("{}").unwrap(),
            Reply::default()
        );
    }

    #[test]
    fn an_action_of_a_kind_this_version_does_not_know_is_an_error_here() {
        // The host reads replies action by action and skips such a one; the type itself is strict.
        assert!(serde_json::from_str::<Action>(r#"{"vibrate":"long"}"#).is_err());
        assert!(serde_json::from_str::<Action>(r#"{"label":{"tone":"ok"}}"#).is_err());
        assert!(
            serde_json::from_str::<Action>(r#"{"label":{"text":"x","tone":"purple"}}"#).is_err()
        );
    }

    #[test]
    fn the_limits_are_part_of_the_contract() {
        assert_eq!(MAX_LABEL_CHARS, 24);
        assert_eq!(MAX_STATUS_CHARS, 48);
        assert_eq!(MAX_NOTIFY_CHARS, 200);
        assert_eq!(MAX_TYPE_BYTES, 4096);
        assert_eq!(MAX_REPLY_BYTES, 64 * 1024);
        assert_eq!(MAX_OUTPUT_BYTES, 64 * 1024);
    }
}
