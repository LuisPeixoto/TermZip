//! The git state of the folder of a terminal, for the status bar.

use std::path::Path;

use termzip_git::GitStatus;

/// The state of the repository `cwd` is in. Nothing is run when the setting is off or the folder is not known,
/// so with the setting off git is never started.
fn status_if(enabled: bool, cwd: Option<&str>) -> Option<GitStatus> {
    let cwd = cwd.map(str::trim).filter(|cwd| !cwd.is_empty())?;
    enabled
        .then(|| termzip_git::status(Path::new(cwd)))
        .flatten()
}

/// The web page of the repository `cwd` is in, from its `origin` remote. Nothing is run when the setting is off.
fn page_if(enabled: bool, cwd: Option<&str>) -> Option<String> {
    let cwd = cwd.map(str::trim).filter(|cwd| !cwd.is_empty())?;
    enabled
        .then(|| termzip_git::repository_page(Path::new(cwd)))
        .flatten()
}

/// `None` when the folder is in no repository, git is not installed or too slow, or the status bar item is turned
/// off in Preferences. The setting is read here, from disk, so it holds even for a stale interface.
///
/// Git runs on a blocking thread: a big repository can take its time, and the interface must not wait for it.
#[tauri::command]
pub async fn git_status(cwd: Option<String>) -> Result<Option<GitStatus>, String> {
    let enabled = termzip_core::Settings::load().is_ok_and(|settings| settings.git_status);
    tauri::async_runtime::spawn_blocking(move || status_if(enabled, cwd.as_deref()))
        .await
        .map_err(|error| format!("Could not read the git state: {error}"))
}

/// `https://host/owner/repo` for the palette's *Open the repository*; `None` when there is no repository, no
/// `origin`, it is not the address of a code host, or git is turned off in Preferences. Only the host and the path
/// of the address are kept, never a user name, password or token.
#[tauri::command]
pub async fn repository_page(cwd: Option<String>) -> Result<Option<String>, String> {
    let enabled = termzip_core::Settings::load().is_ok_and(|settings| settings.git_status);
    tauri::async_runtime::spawn_blocking(move || page_if(enabled, cwd.as_deref()))
        .await
        .map_err(|error| format!("Could not read the repository address: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_the_setting_off_git_is_never_started() {
        // This repository is one, so only the setting can explain the answer.
        let here = Path::new(env!("CARGO_MANIFEST_DIR"));

        assert_eq!(status_if(false, here.to_str()), None);
    }

    #[test]
    fn an_unknown_folder_reads_nothing() {
        assert_eq!(status_if(true, None), None);
        assert_eq!(status_if(true, Some("")), None);
        assert_eq!(status_if(true, Some("   ")), None);
        assert_eq!(status_if(true, Some("/definitely/not/here")), None);
    }

    #[test]
    fn with_the_setting_off_the_repository_address_is_not_read() {
        let here = Path::new(env!("CARGO_MANIFEST_DIR"));

        assert_eq!(page_if(false, here.to_str()), None);
        assert_eq!(page_if(true, None), None);
        assert_eq!(page_if(true, Some("  ")), None);
    }
}
