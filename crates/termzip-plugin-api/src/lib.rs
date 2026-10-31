//! The plugin API of TermZip, version 1.
//!
//! A plugin is one WebAssembly file. Its manifest travels inside it, in a custom section, and what it
//! exchanges with TermZip is JSON in both directions: a pure function from an event to a list of
//! actions. This crate holds only those types and the limits that go with them, so the host and the
//! plugin SDK read the same definitions. Validating a manifest and running a plugin is the host's job
//! (`termzip-plugins`).

mod manifest;
mod wire;

pub use manifest::{
    API_VERSION, MANIFEST_SECTION, MAX_COMMANDS, MAX_LINKS, MAX_MANIFEST_BYTES, MAX_NAME_CHARS,
    MAX_PATTERN_CHARS, MAX_PLUGIN_BYTES,
};
pub use manifest::{CommandSpec, LinkSpec, Manifest, Permission};
pub use wire::{Action, BlockEvent, CommandCall, Reply, Tone};
pub use wire::{
    MAX_LABEL_CHARS, MAX_NOTIFY_CHARS, MAX_OUTPUT_BYTES, MAX_REPLY_BYTES, MAX_STATUS_CHARS,
    MAX_TYPE_BYTES,
};
