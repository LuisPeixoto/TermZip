//! The tasks of the project a folder belongs to: the scripts of `package.json`, Cargo's commands and aliases, the
//! targets of a Makefile, the recipes of a justfile and the tasks of `deno.json`.
//!
//! The tasks are found from the folder of a terminal and **typed at its prompt**, so each one is the command a
//! person would write there. Which file counts follows how the tool itself looks for it: `npm`, `cargo`, `just`
//! and `deno` walk up the folders and use the nearest file, so the nearest one is read (up to the repository
//! root); `make` reads only the Makefile of the current folder, so only that one is.
//!
//! The files of a repository you cloned are not trusted: a file must be a regular file of at most
//! [`MAX_FILE_BYTES`], a name must be plain (see [`parse::is_plain_name`]), and at most [`MAX_TASKS`] come back.

mod parse;

use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

use serde::Serialize;

pub use parse::{
    MAX_NAME_CHARS, cargo_aliases, deno_tasks, is_plain_name, justfile_recipes, makefile_targets,
    package_manager_field, package_scripts,
};

/// The biggest file that is read.
pub const MAX_FILE_BYTES: u64 = 256 * 1024;
/// The most tasks that come back.
pub const MAX_TASKS: usize = 200;
/// How many folders up from the starting one are looked at.
pub const MAX_LEVELS: usize = 8;

/// A command a project offers, as it is typed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Task {
    /// The file or tool it comes from: `package.json`, `Cargo`, `Makefile`, `justfile` or `deno.json`.
    pub source: &'static str,
    /// The name inside that file.
    pub name: String,
    /// What to type: `npm run build`, `cargo test`, `make lint`.
    pub command: String,
}

/// The text of a regular file of a bounded size, or `None`. A name that is a pipe or a device is never opened
/// (opening a pipe would wait for ever), and a file that is too big is not read at all.
fn read_text(path: &Path) -> Option<String> {
    let metadata = fs::metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_FILE_BYTES {
        return None;
    }
    let mut bytes = Vec::new();
    File::open(path)
        .ok()?
        .take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return None;
    }
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

/// The folders to look at, nearest first: `dir` and its parents up to the one with a `.git` (the repository
/// root, included), at most [`MAX_LEVELS`] of them.
fn ancestors(dir: &Path) -> Vec<&Path> {
    let mut out = Vec::new();
    for folder in dir.ancestors().take(MAX_LEVELS) {
        out.push(folder);
        if folder.join(".git").exists() {
            break;
        }
    }
    out
}

/// The nearest file with one of `names`, with its text.
fn nearest<'a>(folders: &[&'a Path], names: &[&str]) -> Option<(&'a Path, String)> {
    folders.iter().find_map(|folder| {
        names
            .iter()
            .find_map(|name| read_text(&folder.join(name)))
            .map(|text| (*folder, text))
    })
}

/// The package manager of a folder with a `package.json`: the `packageManager` field wins, then the lock file.
fn package_manager(folder: &Path, package_json: &str) -> &'static str {
    if let Some(manager) = parse::package_manager_field(package_json) {
        return manager;
    }
    let has = |name: &str| folder.join(name).is_file();
    if has("pnpm-lock.yaml") {
        "pnpm"
    } else if has("yarn.lock") {
        "yarn"
    } else if has("bun.lockb") || has("bun.lock") {
        "bun"
    } else {
        "npm"
    }
}

/// Cargo's own commands, offered wherever a `Cargo.toml` is found.
const CARGO_COMMANDS: [&str; 6] = ["build", "test", "run", "check", "clippy", "fmt"];

/// The tasks of the project `dir` is in, grouped by file in a fixed order (`package.json`, Cargo, Makefile,
/// justfile, `deno.json`). A folder that does not exist, or has nothing, gives none.
pub fn detect(dir: &Path) -> Vec<Task> {
    let folders = ancestors(dir);
    let mut tasks = Vec::new();

    if let Some((folder, text)) = nearest(&folders, &["package.json"]) {
        let manager = package_manager(folder, &text);
        for name in parse::package_scripts(&text) {
            tasks.push(Task {
                source: "package.json",
                command: format!("{manager} run {name}"),
                name,
            });
        }
    }

    if let Some((folder, _)) = nearest(&folders, &["Cargo.toml"]) {
        for name in CARGO_COMMANDS {
            tasks.push(Task {
                source: "Cargo",
                name: name.to_owned(),
                command: format!("cargo {name}"),
            });
        }
        // Aliases come from the nearest config of the folders from here up, as Cargo merges them.
        let config = [".cargo/config.toml", ".cargo/config"];
        if let Some((_, text)) = nearest(
            &folders[folders.iter().position(|f| *f == folder).unwrap_or(0)..],
            &config,
        ) {
            for name in parse::cargo_aliases(&text) {
                if !CARGO_COMMANDS.contains(&name.as_str()) {
                    tasks.push(Task {
                        source: "Cargo",
                        command: format!("cargo {name}"),
                        name,
                    });
                }
            }
        }
    }

    // `make` reads the Makefile of the current folder only.
    if let Some(text) = ["GNUmakefile", "makefile", "Makefile"]
        .iter()
        .find_map(|name| read_text(&dir.join(name)))
    {
        for name in parse::makefile_targets(&text) {
            tasks.push(Task {
                source: "Makefile",
                command: format!("make {name}"),
                name,
            });
        }
    }

    if let Some((_, text)) = nearest(&folders, &["justfile", "Justfile", ".justfile"]) {
        for name in parse::justfile_recipes(&text) {
            tasks.push(Task {
                source: "justfile",
                command: format!("just {name}"),
                name,
            });
        }
    }

    if let Some((_, text)) = nearest(&folders, &["deno.json"]) {
        for name in parse::deno_tasks(&text) {
            tasks.push(Task {
                source: "deno.json",
                command: format!("deno task {name}"),
                name,
            });
        }
    }

    tasks.truncate(MAX_TASKS);
    tasks
}
