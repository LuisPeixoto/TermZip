//! What Preferences offers when a remote profile is made: the hosts of `~/.ssh/config`.

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
}
