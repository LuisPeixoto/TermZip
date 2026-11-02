//! The tasks of the project a terminal is in, for the command palette.

use std::path::Path;

use termzip_tasks::Task;

/// What the project of `cwd` can do, as commands to type. Nothing is read when the folder is not known.
///
/// The files are read on a blocking thread: a project can sit on a slow or network disk, and the interface must
/// not wait for it.
#[tauri::command]
pub async fn list_tasks(cwd: Option<String>) -> Result<Vec<Task>, String> {
    let Some(cwd) = cwd.filter(|cwd| !cwd.trim().is_empty()) else {
        return Ok(Vec::new());
    };
    tauri::async_runtime::spawn_blocking(move || termzip_tasks::detect(Path::new(&cwd)))
        .await
        .map_err(|error| format!("Could not look for the tasks of this folder: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unknown_folder_has_no_tasks_and_reads_nothing() {
        let tasks = tauri::async_runtime::block_on(list_tasks(None)).unwrap();
        assert!(tasks.is_empty());
        let blank = tauri::async_runtime::block_on(list_tasks(Some("  ".to_owned()))).unwrap();
        assert!(blank.is_empty());
    }

    #[test]
    fn the_tasks_of_this_repository_are_found_from_a_subfolder() {
        let here = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let tasks =
            tauri::async_runtime::block_on(list_tasks(Some(here.to_string_lossy().into_owned())))
                .unwrap();
        // apps/desktop/package.json is the nearest package.json, and the workspace has a Cargo.toml.
        assert!(
            tasks.iter().any(|task| task.command.ends_with("run tauri")),
            "{tasks:?}"
        );
    }
}
