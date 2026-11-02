//! What Preferences offers when a remote profile is made: the hosts of `~/.ssh/config` and the installed WSL
//! distributions.

use std::path::Path;

/// The host names of the SSH config in `ssh_dir`; none when there is no such folder.
fn hosts_in(ssh_dir: Option<&Path>) -> Vec<String> {
    ssh_dir.map(termzip_core::ssh_hosts).unwrap_or_default()
}

/// The `Host` aliases of `~/.ssh/config` (and the files it includes), for the host field of an SSH profile. Only
/// names are read; keys, users and proxy commands are not looked at.
///
/// The files are read on a blocking thread: a home folder can be on a slow or network disk.
#[tauri::command]
pub async fn list_ssh_hosts() -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(|| hosts_in(termzip_core::default_ssh_dir().as_deref()))
        .await
        .map_err(|error| format!("Could not read the SSH config: {error}"))
}

/// What `wsl.exe -l -q` prints, for the part of the answer that can be trusted: nothing unless the command
/// succeeded within a few seconds. WSL exists on Windows only.
#[cfg(windows)]
fn run_wsl_list() -> Vec<u8> {
    use std::io::Read;
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    /// Do not flash a console window.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const TIMEOUT: Duration = Duration::from_secs(3);

    let Ok(mut child) = Command::new("wsl.exe")
        .args(["-l", "-q"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
    else {
        return Vec::new();
    };
    let Some(stdout) = child.stdout.take() else {
        let _ = child.kill();
        return Vec::new();
    };
    let reader = std::thread::spawn(move || {
        let mut output = Vec::new();
        let _ = stdout
            .take(termzip_core::MAX_WSL_OUTPUT_BYTES as u64)
            .read_to_end(&mut output);
        output
    });
    let deadline = Instant::now() + TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let output = reader.join().unwrap_or_default();
                return if status.success() { output } else { Vec::new() };
            }
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(50)),
            // Too slow, or it cannot be waited for: stop it, so no wsl.exe is left running.
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = reader.join();
                return Vec::new();
            }
        }
    }
}

#[cfg(not(windows))]
fn run_wsl_list() -> Vec<u8> {
    Vec::new()
}

/// The installed WSL distributions, for the distribution field of a WSL profile; none off Windows, or when WSL
/// is not set up. Run on a blocking thread, with a timeout, because `wsl.exe` can take a moment to wake up.
#[tauri::command]
pub async fn list_wsl_distributions() -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(|| termzip_core::parse_distributions(&run_wsl_list()))
        .await
        .map_err(|error| format!("Could not list the WSL distributions: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_ssh_folder_means_no_hosts() {
        assert!(hosts_in(None).is_empty());
        assert!(hosts_in(Some(Path::new("/definitely/not/here"))).is_empty());
    }

    #[test]
    fn the_hosts_of_a_folder_are_listed() {
        let dir = std::env::temp_dir().join(format!("termzip-remote-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("config"),
            "Host build-box\n  User deploy\nHost *\n",
        )
        .unwrap();

        let hosts = hosts_in(Some(&dir));
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(hosts, ["build-box"]);
    }

    #[cfg(not(windows))]
    #[test]
    fn wsl_has_no_distributions_off_windows() {
        let list = tauri::async_runtime::block_on(list_wsl_distributions()).unwrap();
        assert!(list.is_empty());
    }
}
