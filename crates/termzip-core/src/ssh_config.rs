//! The host names in `~/.ssh/config`, to offer them when an SSH profile is made.
//!
//! Only the `Host` aliases are read, and only the ones that are plain names: a pattern with `*`, `?` or `!` stands
//! for many hosts and is no name to connect to. Nothing else in the file is looked at, so nothing like a key path,
//! a proxy command or a user is read, kept or shown. `Include` is followed a little (two levels, inside the `.ssh`
//! folder only), and the file is untrusted input: bounded in size, in files and in names, and never a panic.

use std::fs::{self, File};
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use crate::paths::expand_home;
use crate::profile::is_valid_host;

/// The biggest config file that is read.
pub const MAX_CONFIG_BYTES: u64 = 256 * 1024;
/// The most files read (the config and what it includes).
pub const MAX_CONFIG_FILES: usize = 32;
/// How many `Include` levels are followed.
pub const MAX_INCLUDE_DEPTH: usize = 2;
/// The most host names returned.
pub const MAX_HOSTS: usize = 200;

/// `~/.ssh`, where the config of the person is.
pub fn default_ssh_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".ssh"))
}

/// What a line of the config says: its keyword in lower case and its arguments, with the quotes removed.
fn split_line(line: &str) -> Option<(String, Vec<String>)> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    // `Keyword value` or `Keyword=value`.
    let end = line.find(|c: char| c.is_whitespace() || c == '=')?;
    let keyword = line[..end].to_ascii_lowercase();
    let rest = line[end..].trim_start_matches(|c: char| c.is_whitespace() || c == '=');
    let mut arguments = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut started = false;
    for c in rest.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                started = true;
            }
            '#' if !quoted && !started => break,
            c if c.is_whitespace() && !quoted => {
                if started {
                    arguments.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            c => {
                current.push(c);
                started = true;
            }
        }
    }
    if started {
        arguments.push(current);
    }
    Some((keyword, arguments))
}

/// What one file says: the host names, and the `Include` arguments to follow.
fn parse(text: &str) -> (Vec<String>, Vec<String>) {
    let mut hosts = Vec::new();
    let mut includes = Vec::new();
    for line in text.lines() {
        let Some((keyword, arguments)) = split_line(line) else {
            continue;
        };
        match keyword.as_str() {
            "host" => {
                for pattern in arguments {
                    let wide = pattern.contains(['*', '?', '!']);
                    if !wide && is_valid_host(&pattern) && !hosts.contains(&pattern) {
                        hosts.push(pattern);
                    }
                }
            }
            "include" => includes.extend(arguments),
            _ => {}
        }
    }
    (hosts, includes)
}

/// The host names in the text of a config file, without following any `Include`.
pub fn parse_hosts(text: &str) -> Vec<String> {
    parse(text).0
}

/// Whether `name` matches `pattern`, where `*` is any run of characters and `?` is one.
fn glob_matches(pattern: &str, name: &str) -> bool {
    let (pattern, name): (Vec<char>, Vec<char>) =
        (pattern.chars().collect(), name.chars().collect());
    // Iterative matching with a single backtrack point, so a hostile pattern cannot take exponential time.
    let (mut p, mut n) = (0, 0);
    let (mut star, mut mark) = (None, 0);
    while n < name.len() {
        if p < pattern.len() && (pattern[p] == '?' || pattern[p] == name[n]) {
            p += 1;
            n += 1;
        } else if p < pattern.len() && pattern[p] == '*' {
            star = Some(p);
            mark = n;
            p += 1;
        } else if let Some(s) = star {
            p = s + 1;
            mark += 1;
            n = mark;
        } else {
            return false;
        }
    }
    pattern[p..].iter().all(|c| *c == '*')
}

/// A path with no `..` that sits inside `root`. The comparison is on the written path; nothing is resolved.
fn is_inside(root: &Path, path: &Path) -> bool {
    path.starts_with(root) && !path.components().any(|c| c == Component::ParentDir)
}

/// The files an `Include` argument names: a path relative to `ssh_dir` (or starting with `~`), whose last part may
/// have `*` and `?`. Only a file inside `ssh_dir` is returned.
fn included_files(ssh_dir: &Path, argument: &str) -> Vec<PathBuf> {
    let path = if argument.starts_with('~') {
        expand_home(argument)
    } else {
        let given = Path::new(argument);
        if given.is_absolute() {
            given.to_path_buf()
        } else {
            ssh_dir.join(given)
        }
    };
    if !is_inside(ssh_dir, &path) {
        return Vec::new();
    }
    let (Some(folder), Some(name)) = (path.parent(), path.file_name().and_then(|n| n.to_str()))
    else {
        return Vec::new();
    };
    if !name.contains(['*', '?']) {
        return vec![path];
    }
    let Ok(entries) = fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .is_some_and(|n| glob_matches(name, n))
        })
        .map(|entry| entry.path())
        .collect();
    files.sort();
    files
}

/// The text of a regular file of a bounded size; a pipe or a device is never opened.
fn read_config(path: &Path) -> Option<String> {
    let metadata = fs::metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_CONFIG_BYTES {
        return None;
    }
    let mut bytes = Vec::new();
    File::open(path)
        .ok()?
        .take(MAX_CONFIG_BYTES + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    (bytes.len() as u64 <= MAX_CONFIG_BYTES).then(|| String::from_utf8_lossy(&bytes).into_owned())
}

fn collect(
    ssh_dir: &Path,
    file: &Path,
    depth: usize,
    files_read: &mut usize,
    hosts: &mut Vec<String>,
) {
    if *files_read >= MAX_CONFIG_FILES || hosts.len() >= MAX_HOSTS {
        return;
    }
    let Some(text) = read_config(file) else {
        return;
    };
    *files_read += 1;
    let (found, includes) = parse(&text);
    for host in found {
        if hosts.len() < MAX_HOSTS && !hosts.contains(&host) {
            hosts.push(host);
        }
    }
    if depth >= MAX_INCLUDE_DEPTH {
        return;
    }
    for argument in includes {
        for included in included_files(ssh_dir, &argument) {
            collect(ssh_dir, &included, depth + 1, files_read, hosts);
        }
    }
}

/// The host names of `<ssh_dir>/config`, in the order they are written, followed by the ones of the files it
/// includes. A missing or unreadable config gives none.
pub fn ssh_hosts(ssh_dir: &Path) -> Vec<String> {
    let mut hosts = Vec::new();
    let mut files_read = 0;
    collect(
        ssh_dir,
        &ssh_dir.join("config"),
        0,
        &mut files_read,
        &mut hosts,
    );
    hosts
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    struct Scratch(PathBuf);

    impl Scratch {
        fn new() -> Self {
            let n = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!("termzip-ssh-{}-{n}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn write(&self, relative: &str, text: &str) {
            let path = self.0.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// A config as people write them: comments, mixed case, `=`, several names on a line, and the secrets that
    /// must never be read.
    const REAL: &str = "\
# my machines
Host build-box
    HostName 10.0.0.7
    User deploy
    IdentityFile ~/.ssh/id_ed25519
    ProxyCommand ssh -W %h:%p bastion

Host=staging  prod.example.com   # trailing comment
  Port 2222

host Lower
  User x

Host *
  ServerAliveInterval 30

Host *.internal !secret db?
  User y

Host \"quoted-alias\"
Match host foo exec \"true\"
  User z
";

    #[test]
    fn the_aliases_are_read_and_patterns_are_not() {
        assert_eq!(
            parse_hosts(REAL),
            [
                "build-box",
                "staging",
                "prod.example.com",
                "Lower",
                "quoted-alias"
            ]
        );
    }

    #[test]
    fn nothing_but_host_names_comes_out() {
        let all = parse_hosts(REAL).join(" ");
        for secret in [
            "10.0.0.7",
            "deploy",
            "id_ed25519",
            "ProxyCommand",
            "bastion",
            "2222",
        ] {
            assert!(!all.contains(secret), "{secret}");
        }
    }

    #[test]
    fn a_name_ssh_could_read_as_an_option_or_a_shell_word_is_not_offered() {
        let text = "Host -oProxyCommand=x ok a;b a$b `c` d/e\n";
        assert_eq!(parse_hosts(text), ["ok"]);
    }

    #[test]
    fn a_name_is_listed_once() {
        assert_eq!(parse_hosts("Host a b a\nHost b c"), ["a", "b", "c"]);
    }

    #[test]
    fn hostile_text_never_panics() {
        for text in [
            "".to_owned(),
            "Host".to_owned(),
            "Host \"".to_owned(),
            "=".to_owned(),
            "Host =".to_owned(),
            "Include".to_owned(),
            "\0\0".to_owned(),
            "Host ".to_owned() + &"a ".repeat(100_000),
            "Host \"".to_owned() + &"b".repeat(100_000),
            "#".repeat(100_000),
            "Host é\u{202e}x\n".repeat(1000),
        ] {
            let _ = parse_hosts(&text);
        }
    }

    #[test]
    fn a_glob_matches_what_it_says() {
        assert!(glob_matches("*", "anything"));
        assert!(glob_matches("*.conf", "work.conf"));
        assert!(!glob_matches("*.conf", "work.conf.bak"));
        assert!(glob_matches("a?c", "abc"));
        assert!(!glob_matches("a?c", "ac"));
        assert!(glob_matches("a*b*c", "aXXbYYc"));
        assert!(glob_matches("", ""));
        assert!(!glob_matches("", "x"));
        // No catastrophic backtracking on a pattern built to cause it.
        let nasty = "a*".repeat(2000);
        assert!(glob_matches(&nasty, &"a".repeat(3000)));
        assert!(!glob_matches(&format!("{nasty}b"), &"a".repeat(3000)));
    }

    #[test]
    fn a_missing_config_has_no_hosts() {
        let scratch = Scratch::new();
        assert!(ssh_hosts(&scratch.0).is_empty());
        assert!(ssh_hosts(Path::new("/definitely/not/here")).is_empty());
    }

    #[test]
    fn included_files_are_followed_inside_the_ssh_folder() {
        let scratch = Scratch::new();
        scratch.write(
            "config",
            "Include config.d/*.conf\nInclude extra\nHost main\n",
        );
        scratch.write("config.d/work.conf", "Host work-1 work-2\n");
        scratch.write("config.d/notes.txt", "Host ignored\n");
        scratch.write("extra", "Host extra-box\n");

        // The names of a file come first, then those of the files it includes, in the order they are written.
        assert_eq!(
            ssh_hosts(&scratch.0),
            ["main", "work-1", "work-2", "extra-box"]
        );
    }

    #[test]
    fn an_include_outside_the_ssh_folder_or_with_dots_is_not_followed() {
        let scratch = Scratch::new();
        scratch.write(
            "ssh/config",
            "Include ../secret\nInclude /etc/ssh/ssh_config\nHost mine\n",
        );
        scratch.write("secret", "Host stolen\n");

        assert_eq!(ssh_hosts(&scratch.0.join("ssh")), ["mine"]);
    }

    #[test]
    fn includes_are_followed_two_levels_and_a_loop_ends() {
        let scratch = Scratch::new();
        scratch.write("config", "Host a\nInclude one\n");
        scratch.write("one", "Host b\nInclude two\n");
        scratch.write("two", "Host c\nInclude three\n");
        scratch.write("three", "Host d\n");
        scratch.write("loop", "Include config\n");

        assert_eq!(ssh_hosts(&scratch.0), ["a", "b", "c"]);

        scratch.write("config", "Host a\nInclude loop\n");
        let hosts = ssh_hosts(&scratch.0);
        assert_eq!(hosts, ["a"]);
    }

    #[test]
    fn a_config_that_is_too_big_is_not_read() {
        let scratch = Scratch::new();
        scratch.write(
            "config",
            &format!("Host big\n{}", "#".repeat(MAX_CONFIG_BYTES as usize)),
        );

        assert!(ssh_hosts(&scratch.0).is_empty());
    }

    #[test]
    fn there_is_a_limit_to_the_names_and_to_the_files() {
        let scratch = Scratch::new();
        let many: Vec<String> = (0..500).map(|n| format!("h{n}")).collect();
        scratch.write("config", &format!("Host {}\n", many.join(" ")));
        assert_eq!(ssh_hosts(&scratch.0).len(), MAX_HOSTS);

        let includes: String = (0..100).map(|n| format!("Include f{n}\n")).collect();
        scratch.write("config", &includes);
        for n in 0..100 {
            scratch.write(&format!("f{n}"), &format!("Host file-{n}\n"));
        }
        assert_eq!(ssh_hosts(&scratch.0).len(), MAX_CONFIG_FILES - 1);
    }

    #[cfg(unix)]
    #[test]
    fn a_named_pipe_is_never_opened() {
        let scratch = Scratch::new();
        let status = std::process::Command::new("mkfifo")
            .arg(scratch.0.join("config"))
            .status();
        if !matches!(status, Ok(status) if status.success()) {
            return;
        }

        assert!(ssh_hosts(&scratch.0).is_empty());
    }
}
