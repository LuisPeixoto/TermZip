//! The plugins that are installed and the user's approvals, kept in the data folder:
//! `plugins/<id>.wasm` for each plugin and `plugins/state.json` for what was enabled.
//!
//! An approval belongs to the **contents** of the file (its SHA-256), not to its name: a plugin that is
//! replaced or edited is disabled until the user enables it again, so what was approved is exactly what runs.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use termzip_core::{data_dir, write_atomic};
use termzip_plugin_api::{MAX_PLUGIN_BYTES, Manifest};

use crate::manifest::read_module;

/// How many plugins can be installed at once.
pub const MAX_PLUGINS: usize = 64;
const MAX_STATE_BYTES: u64 = 64 * 1024;
const STATE_FILE: &str = "state.json";

/// Where a plugin came from: a signed catalog, or a file the user chose (not verified by anyone).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Local,
    Catalog,
}

/// What the user decided about one plugin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Entry {
    enabled: bool,
    /// The hash of the file the user enabled; the plugin is only enabled while the file still has it.
    approved_sha256: Option<String>,
    source: Source,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct State {
    version: u32,
    plugins: BTreeMap<String, Entry>,
}

/// An installed plugin, as the interface shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginInfo {
    pub manifest: Manifest,
    pub sha256: String,
    pub size: u64,
    pub source: Source,
    /// Approved and unchanged since: only these run.
    pub enabled: bool,
    /// The file is not the one that was enabled, so the plugin was disabled.
    pub changed: bool,
}

/// What a scan of the folder found: the usable plugins, and a sentence for each file that was left out.
#[derive(Debug, Default, Serialize)]
pub struct Listing {
    pub plugins: Vec<PluginInfo>,
    pub problems: Vec<String>,
}

/// The folder the plugins live in.
pub fn plugins_dir() -> Result<PathBuf, String> {
    data_dir()
        .map(|dir| dir.join("plugins"))
        .map_err(|error| error.to_string())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The SHA-256 of some bytes, in lowercase hex: how a plugin file is identified.
pub fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

/// A regular file of at most `limit` bytes, read only after that is known.
fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let shown = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "That file".to_owned());
    let metadata =
        fs::metadata(path).map_err(|error| format!("Could not open “{shown}”: {error}."))?;
    if !metadata.is_file() {
        return Err(format!("“{shown}” is not a file."));
    }
    if metadata.len() > limit {
        return Err(format!("“{shown}” is larger than {} KiB.", limit / 1024));
    }
    fs::read(path).map_err(|error| format!("Could not read “{shown}”: {error}."))
}

/// Reads a plugin file the user picked: a regular file of at most 4 MiB, read only after that is known.
pub fn read_plugin_file(path: &Path) -> Result<Vec<u8>, String> {
    read_bounded(path, MAX_PLUGIN_BYTES as u64)
}

#[derive(Debug, Clone)]
pub struct PluginStore {
    dir: PathBuf,
}

impl PluginStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// The store in the data folder.
    pub fn in_data_dir() -> Result<Self, String> {
        plugins_dir().map(Self::new)
    }

    fn file(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.wasm"))
    }

    fn state(&self) -> State {
        // A state that cannot be read approves nothing: every plugin starts disabled.
        read_bounded(&self.dir.join(STATE_FILE), MAX_STATE_BYTES)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    fn save_state(&self, state: &State) -> Result<(), String> {
        let state = State {
            version: 1,
            plugins: state.plugins.clone(),
        };
        let text = serde_json::to_vec_pretty(&state).map_err(|error| error.to_string())?;
        write_atomic(&self.dir.join(STATE_FILE), &text).map_err(|error| error.to_string())
    }

    fn info(bytes: &[u8], manifest: Manifest, entry: Option<&Entry>) -> PluginInfo {
        let sha256 = sha256_hex(bytes);
        let approved = entry.and_then(|entry| entry.approved_sha256.as_deref());
        let enabled = entry.is_some_and(|entry| entry.enabled) && approved == Some(sha256.as_str());
        let changed = entry.is_some_and(|entry| entry.enabled) && approved != Some(sha256.as_str());
        PluginInfo {
            manifest,
            sha256,
            size: bytes.len() as u64,
            source: entry.map_or(Source::Local, |entry| entry.source),
            enabled,
            changed,
        }
    }

    /// Everything installed, by name.
    pub fn list(&self) -> Listing {
        let mut found = Listing::default();
        let Ok(entries) = fs::read_dir(&self.dir) else {
            return found; // no folder yet is the normal case
        };
        let state = self.state();
        let mut files: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "wasm")
            })
            .collect();
        files.sort();

        for path in files.into_iter().take(MAX_PLUGINS + 1) {
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            if found.plugins.len() >= MAX_PLUGINS {
                found.problems.push(format!(
                    "“{name}” was skipped: at most {MAX_PLUGINS} plugins are loaded."
                ));
                break;
            }
            let loaded = read_bounded(&path, MAX_PLUGIN_BYTES as u64)
                .and_then(|bytes| read_module(&bytes).map(|module| (bytes, module)));
            match loaded {
                Ok((bytes, module)) => {
                    let id = module.manifest.id.clone();
                    if path
                        .file_stem()
                        .is_none_or(|stem| stem.to_string_lossy() != id)
                    {
                        found.problems.push(format!(
                            "“{name}” was skipped: its file name does not match its id “{id}”."
                        ));
                        continue;
                    }
                    let info = Self::info(&bytes, module.manifest, state.plugins.get(&id));
                    if info.changed {
                        found.problems.push(format!(
                            "“{}” changed since you enabled it, so it was disabled. Enable it again if you trust the new file.",
                            info.manifest.name
                        ));
                    }
                    found.plugins.push(info);
                }
                Err(problem) => found.problems.push(format!("“{name}”: {problem}")),
            }
        }
        found.plugins.sort_by_cached_key(|info| {
            (info.manifest.name.to_lowercase(), info.manifest.id.clone())
        });
        found
    }

    /// Installs a plugin file. A plugin that is already installed is replaced; if the file is not
    /// the same one it is disabled, because the approval was for the old contents.
    pub fn install(&self, bytes: &[u8], source: Source) -> Result<PluginInfo, String> {
        let module = read_module(bytes)?;
        let id = module.manifest.id.clone();
        let existing = self.list().plugins;
        if existing.len() >= MAX_PLUGINS && !existing.iter().any(|info| info.manifest.id == id) {
            return Err(format!(
                "At most {MAX_PLUGINS} plugins can be installed. Uninstall one first."
            ));
        }
        write_atomic(&self.file(&id), bytes).map_err(|error| error.to_string())?;

        let mut state = self.state();
        let sha256 = sha256_hex(bytes);
        let entry = state.plugins.entry(id.clone()).or_insert(Entry {
            enabled: false,
            approved_sha256: None,
            source,
        });
        entry.source = source;
        if entry.approved_sha256.as_deref() != Some(sha256.as_str()) {
            entry.enabled = false;
            entry.approved_sha256 = None;
        }
        self.save_state(&state)?;
        Ok(Self::info(bytes, module.manifest, state.plugins.get(&id)))
    }

    fn installed(&self, id: &str) -> Result<(Vec<u8>, Manifest), String> {
        if id.is_empty()
            || !id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err("That is not the id of an installed plugin.".to_owned());
        }
        let bytes = read_bounded(&self.file(id), MAX_PLUGIN_BYTES as u64)
            .map_err(|_| "That plugin is not installed.".to_owned())?;
        let module = read_module(&bytes)?;
        if module.manifest.id != id {
            return Err("That plugin's file does not match its id.".to_owned());
        }
        Ok((bytes, module.manifest))
    }

    /// Approves the plugin as it is now. The interface calls this only after showing what it may do.
    pub fn enable(&self, id: &str) -> Result<PluginInfo, String> {
        let (bytes, manifest) = self.installed(id)?;
        let mut state = self.state();
        let entry = state.plugins.entry(id.to_owned()).or_insert(Entry {
            enabled: false,
            approved_sha256: None,
            source: Source::Local,
        });
        entry.enabled = true;
        entry.approved_sha256 = Some(sha256_hex(&bytes));
        self.save_state(&state)?;
        Ok(Self::info(&bytes, manifest, state.plugins.get(id)))
    }

    pub fn disable(&self, id: &str) -> Result<(), String> {
        self.installed(id)?;
        let mut state = self.state();
        if let Some(entry) = state.plugins.get_mut(id) {
            entry.enabled = false;
            entry.approved_sha256 = None;
        }
        self.save_state(&state)
    }

    /// Deletes the plugin and what was decided about it.
    pub fn uninstall(&self, id: &str) -> Result<(), String> {
        self.installed(id)?;
        fs::remove_file(self.file(id)).map_err(|error| format!("Could not delete it: {error}."))?;
        let mut state = self.state();
        state.plugins.remove(id);
        self.save_state(&state)
    }

    /// The bytes of an **enabled** plugin, for the runtime. A plugin that is not approved, or whose file
    /// is not the approved one, is never handed over.
    pub fn approved_bytes(&self, id: &str) -> Result<Vec<u8>, String> {
        let (bytes, _) = self.installed(id)?;
        let state = self.state();
        let approved = state
            .plugins
            .get(id)
            .filter(|entry| entry.enabled)
            .and_then(|entry| entry.approved_sha256.as_deref());
        if approved != Some(sha256_hex(&bytes).as_str()) {
            return Err("That plugin is not enabled.".to_owned());
        }
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::fixtures::{MINIMAL, minimal, plugin};

    fn temp_store(name: &str) -> (PluginStore, PathBuf) {
        let dir =
            std::env::temp_dir().join(format!("termzip-plugins-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        (PluginStore::new(&dir), dir)
    }

    fn variant(id: &str, version: &str) -> Vec<u8> {
        plugin(
            &MINIMAL
                .replace("\"hello\"", &format!("\"{id}\""))
                .replace("1.0.0", version),
            "",
        )
    }

    #[test]
    fn a_missing_folder_is_just_an_empty_list() {
        let (store, _) = temp_store("missing");
        let listing = store.list();
        assert!(listing.plugins.is_empty() && listing.problems.is_empty());
    }

    #[test]
    fn an_installed_plugin_is_listed_and_starts_disabled() {
        let (store, dir) = temp_store("install");
        let info = store.install(&minimal(), Source::Local).unwrap();

        assert_eq!(info.manifest.id, "hello");
        assert!(!info.enabled && !info.changed);
        assert_eq!(info.source, Source::Local);
        assert_eq!(info.sha256.len(), 64);
        let listing = store.list();
        assert_eq!(listing.plugins.len(), 1);
        assert!(dir.join("hello.wasm").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn enabling_approves_the_exact_file() {
        let (store, dir) = temp_store("enable");
        store.install(&minimal(), Source::Catalog).unwrap();
        let info = store.enable("hello").unwrap();

        assert!(info.enabled);
        assert_eq!(info.source, Source::Catalog);
        assert_eq!(store.approved_bytes("hello").unwrap(), minimal());
        store.disable("hello").unwrap();
        assert!(!store.list().plugins[0].enabled);
        assert_eq!(
            store.approved_bytes("hello").unwrap_err(),
            "That plugin is not enabled."
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_plugin_that_was_edited_is_disabled_and_says_so() {
        let (store, dir) = temp_store("edited");
        store.install(&minimal(), Source::Local).unwrap();
        store.enable("hello").unwrap();

        // Someone changes the file behind TermZip's back.
        fs::write(dir.join("hello.wasm"), variant("hello", "1.0.1")).unwrap();

        let listing = store.list();
        assert!(!listing.plugins[0].enabled);
        assert!(listing.plugins[0].changed);
        assert!(
            listing.problems[0].contains("changed since you enabled it"),
            "{:?}",
            listing.problems
        );
        assert!(
            store.approved_bytes("hello").is_err(),
            "the changed file must never run"
        );
        // Enabling it again approves the new contents.
        assert!(store.enable("hello").unwrap().enabled);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn replacing_a_plugin_with_a_different_file_disables_it() {
        let (store, dir) = temp_store("replace");
        store.install(&minimal(), Source::Catalog).unwrap();
        store.enable("hello").unwrap();

        // The same file again keeps the approval.
        assert!(store.install(&minimal(), Source::Catalog).unwrap().enabled);
        // A new version needs a new approval.
        let updated = store
            .install(&variant("hello", "1.1.0"), Source::Catalog)
            .unwrap();
        assert!(!updated.enabled && !updated.changed);
        assert_eq!(updated.manifest.version, "1.1.0");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn uninstalling_removes_the_file_and_the_decision() {
        let (store, dir) = temp_store("uninstall");
        store.install(&minimal(), Source::Local).unwrap();
        store.enable("hello").unwrap();
        store.uninstall("hello").unwrap();

        assert!(store.list().plugins.is_empty());
        assert!(!dir.join("hello.wasm").exists());
        // A plugin installed again later does not inherit the old approval.
        assert!(!store.install(&minimal(), Source::Local).unwrap().enabled);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn what_is_not_a_plugin_is_never_installed_or_listed() {
        let (store, dir) = temp_store("bad");
        assert!(store.install(b"nope", Source::Local).is_err());
        assert!(!dir.join("nope.wasm").exists());

        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("broken.wasm"), b"\0asm garbage").unwrap();
        fs::write(dir.join("notes.txt"), "ignored").unwrap();
        fs::create_dir_all(dir.join("sneaky.wasm")).unwrap();
        // A valid plugin under the wrong name.
        fs::write(dir.join("other.wasm"), minimal()).unwrap();

        let listing = store.list();
        assert!(listing.plugins.is_empty());
        assert_eq!(listing.problems.len(), 3, "{:?}", listing.problems);
        assert!(
            listing
                .problems
                .iter()
                .any(|p| p.contains("file name does not match"))
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn ids_that_could_leave_the_folder_are_refused() {
        let (store, dir) = temp_store("ids");
        store.install(&minimal(), Source::Local).unwrap();
        for id in [
            "",
            "../hello",
            "hello/../hello",
            "Hello",
            "hello.wasm",
            "/etc/passwd",
        ] {
            assert!(store.enable(id).is_err(), "{id:?}");
            assert!(store.uninstall(id).is_err(), "{id:?}");
        }
        assert!(dir.join("hello.wasm").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_damaged_state_file_approves_nothing() {
        let (store, dir) = temp_store("state");
        store.install(&minimal(), Source::Local).unwrap();
        store.enable("hello").unwrap();
        fs::write(dir.join("state.json"), "{ not json").unwrap();

        assert!(!store.list().plugins[0].enabled);
        assert!(store.approved_bytes("hello").is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn only_so_many_plugins_can_be_installed() {
        let (store, dir) = temp_store("many");
        for n in 0..MAX_PLUGINS {
            store
                .install(&variant(&format!("p{n}"), "1.0.0"), Source::Local)
                .unwrap();
        }
        let error = store
            .install(&variant("one-more", "1.0.0"), Source::Local)
            .unwrap_err();
        assert!(error.contains("Uninstall one first"));
        // Updating one that is there is still fine.
        assert!(
            store
                .install(&variant("p3", "2.0.0"), Source::Local)
                .is_ok()
        );
        assert_eq!(store.list().plugins.len(), MAX_PLUGINS);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_list_is_in_name_order() {
        let (store, dir) = temp_store("order");
        for (id, name) in [("zeta", "Zeta"), ("alpha", "alpha"), ("mid", "Middle")] {
            let manifest = MINIMAL
                .replace("\"hello\"", &format!("\"{id}\""))
                .replace("\"Hello\"", &format!("\"{name}\""));
            store
                .install(&plugin(&manifest, ""), Source::Local)
                .unwrap();
        }
        let ids: Vec<_> = store
            .list()
            .plugins
            .iter()
            .map(|p| p.manifest.id.clone())
            .collect();
        assert_eq!(ids, ["alpha", "mid", "zeta"]);
        let _ = fs::remove_dir_all(&dir);
    }
}
