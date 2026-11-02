//! The web page of a repository, from the address of its `origin` remote.
//!
//! A remote is written in several ways (`https://host/owner/repo.git`, `git@host:owner/repo.git`,
//! `ssh://git@host:2222/owner/repo.git`); the page of the repository is `https://host/owner/repo` for the code
//! hosts that people use. Only the host and the path are kept: a user name, a password or a token written in the
//! address is dropped, never opened and never shown, and the SSH port is not an HTTPS port.
//!
//! The address comes from a repository that may not be trusted, so anything that is not a plain host and a plain
//! path is refused rather than repaired.

const MAX_REMOTE_BYTES: usize = 2048;

fn is_host(host: &str) -> bool {
    // One character is a drive letter (`C:\repo`), not a host.
    host.len() >= 2
        && host.len() <= 253
        && !host.starts_with(['-', '.'])
        && !host.ends_with('-')
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-'))
}

fn is_path_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment != "."
        && segment != ".."
        && segment
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '~' | '+' | '-'))
}

/// `host` or `host:port` with the port of a web address; a port that is not digits is refused.
fn web_authority(host_port: &str) -> Option<String> {
    match host_port.rsplit_once(':') {
        Some((host, port))
            if is_host(host) && !port.is_empty() && port.chars().all(|c| c.is_ascii_digit()) =>
        {
            Some(format!("{host}:{port}"))
        }
        Some(_) => None,
        None => is_host(host_port).then(|| host_port.to_owned()),
    }
}

/// The host of an address that is not a web one: a port in it (`:2222`) is the SSH port and is dropped.
fn bare_host(host_port: &str) -> Option<String> {
    let host = match host_port.rsplit_once(':') {
        Some((host, port)) if !port.is_empty() && port.chars().all(|c| c.is_ascii_digit()) => host,
        Some(_) => return None,
        None => host_port,
    };
    is_host(host).then(|| host.to_owned())
}

/// The part of an authority after the last `@`: what comes before is a user name or credentials.
fn without_credentials(authority: &str) -> &str {
    authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host)
}

/// `https://host/owner/repo` for the address of a remote, or `None` when it is no address of a code host (a path
/// on this computer, a `file://` address, anything with spaces, control characters or an odd host or path).
pub fn remote_to_https(remote: &str) -> Option<String> {
    let remote = remote.trim();
    if remote.is_empty()
        || remote.len() > MAX_REMOTE_BYTES
        || remote.chars().any(|c| c.is_control() || c.is_whitespace())
    {
        return None;
    }
    let (authority, path) = if let Some((scheme, rest)) = remote.split_once("://") {
        let (authority, path) = rest.split_once('/')?;
        let host_port = without_credentials(authority);
        let authority = match scheme.to_ascii_lowercase().as_str() {
            "https" | "http" => web_authority(host_port)?,
            "ssh" | "git" | "git+ssh" | "ssh+git" => bare_host(host_port)?,
            _ => return None,
        };
        (authority, path)
    } else {
        // The form `[user@]host:path` that git accepts for SSH.
        let (authority, path) = remote.split_once(':')?;
        (bare_host(without_credentials(authority))?, path)
    };
    let path = path.trim_end_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);
    let path = path.trim_start_matches('/');
    let segments: Vec<&str> = path.split('/').collect();
    if segments.len() < 2 || !segments.iter().all(|segment| is_path_segment(segment)) {
        return None;
    }
    Some(format!("https://{authority}/{path}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_usual_forms_of_a_github_remote_give_the_same_page() {
        for remote in [
            "https://github.com/LuisPeixoto/TermZip.git",
            "https://github.com/LuisPeixoto/TermZip",
            "https://github.com/LuisPeixoto/TermZip/",
            "http://github.com/LuisPeixoto/TermZip.git",
            "git@github.com:LuisPeixoto/TermZip.git",
            "git@github.com:LuisPeixoto/TermZip",
            "ssh://git@github.com/LuisPeixoto/TermZip.git",
            "git://github.com/LuisPeixoto/TermZip.git",
            "  https://github.com/LuisPeixoto/TermZip.git\n",
        ] {
            assert_eq!(
                remote_to_https(remote).as_deref(),
                Some("https://github.com/LuisPeixoto/TermZip"),
                "{remote:?}"
            );
        }
    }

    #[test]
    fn credentials_in_the_address_are_dropped() {
        for remote in [
            "https://ghp_secrettoken@github.com/o/r.git",
            "https://user:password@github.com/o/r.git",
            "ssh://deploy@github.com/o/r.git",
        ] {
            let page = remote_to_https(remote).unwrap();
            assert_eq!(page, "https://github.com/o/r", "{remote}");
            assert!(!page.contains("secret") && !page.contains("password") && !page.contains('@'));
        }
    }

    #[test]
    fn the_ssh_port_is_not_a_web_port_but_a_web_port_is_kept() {
        assert_eq!(
            remote_to_https("ssh://git@git.example.com:2222/team/app.git").as_deref(),
            Some("https://git.example.com/team/app")
        );
        assert_eq!(
            remote_to_https("https://git.example.com:8443/team/app.git").as_deref(),
            Some("https://git.example.com:8443/team/app")
        );
    }

    #[test]
    fn nested_groups_and_a_leading_slash_in_the_scp_form_work() {
        assert_eq!(
            remote_to_https("git@gitlab.com:group/sub/project.git").as_deref(),
            Some("https://gitlab.com/group/sub/project")
        );
        assert_eq!(
            remote_to_https("git@host.example:/srv/team/app.git").as_deref(),
            Some("https://host.example/srv/team/app")
        );
        assert_eq!(
            remote_to_https("https://git.sr.ht/~someone/project").as_deref(),
            Some("https://git.sr.ht/~someone/project")
        );
    }

    #[test]
    fn a_path_on_this_computer_is_not_a_page() {
        for local in [
            "/srv/repos/app.git",
            "../app.git",
            "./app",
            "file:///srv/app.git",
            "C:\\repos\\app.git",
            "C:/repos/app.git",
            "app.git",
            "",
            "   ",
        ] {
            assert_eq!(remote_to_https(local), None, "{local:?}");
        }
    }

    #[test]
    fn schemes_that_are_not_a_code_host_are_refused() {
        for remote in [
            "ftp://host.example/o/r.git",
            "javascript://host.example/o/r",
            "data://host.example/o/r",
            "ext::sh -c evil",
        ] {
            assert_eq!(remote_to_https(remote), None, "{remote}");
        }
    }

    #[test]
    fn an_odd_host_or_path_is_refused_not_repaired() {
        for remote in [
            "https://exa mple.com/o/r",
            "https://-oops.example/o/r",
            "https://host.example/../etc/passwd",
            "https://host.example/o/../r",
            "https://host.example/o//r",
            "https://host.example/o",
            "https://host.example/",
            "https://host.example",
            "https://host.example/o/r?x=1",
            "https://host.example/o/r#frag",
            "https://host.example/o/r%20x",
            "https://host.example/o/r\u{1b}[31m",
            "https://exämple.com/o/r",
            "https://[::1]/o/r",
            "https://host.example:port/o/r",
            "git@host.example:o/r;rm",
            "git@host.example:o/$(x)",
        ] {
            assert_eq!(remote_to_https(remote), None, "{remote:?}");
        }
    }

    #[test]
    fn an_address_that_looks_like_a_host_trick_gives_the_host_it_really_has() {
        // Everything before the last `@` is credentials, so this is a page of evil.example, shown as such.
        assert_eq!(
            remote_to_https("https://github.com@evil.example/o/r.git").as_deref(),
            Some("https://evil.example/o/r")
        );
    }

    #[test]
    fn a_huge_address_is_refused() {
        let huge = format!("https://host.example/{}", "a/".repeat(2000));
        assert_eq!(remote_to_https(&huge), None);
    }
}
