//! The git state of a folder, read locally for the status bar: the branch, how far it is from its upstream and how
//! many files changed. Nothing here uses the network: `git status` only looks at the repository on disk.
//!
//! The repository may belong to a project you just cloned, so git is run defensively:
//! - `core.fsmonitor` is turned off for the run. A repository's own config can name a program for it, and `git
//!   status` would start it.
//! - `GIT_OPTIONAL_LOCKS=0` keeps `git status` from taking the index lock, so a refresh in the background can never
//!   make your own `git commit` fail.
//! - `GIT_TERMINAL_PROMPT=0` makes sure nothing waits for a password, and the run stops after [`TIMEOUT`].
//! - Git's own protection against a repository owned by someone else (`safe.directory`) is left as it is.

mod parse;
mod remote;

use std::io::{self, Read};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

pub use parse::{GitStatus, MAX_OUTPUT_BYTES, parse_status};
pub use remote::remote_to_https;

/// How long git may take before it is stopped; a huge repository on a slow disk gives no status instead of a wait.
pub const TIMEOUT: Duration = Duration::from_secs(2);

/// What git printed to its standard output, when it ran in `dir` and exited with success within [`TIMEOUT`].
fn run_git(dir: &Path, args: &[&str]) -> Option<Vec<u8>> {
    let mut command = Command::new("git");
    command
        .current_dir(dir)
        .args(args)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Do not flash a console window.
        command.creation_flags(0x0800_0000);
    }
    let mut child = command.spawn().ok()?;
    let mut stdout = child.stdout.take()?;
    // Read what is wanted and drain the rest, so a long output never blocks git on a full pipe.
    let reader = thread::spawn(move || {
        let mut output = Vec::new();
        let _ = (&mut stdout)
            .take(MAX_OUTPUT_BYTES as u64 + 1)
            .read_to_end(&mut output);
        let _ = io::copy(&mut stdout, &mut io::sink());
        output
    });
    let deadline = Instant::now() + TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let output = reader.join().unwrap_or_default();
                return status.success().then_some(output);
            }
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(10)),
            // Too slow, or it cannot be waited for: stop it so no git is left running.
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = reader.join();
                return None;
            }
        }
    }
}

/// The state of the repository `dir` is in, or `None` when it is in none, git is not installed, or git took too
/// long. Renames are not looked for (`--no-renames`), which is also what keeps this quick in a big repository.
pub fn status(dir: &Path) -> Option<GitStatus> {
    let output = run_git(
        dir,
        &[
            "-c",
            "core.fsmonitor=false",
            "status",
            "--porcelain=v2",
            "--branch",
            "-z",
            "--no-renames",
        ],
    )?;
    parse_status(&output)
}

/// The address of the `origin` remote of the repository `dir` is in. Reading the config starts no program and
/// uses no network.
pub fn origin_url(dir: &Path) -> Option<String> {
    let output = run_git(dir, &["config", "--get", "remote.origin.url"])?;
    let url = String::from_utf8(output).ok()?;
    let url = url.trim();
    (!url.is_empty()).then(|| url.to_owned())
}

/// The web page of the repository `dir` is in, from its `origin` remote: `https://host/owner/repo`. `None` when
/// there is no repository, no `origin`, or it is not the address of a code host.
pub fn repository_page(dir: &Path) -> Option<String> {
    remote_to_https(&origin_url(dir)?)
}
