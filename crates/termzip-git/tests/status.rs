//! The parser on real `git status` output (captured from throwaway repositories), and the runner on a real git.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use termzip_git::{GitStatus, parse_status, status};

fn fixture(name: &str) -> GitStatus {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    parse_status(&fs::read(path).unwrap()).unwrap_or_else(|| panic!("{name} is not a status"))
}

#[test]
fn a_clean_branch_in_sync_with_its_upstream() {
    let status = fixture("clean.bin");

    assert_eq!(status.branch.as_deref(), Some("main"));
    assert!(status.has_upstream);
    assert_eq!((status.ahead, status.behind), (0, 0));
    assert!(!status.is_dirty());
}

#[test]
fn a_branch_ahead_of_its_upstream() {
    let status = fixture("ahead.bin");

    assert_eq!((status.ahead, status.behind), (2, 0));
    assert!(!status.is_dirty());
}

#[test]
fn a_busy_working_tree_behind_its_upstream() {
    let status = fixture("busy.bin");

    assert_eq!((status.ahead, status.behind), (0, 2));
    // b.txt edited; both.txt added then edited; staged.txt added; untracked.txt new.
    assert_eq!(status.staged, 2);
    assert_eq!(status.modified, 2);
    assert_eq!(status.untracked, 1);
    assert_eq!(status.conflicted, 0);
    assert!(status.is_dirty());
}

#[test]
fn a_rename_is_one_staged_file_and_its_old_path_is_not_a_file() {
    // Taken with renames detected, so it has the record with two paths.
    let status = fixture("renamed.bin");

    assert_eq!(status.staged, 3);
    assert_eq!(status.modified, 2);
    assert_eq!(status.untracked, 1);
}

#[test]
fn a_merge_conflict() {
    let status = fixture("conflict.bin");

    assert_eq!(status.conflicted, 1);
    assert_eq!((status.ahead, status.behind), (1, 2));
    assert!(status.is_dirty());
}

#[test]
fn a_detached_head() {
    let status = fixture("detached.bin");

    assert!(status.detached);
    assert_eq!(status.branch, None);
    assert_eq!(status.commit.as_deref(), Some("6004959"));
    assert!(!status.has_upstream);
}

#[test]
fn a_repository_with_no_commit_yet() {
    let status = fixture("unborn.bin");

    assert_eq!(status.branch.as_deref(), Some("main"));
    assert_eq!(status.commit, None);
    assert_eq!(status.untracked, 1);
}

// --- the runner, with a real git -----------------------------------------------------------------

static COUNTER: AtomicUsize = AtomicUsize::new(0);

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("termzip-git-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn git_is_installed() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}

/// Runs git in `dir` the way a test needs it: no global config, no signing, a fixed identity.
fn git(dir: &Path, args: &[&str]) -> bool {
    Command::new("git")
        .current_dir(dir)
        .args([
            "-c",
            "user.name=T",
            "-c",
            "user.email=t@example.com",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .is_ok_and(|output| output.status.success())
}

#[test]
fn the_state_of_a_real_repository_is_read() {
    if !git_is_installed() {
        return;
    }
    let scratch = Scratch::new();
    let dir = &scratch.0;
    assert!(git(dir, &["init", "-q", "-b", "main"]));
    fs::write(dir.join("a.txt"), "a\n").unwrap();
    assert!(git(dir, &["add", "a.txt"]));
    assert!(git(dir, &["commit", "-q", "-m", "one"]));

    let clean = status(dir).expect("a repository has a status");
    assert_eq!(clean.branch.as_deref(), Some("main"));
    assert!(!clean.is_dirty());

    fs::write(dir.join("a.txt"), "changed\n").unwrap();
    fs::write(dir.join("new.txt"), "new\n").unwrap();
    let dirty = status(dir).unwrap();
    assert_eq!((dirty.modified, dirty.untracked, dirty.staged), (1, 1, 0));
    // A folder inside the repository is in the same repository.
    fs::create_dir(dir.join("sub")).unwrap();
    assert_eq!(status(&dir.join("sub")).unwrap().modified, 1);
}

#[test]
fn a_folder_that_is_no_repository_has_no_status() {
    if !git_is_installed() {
        return;
    }
    let scratch = Scratch::new();

    assert_eq!(status(&scratch.0), None);
    assert_eq!(status(Path::new("/definitely/not/here")), None);
}

#[cfg(unix)]
#[test]
fn a_program_named_by_the_repository_is_not_started() {
    use std::os::unix::fs::PermissionsExt;

    if !git_is_installed() {
        return;
    }
    let scratch = Scratch::new();
    let dir = &scratch.0;
    assert!(git(dir, &["init", "-q", "-b", "main"]));
    fs::write(dir.join("a.txt"), "a\n").unwrap();
    assert!(git(dir, &["add", "a.txt"]));
    assert!(git(dir, &["commit", "-q", "-m", "one"]));
    // A repository can say, in its own config, to run a program on every `git status`.
    let marker = dir.join("ran");
    let hook = dir.join("hook.sh");
    fs::write(
        &hook,
        format!("#!/bin/sh\ntouch '{}'\nprintf '\\0'\n", marker.display()),
    )
    .unwrap();
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(git(
        dir,
        &["config", "core.fsmonitor", hook.to_str().unwrap()]
    ));

    // Without the protection git does start it; this proves the check below means something.
    assert!(git(dir, &["status"]));
    let started_by_plain_git = marker.exists();
    let _ = fs::remove_file(&marker);

    let state = status(dir);

    assert!(state.is_some());
    assert!(
        !marker.exists(),
        "the program named by the repository was started"
    );
    if !started_by_plain_git {
        eprintln!("this git does not run core.fsmonitor hooks here; the check proved nothing");
    }
}
