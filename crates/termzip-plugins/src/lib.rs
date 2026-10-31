//! Loading, approving and running TermZip plugins.
//!
//! A plugin is one WebAssembly file (see `termzip-plugin-api`). This crate reads its manifest and checks it,
//! keeps the installed plugins and the user's approvals in the data folder, and runs the approved ones in
//! a sandbox. It has no platform code, so it builds and is tested on every system.

mod manifest;
mod sanitize;
mod store;

pub use manifest::{Module, read_module, validate_manifest};
pub use sanitize::{Checked, check_actions, clean_line, clean_typed, is_safe_url};
pub use store::{Listing, MAX_PLUGINS, PluginInfo, PluginStore, Source, plugins_dir};
