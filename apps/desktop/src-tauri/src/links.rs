//! What a click on terminal text is allowed to open.
//!
//! Terminal output is untrusted: any program can print a link. So the target is checked here, in
//! Rust, before anything is handed to the operating system: only a few URL schemes, only files
//! that exist, and never a program (its folder is shown instead, the program is not run).

use std::path::{Path, PathBuf};

const MAX_TARGET_CHARS: usize = 2048;
const URL_SCHEMES: [&str; 3] = ["http", "https", "mailto"];

/// Extensions the system would run or install when asked to "open" them.
const PROGRAM_EXTENSIONS: [&str; 26] = [
    "app", "command", "workflow", "action", "terminal", "pkg", "dmg", "exe", "com", "scr", "msi",
    "bat", "cmd", "ps1", "vbs", "vbe", "js", "jse", "wsf", "jar", "lnk", "sh", "desktop",
    "appimage", "run", "scpt",
];

#[derive(Debug, PartialEq, Eq)]
pub enum LinkTarget {
    /// Open in the default browser or mail client.
    Url(String),
    /// Open in the default application.
    Open(PathBuf),
    /// Show the folder that holds it: the target is a program and must not be started by a click.
    Reveal(PathBuf),
}

/// Decides what `target` is and whether it may be opened. `cwd` is the folder of the pane the text
/// was printed in, `home` the user's home folder (for `~/`).
pub fn resolve(
    target: &str,
    cwd: Option<&Path>,
    home: Option<&Path>,
) -> Result<LinkTarget, String> {
    let target = target.trim();
    if target.is_empty() {
        return Err("There is nothing to open.".to_owned());
    }
    if target.chars().count() > MAX_TARGET_CHARS {
        return Err("That link is too long to open.".to_owned());
    }
    if target.chars().any(char::is_control) {
        return Err("That link contains control characters.".to_owned());
    }

    if let Some(scheme) = url_scheme(target) {
        return resolve_url(target, &scheme);
    }
    resolve_path(target, cwd, home)
}

/// The scheme when `target` is written as a URL (`scheme://…` or `mailto:…`).
fn url_scheme(target: &str) -> Option<String> {
    let colon = target.find(':')?;
    let scheme = &target[..colon];
    let valid = !scheme.is_empty()
        && scheme
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
    let rest = &target[colon + 1..];
    // `C:\dir` and `C:/dir` are paths, not a one-letter scheme.
    if scheme.len() == 1 || !valid {
        return None;
    }
    if rest.starts_with("//") || scheme.eq_ignore_ascii_case("mailto") {
        return Some(scheme.to_ascii_lowercase());
    }
    // `javascript:alert(1)`, `data:…`: a URL, just not one that may be opened.
    if matches!(
        scheme.to_ascii_lowercase().as_str(),
        "javascript" | "data" | "vbscript" | "file" | "blob" | "about"
    ) {
        return Some(scheme.to_ascii_lowercase());
    }
    None
}

fn resolve_url(target: &str, scheme: &str) -> Result<LinkTarget, String> {
    if !URL_SCHEMES.contains(&scheme) {
        return Err("Only http, https and mailto links can be opened.".to_owned());
    }
    if target.chars().any(char::is_whitespace) {
        return Err("That link contains spaces.".to_owned());
    }
    let rest = &target[scheme.len() + 1..];
    if scheme == "mailto" {
        if rest.is_empty() {
            return Err("That mail link has no address.".to_owned());
        }
    } else {
        // Exactly `//` introduces the host; `https:///path` has none.
        let host = rest
            .strip_prefix("//")
            .and_then(|authority| authority.split(['/', '?', '#']).next())
            .unwrap_or("");
        if host.is_empty() {
            return Err("That link has no host.".to_owned());
        }
    }
    Ok(LinkTarget::Url(target.to_owned()))
}

/// `src/main.rs:12:3` is `src/main.rs`: editors' `:line:column` suffixes are not part of the name.
fn strip_position(target: &str) -> &str {
    let mut text = target;
    for _ in 0..2 {
        match text.rsplit_once(':') {
            Some((head, tail))
                if !head.is_empty()
                    && !tail.is_empty()
                    && tail.chars().all(|c| c.is_ascii_digit())
                    // `C:12` is not a position; keep a drive letter intact.
                    && head.len() > 1 =>
            {
                text = head;
            }
            _ => break,
        }
    }
    text
}

fn resolve_path(
    target: &str,
    cwd: Option<&Path>,
    home: Option<&Path>,
) -> Result<LinkTarget, String> {
    let text = strip_position(target);
    let expanded: PathBuf =
        if let Some(rest) = text.strip_prefix("~/").or_else(|| text.strip_prefix("~\\")) {
            home.ok_or("The home folder is unknown.")?.join(rest)
        } else {
            PathBuf::from(text)
        };
    let absolute = if expanded.is_absolute() {
        expanded
    } else {
        cwd.filter(|dir| dir.is_absolute())
            .ok_or("This folder is unknown, so a relative path cannot be opened.")?
            .join(expanded)
    };
    let path = absolute
        .canonicalize()
        .map_err(|_| format!("“{text}” does not exist."))?;

    let extension = path
        .extension()
        .map(|ext| ext.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    if PROGRAM_EXTENSIONS.contains(&extension.as_str()) {
        return Ok(LinkTarget::Reveal(path));
    }
    Ok(LinkTarget::Open(path))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn sandbox(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("termzip-links-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        // Canonical, so the paths the tests build equal the ones `resolve` returns.
        dir.canonicalize().unwrap()
    }

    fn url(target: &str) -> Result<LinkTarget, String> {
        resolve(target, None, None)
    }

    #[test]
    fn web_and_mail_links_are_opened() {
        assert_eq!(
            url("https://example.com/a?b=1#c").unwrap(),
            LinkTarget::Url("https://example.com/a?b=1#c".to_owned())
        );
        assert!(matches!(url("HTTP://EXAMPLE.COM"), Ok(LinkTarget::Url(_))));
        assert!(matches!(
            url("http://localhost:3000"),
            Ok(LinkTarget::Url(_))
        ));
        assert!(matches!(
            url("mailto:someone@example.com"),
            Ok(LinkTarget::Url(_))
        ));
    }

    #[test]
    fn dangerous_schemes_are_refused() {
        for target in [
            "javascript:alert(1)",
            "JavaScript:alert(1)",
            "data:text/html;base64,PHNjcmlwdD4=",
            "vbscript:msgbox(1)",
            "file:///etc/passwd",
            "ftp://example.com/x",
            "ssh://host",
            "x-apple.systempreferences://",
            "blob:https://example.com/1",
        ] {
            let error = url(target).unwrap_err();
            assert!(error.contains("Only http"), "{target}: {error}");
        }
    }

    #[test]
    fn malformed_links_are_refused() {
        for target in [
            "",
            "   ",
            "https://",
            "https:///path",
            "mailto:",
            "http://exa mple.com",
        ] {
            assert!(url(target).is_err(), "{target:?} must be refused");
        }
        assert!(
            url("https://example.com/\u{1b}[31m")
                .unwrap_err()
                .contains("control")
        );
        assert!(
            url("https://example.com/\n").is_ok(),
            "trailing whitespace is trimmed"
        );
        assert!(
            url(&format!("https://example.com/{}", "a".repeat(3000)))
                .unwrap_err()
                .contains("too long")
        );
    }

    #[test]
    fn an_existing_file_is_opened_with_or_without_a_position() {
        let dir = sandbox("files");
        let file = dir.join("main.rs");
        fs::write(&file, "fn main() {}").unwrap();
        let expected = LinkTarget::Open(file.clone());

        assert_eq!(
            resolve(file.to_str().unwrap(), None, None).unwrap(),
            expected
        );
        assert_eq!(
            resolve(&format!("{}:12", file.display()), None, None).unwrap(),
            expected
        );
        assert_eq!(
            resolve(&format!("{}:12:3", file.display()), None, None).unwrap(),
            expected
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn relative_paths_use_the_folder_of_the_pane() {
        let dir = sandbox("relative");
        fs::create_dir_all(dir.join("src")).unwrap();
        let file = dir.join("src").join("lib.rs");
        fs::write(&file, "").unwrap();

        assert_eq!(
            resolve("src/lib.rs:4", Some(&dir), None).unwrap(),
            LinkTarget::Open(file.clone())
        );
        assert_eq!(
            resolve("./src/../src/lib.rs", Some(&dir), None).unwrap(),
            LinkTarget::Open(file)
        );
        assert!(
            resolve("src/lib.rs", None, None)
                .unwrap_err()
                .contains("folder is unknown")
        );
        assert!(resolve("src/lib.rs", Some(Path::new("relative")), None).is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_home_folder_is_expanded() {
        let home = sandbox("home");
        let file = home.join("notes.txt");
        fs::write(&file, "").unwrap();

        assert_eq!(
            resolve("~/notes.txt", None, Some(&home)).unwrap(),
            LinkTarget::Open(file)
        );
        assert!(resolve("~/notes.txt", None, None).is_err());
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn a_path_that_does_not_exist_is_refused() {
        let dir = sandbox("missing");
        let error = resolve("nope/missing.txt", Some(&dir), None).unwrap_err();
        assert!(error.contains("does not exist"), "{error}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn programs_are_revealed_never_started() {
        let dir = sandbox("programs");
        for name in [
            "run.command",
            "setup.EXE",
            "tool.sh",
            "go.bat",
            "x.ps1",
            "App.AppImage",
            "evil.desktop",
            "k.scpt",
        ] {
            let file = dir.join(name);
            fs::write(&file, "").unwrap();
            assert_eq!(
                resolve(file.to_str().unwrap(), None, None).unwrap(),
                LinkTarget::Reveal(file.canonicalize().unwrap()),
                "{name}"
            );
        }
        // A macOS application is a folder; it must not be launched either.
        let bundle = dir.join("Thing.app");
        fs::create_dir_all(&bundle).unwrap();
        assert!(matches!(
            resolve(bundle.to_str().unwrap(), None, None),
            Ok(LinkTarget::Reveal(_))
        ));
        // An ordinary folder is fine to open.
        let folder = dir.join("docs");
        fs::create_dir_all(&folder).unwrap();
        assert!(matches!(
            resolve(folder.to_str().unwrap(), None, None),
            Ok(LinkTarget::Open(_))
        ));
        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_to_a_program_is_still_treated_as_a_program() {
        let dir = sandbox("symlink");
        let program = dir.join("payload.command");
        fs::write(&program, "").unwrap();
        let link = dir.join("innocent.txt");
        std::os::unix::fs::symlink(&program, &link).unwrap();

        // The link is followed first, so the real extension decides.
        assert!(matches!(
            resolve(link.to_str().unwrap(), None, None),
            Ok(LinkTarget::Reveal(_))
        ));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_drive_letter_is_not_a_url_scheme() {
        assert_eq!(url_scheme("C:\\Users\\me\\file.txt"), None);
        assert_eq!(url_scheme("C:/Users/me"), None);
        assert_eq!(strip_position("C:\\dir\\file.rs:5"), "C:\\dir\\file.rs");
        assert_eq!(strip_position("C:12"), "C:12");
    }
}
