//! Detection of the user's shell and the launch configuration used to start it
//! with TermZip's integration preloaded.

use std::path::{Path, PathBuf};

use crate::scripts::ensure_script;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellKind {
    Zsh,
    Bash,
    Fish,
    PowerShell,
    Cmd,
    Other,
}

impl ShellKind {
    pub fn integrates(self) -> bool {
        !matches!(self, Self::Cmd | Self::Other)
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Zsh => "zsh",
            Self::Bash => "bash",
            Self::Fish => "fish",
            Self::PowerShell => "PowerShell",
            Self::Cmd => "cmd",
            Self::Other => "shell",
        }
    }
}

pub fn detect_shell_kind(program: &Path) -> ShellKind {
    let name = program
        .file_name()
        .map(|name| name.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    match name.as_str() {
        "zsh" => ShellKind::Zsh,
        "bash" => ShellKind::Bash,
        "fish" => ShellKind::Fish,
        "pwsh" | "powershell" | "powershell.exe" | "pwsh.exe" => ShellKind::PowerShell,
        "cmd" | "cmd.exe" => ShellKind::Cmd,
        _ => ShellKind::Other,
    }
}

/// Everything the session layer needs to spawn the shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launch {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub integrated: bool,
}

pub fn default_shell() -> PathBuf {
    #[cfg(windows)]
    {
        std::env::var_os("COMSPEC")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("cmd.exe"))
    }

    #[cfg(not(windows))]
    {
        std::env::var_os("SHELL")
            .map(PathBuf::from)
            .filter(|shell| shell.is_file())
            .unwrap_or_else(|| PathBuf::from("/bin/sh"))
    }
}

/// Build the spawn configuration for the configured shell, injecting the
/// bundled integration script when the shell supports it.
pub fn launch_config(shell: Option<&str>, integration_dir: &Path) -> Launch {
    let program = shell.map(PathBuf::from).unwrap_or_else(default_shell);
    let kind = detect_shell_kind(&program);

    let Some(script) = ensure_script(kind, integration_dir) else {
        return Launch {
            program,
            args: Vec::new(),
            env: Vec::new(),
            integrated: false,
        };
    };
    let script = script.display().to_string();

    let mut launch = Launch {
        program: program.clone(),
        args: Vec::new(),
        env: Vec::new(),
        integrated: true,
    };

    match kind {
        ShellKind::Zsh => {
            let staged = staged_dir(integration_dir, "zsh");
            if write_zsh_stubs(&staged, &script) {
                // Without a home the stubs fall back to $HOME themselves.
                let user_zdotdir = std::env::var_os("ZDOTDIR")
                    .map(PathBuf::from)
                    .unwrap_or_else(home_dir);
                if !user_zdotdir.as_os_str().is_empty() {
                    launch.env.push((
                        "TERMZIP_USER_ZDOTDIR".to_owned(),
                        user_zdotdir.display().to_string(),
                    ));
                }
                launch
                    .env
                    .push(("ZDOTDIR".to_owned(), staged.display().to_string()));
            } else {
                // Better a plain shell with the user's own config than a broken ZDOTDIR.
                launch.integrated = false;
            }
        }
        ShellKind::Bash => {
            let staged = staged_dir(integration_dir, "bash");
            let rc = staged.join("bashrc");
            let contents = format!(
                "[ -f \"$HOME/.bashrc\" ] && builtin source \"$HOME/.bashrc\"\nbuiltin source {}\n",
                quote_posix(&script)
            );
            if std::fs::create_dir_all(&staged).is_ok() && std::fs::write(&rc, contents).is_ok() {
                launch.args.push("--rcfile".to_owned());
                launch.args.push(rc.display().to_string());
            } else {
                launch.integrated = false;
            }
        }
        ShellKind::Fish => {
            launch.args.push("-C".to_owned());
            launch.args.push(format!("source {}", quote_fish(&script)));
        }
        ShellKind::PowerShell => {
            launch.args.push("-NoExit".to_owned());
            launch.args.push("-Command".to_owned());
            launch.args.push(format!(". {}", quote_powershell(&script)));
        }
        ShellKind::Cmd | ShellKind::Other => {
            launch.integrated = false;
        }
    }

    launch
}

fn staged_dir(integration_dir: &Path, shell: &str) -> PathBuf {
    integration_dir.join(shell)
}

fn home_dir() -> PathBuf {
    #[cfg(windows)]
    {
        std::env::var_os("USERPROFILE")
            .map(PathBuf::from)
            .unwrap_or_default()
    }

    #[cfg(not(windows))]
    {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_default()
    }
}

/// Writes the `.zshenv` / `.zshrc` stubs that hand control to the user's own files and then
/// load the integration. Returns `false` when they could not be written.
fn write_zsh_stubs(dir: &Path, integration: &str) -> bool {
    if std::fs::create_dir_all(dir).is_err() {
        return false;
    }
    let zshenv = "typeset -g TERMZIP_STAGED_ZDOTDIR=\"$ZDOTDIR\"\n\
export ZDOTDIR=\"${TERMZIP_USER_ZDOTDIR:-$HOME}\"\n\
[[ -f \"$ZDOTDIR/.zshenv\" ]] && builtin source \"$ZDOTDIR/.zshenv\"\n\
export ZDOTDIR=\"$TERMZIP_STAGED_ZDOTDIR\"\n";
    let zshrc = format!(
        "export ZDOTDIR=\"${{TERMZIP_USER_ZDOTDIR:-$HOME}}\"\n\
         [[ -f \"$ZDOTDIR/.zshrc\" ]] && builtin source \"$ZDOTDIR/.zshrc\"\n\
         export ZDOTDIR=\"$TERMZIP_STAGED_ZDOTDIR\"\n\
         builtin source {}\n",
        quote_posix(integration)
    );
    std::fs::write(dir.join(".zshenv"), zshenv).is_ok()
        && std::fs::write(dir.join(".zshrc"), zshrc).is_ok()
}

/// Single-quotes a value for zsh and bash: nothing inside is expanded, and a
/// quote is written as close-quote, escaped quote, reopen.
fn quote_posix(value: &str) -> String {
    format!("'{}'", value.replace('\'', r"'\''"))
}

/// fish expands nothing inside single quotes either, but `\` and `\'` are escapes there.
fn quote_fish(value: &str) -> String {
    format!("'{}'", value.replace('\\', r"\\").replace('\'', r"\'"))
}

/// PowerShell doubles a quote inside a single-quoted string, and also treats the
/// typographic single quotes as quotes.
fn quote_powershell(value: &str) -> String {
    let mut quoted = String::with_capacity(value.len() + 2);
    quoted.push('\'');
    for character in value.chars() {
        quoted.push(character);
        if matches!(
            character,
            '\'' | '\u{2018}' | '\u{2019}' | '\u{201a}' | '\u{201b}'
        ) {
            quoted.push(character);
        }
    }
    quoted.push('\'');
    quoted
}

#[cfg(test)]
mod tests {
    use super::{ShellKind, detect_shell_kind, launch_config, quote_fish, quote_powershell};
    use std::path::{Path, PathBuf};

    fn kind_of(name: &str) -> ShellKind {
        detect_shell_kind(Path::new(name))
    }

    #[test]
    fn shells_are_detected_by_name() {
        assert_eq!(kind_of("/bin/zsh"), ShellKind::Zsh);
        assert_eq!(kind_of("/usr/bin/bash"), ShellKind::Bash);
        assert_eq!(kind_of("fish"), ShellKind::Fish);
        assert_eq!(kind_of("pwsh.exe"), ShellKind::PowerShell);
        assert_eq!(kind_of("cmd.exe"), ShellKind::Cmd);
        assert_eq!(kind_of("/bin/tcsh"), ShellKind::Other);
    }

    #[test]
    fn zsh_launch_stages_zdotdir() {
        let dir = std::env::temp_dir().join(format!("termzip-launch-{}", std::process::id()));
        let launch = launch_config(Some("/bin/zsh"), &dir);

        assert!(launch.integrated);
        assert!(launch.env.iter().any(|(key, _)| key == "ZDOTDIR"));
        assert!(
            launch
                .env
                .iter()
                .any(|(key, _)| key == "TERMZIP_USER_ZDOTDIR")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unknown_shells_run_without_integration() {
        let dir = std::env::temp_dir().join(format!("termzip-other-{}", std::process::id()));
        let launch = launch_config(Some("/bin/tcsh"), &dir);

        assert!(!launch.integrated);
        assert!(launch.args.is_empty());
        assert!(launch.env.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn bash_launch_uses_rcfile() {
        let dir = std::env::temp_dir().join(format!("termzip-bash-{}", std::process::id()));
        let launch = launch_config(Some("/bin/bash"), &dir);

        assert!(launch.integrated);
        assert!(launch.args.iter().any(|arg| arg == "--rcfile"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn default_shell_path_is_usable() {
        let path = PathBuf::from("placeholder");
        let _ = path;
        assert!(!super::default_shell().as_os_str().is_empty());
    }

    #[cfg(unix)]
    fn unique_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("termzip-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[cfg(unix)]
    #[test]
    fn a_hostile_integration_path_is_never_executed_by_bash() {
        if !Path::new("/bin/bash").exists() {
            return;
        }
        let root = unique_dir("hostile-bash");
        let marker = root.join("pwned");
        let integration = root.join(format!(
            "a'b$(touch {})`touch {}`",
            marker.display(),
            marker.display()
        ));

        let launch = launch_config(Some("/bin/bash"), &integration);
        assert!(launch.integrated);
        let status = std::process::Command::new("/bin/bash")
            .args(&launch.args)
            .args(["-i", "-c", "true"])
            .env("HOME", &root)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap();

        assert!(status.success());
        assert!(
            !marker.exists(),
            "the integration path was evaluated by the shell"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn zsh_without_a_staging_directory_is_not_reported_as_integrated() {
        let root = unique_dir("zsh-readonly");
        // A file where the staging directory must go makes the staging fail.
        std::fs::write(root.join("zsh"), "in the way").unwrap();

        let launch = launch_config(Some("/bin/zsh"), &root);

        assert!(!launch.integrated);
        assert!(launch.env.iter().all(|(key, _)| key != "ZDOTDIR"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn posix_quoting_round_trips_through_a_real_shell() {
        use super::quote_posix;

        const HOSTILE: &str = "/tmp/it's $HOME `id` $(id) \"x\" back\\slash";
        let output = std::process::Command::new("/bin/sh")
            .args(["-c", &format!("printf %s {}", quote_posix(HOSTILE))])
            .output()
            .unwrap();
        assert_eq!(String::from_utf8_lossy(&output.stdout), HOSTILE);
    }

    #[test]
    fn fish_quoting_escapes_quotes_and_backslashes() {
        assert_eq!(quote_fish("/a b"), "'/a b'");
        assert_eq!(quote_fish("it's"), r"'it\'s'");
        assert_eq!(quote_fish(r"C:\dir"), r"'C:\\dir'");
    }

    #[test]
    fn powershell_quoting_doubles_every_kind_of_single_quote() {
        assert_eq!(quote_powershell("/a b"), "'/a b'");
        assert_eq!(quote_powershell("it's"), "'it''s'");
        assert_eq!(quote_powershell("it\u{2019}s"), "'it\u{2019}\u{2019}s'");
    }
}
