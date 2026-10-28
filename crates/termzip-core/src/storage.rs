//! How TermZip writes the files it owns: privately, atomically, and without destroying what it
//! cannot read.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::SettingsError;

/// Creates the folder (private to the user on Unix), writes `contents` next to `path` and renames
/// it into place, so a crash mid-write never leaves a half-written file behind.
pub(crate) fn write_atomic(path: &Path, contents: &[u8]) -> Result<(), SettingsError> {
    let parent = path.parent().ok_or(SettingsError::InvalidPath)?;
    let write_error = |path: &Path| {
        let path = path.to_owned();
        move |source| SettingsError::Write { path, source }
    };
    fs::create_dir_all(parent).map_err(write_error(parent))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700))
            .map_err(write_error(parent))?;
    }

    let mut temporary = path.as_os_str().to_owned();
    temporary.push(".tmp");
    let temporary = PathBuf::from(temporary);

    let written = (|| {
        let mut file = fs::File::create(&temporary)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.set_permissions(fs::Permissions::from_mode(0o600))?;
        }
        file.write_all(contents)?;
        file.sync_all()?;
        fs::rename(&temporary, path)
    })();
    if let Err(source) = written {
        let _ = fs::remove_file(&temporary);
        return Err(SettingsError::Write {
            path: path.to_owned(),
            source,
        });
    }
    Ok(())
}

/// Keeps an unreadable file as `<name>.invalid` instead of letting the next save overwrite it.
pub(crate) fn set_aside(path: &Path) {
    let mut backup = path.as_os_str().to_owned();
    backup.push(".invalid");
    let _ = fs::rename(path, PathBuf::from(backup));
}
