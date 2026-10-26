//! Shell integration for TermZip.
//!
//! The parser recognizes the semantic-prompt protocol (OSC 133) plus two private
//! extensions sent by the bundled integration scripts: `1337;CurrentDir=` and
//! `1337;CommandB64=`. Bytes flow through unchanged; this crate only extracts
//! markers so the session layer can route regions and build command blocks.

pub mod blocks;
pub mod launch;
pub mod marker;
pub mod scripts;

pub use blocks::{Block, BlockTracker};
pub use launch::{ShellKind, default_shell, detect_shell_kind, launch_config};
pub use marker::{Marker, OscParser, strip_ansi};
