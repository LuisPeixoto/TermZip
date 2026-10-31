//! Loading, approving and running TermZip plugins.
//!
//! A plugin is one WebAssembly file (see `termzip-plugin-api`). This crate reads its manifest and checks it,
//! keeps the installed plugins and the user's approvals in the data folder, and runs the approved ones in
//! a sandbox. It has no platform code, so it builds and is tested on every system.

mod catalog;
mod links;
mod manifest;
mod runtime;
mod sandbox;
mod sanitize;
mod store;

pub use catalog::{
    Catalog, CatalogEntry, CatalogSource, Fetch, HttpFetch, MAX_CATALOG_PLUGINS, MAX_INDEX_BYTES,
    download, is_newer, load_catalog, parse_index, validate_entry, verify_and_parse,
};
pub use links::{LinkMatch, LinkMatcher};
pub use manifest::{Module, read_module, validate_manifest};
pub use runtime::{MAX_STRIKES, QUEUE_LEN, Report, Runtime, Sink, Trigger};
pub use sandbox::{FUEL_PER_CALL, Fault, MAX_MEMORY_BYTES, Sandbox, parse_reply};
pub use sanitize::{Checked, check_actions, clean_line, clean_typed, is_safe_url};
pub use store::{
    Listing, MAX_PLUGINS, PluginInfo, PluginStore, Source, plugins_dir, read_plugin_file,
};
