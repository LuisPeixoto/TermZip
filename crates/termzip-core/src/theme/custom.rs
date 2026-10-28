//! The themes a user added: one TOML file each in `<data folder>/themes/`.
//!
//! The folder is user-editable input like the config file, so nothing in it can break startup: files
//! that cannot be used are skipped and reported as sentences, and only regular files under the size
//! limit are ever read.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::{MAX_THEME_BYTES, Theme, built_in_themes};
use crate::profile::slug;
use crate::storage::write_atomic;
use crate::{SettingsError, data_dir};

pub const MAX_CUSTOM_THEMES: usize = 128;

/// Where the custom themes live.
pub fn themes_dir() -> Result<PathBuf, SettingsError> {
    Ok(data_dir()?.join("themes"))
}

/// What the folder held: the usable themes and a sentence for every file that was left out.
#[derive(Debug, Default)]
pub struct CustomThemes {
    pub themes: Vec<Theme>,
    pub problems: Vec<String>,
}

/// Reads a file the user picked: it must be a regular file under the size limit, and is only read after
/// that is known, so a device or a huge file cannot hang or exhaust the app.
pub fn read_theme_file(path: &Path) -> Result<Vec<u8>, String> {
    let shown = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "That file".to_owned());
    let metadata =
        fs::metadata(path).map_err(|error| format!("Could not open “{shown}”: {error}."))?;
    if !metadata.is_file() {
        return Err(format!("“{shown}” is not a file."));
    }
    if metadata.len() > MAX_THEME_BYTES as u64 {
        return Err(format!(
            "“{shown}” is larger than {} KiB, which is too big for a theme.",
            MAX_THEME_BYTES / 1024
        ));
    }
    fs::read(path).map_err(|error| format!("Could not read “{shown}”: {error}."))
}

pub fn load_custom_themes() -> CustomThemes {
    match themes_dir() {
        Ok(dir) => load_custom_themes_from(&dir),
        Err(error) => CustomThemes {
            themes: Vec::new(),
            problems: vec![error.to_string()],
        },
    }
}

pub fn load_custom_themes_from(dir: &Path) -> CustomThemes {
    let mut found = CustomThemes::default();
    let Ok(entries) = fs::read_dir(dir) else {
        return found; // no folder yet is the normal case
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "toml"))
        .collect();
    files.sort();

    let mut taken: HashSet<String> = built_in_themes().into_iter().map(|t| t.id).collect();
    for path in files {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        if found.themes.len() >= MAX_CUSTOM_THEMES {
            found.problems.push(format!(
                "“{name}” was skipped: at most {MAX_CUSTOM_THEMES} custom themes are loaded."
            ));
            continue;
        }
        let theme = read_theme_file(&path)
            .and_then(|bytes| {
                String::from_utf8(bytes).map_err(|_| format!("“{name}” is not UTF-8 text."))
            })
            .and_then(|text| Theme::from_toml(&text).map_err(|error| format!("“{name}”: {error}")));
        match theme {
            Ok(theme) if taken.insert(theme.id.clone()) => found.themes.push(theme),
            Ok(theme) => found.problems.push(format!(
                "“{name}” was skipped: another theme already uses the id “{}”.",
                theme.id
            )),
            Err(problem) => found.problems.push(problem),
        }
    }
    // By name, so the gallery lists them the way a person would look for them, not by file name.
    found
        .themes
        .sort_by_cached_key(|theme| (theme.name.to_lowercase(), theme.id.clone()));
    found
}

/// Saves `theme` as `<id>.toml`, first making its id unique among the built-in and the custom themes.
/// Returns the theme as it was saved.
pub fn save_custom_theme_to(dir: &Path, theme: Theme) -> Result<Theme, String> {
    let existing = load_custom_themes_from(dir).themes;
    if existing.len() >= MAX_CUSTOM_THEMES {
        return Err(format!(
            "At most {MAX_CUSTOM_THEMES} custom themes can be kept. Delete one first."
        ));
    }
    let mut theme = theme.normalized()?;
    let taken: HashSet<String> = built_in_themes()
        .into_iter()
        .chain(existing)
        .map(|t| t.id)
        .collect();
    let base = theme.id.clone();
    let mut number = 2;
    while taken.contains(&theme.id) {
        theme.id = format!("{base}-{number}");
        number += 1;
    }
    let text = theme.to_toml()?;
    write_atomic(&dir.join(format!("{}.toml", theme.id)), text.as_bytes())
        .map_err(|error| error.to_string())?;
    Ok(theme)
}

/// Deletes a custom theme. Built-in themes cannot be deleted, and `id` can never leave the folder.
pub fn delete_custom_theme_from(dir: &Path, id: &str) -> Result<(), String> {
    if id.is_empty() || slug(id) != id {
        return Err("That is not the id of a custom theme.".to_owned());
    }
    let owned = load_custom_themes_from(dir)
        .themes
        .iter()
        .any(|theme| theme.id == id);
    if !owned {
        return Err("Only themes you added can be deleted.".to_owned());
    }
    // The id lives inside the file, so the file may be named otherwise: find it.
    let Ok(entries) = fs::read_dir(dir) else {
        return Err("The themes folder cannot be read.".to_owned());
    };
    for path in entries.filter_map(Result::ok).map(|entry| entry.path()) {
        let is_it = read_theme_file(&path)
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .and_then(|text| Theme::from_toml(&text).ok())
            .is_some_and(|theme| theme.id == id);
        if is_it {
            return fs::remove_file(&path)
                .map_err(|error| format!("Could not delete it: {error}."));
        }
    }
    Err("Only themes you added can be deleted.".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::built_in_theme;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("termzip-themes-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn mine(name: &str) -> Theme {
        let mut theme = built_in_theme("nord").unwrap();
        theme.id = String::new();
        theme.name = name.to_owned();
        theme
    }

    #[test]
    fn a_missing_folder_is_just_an_empty_library() {
        let found = load_custom_themes_from(&temp_dir("missing"));
        assert!(found.themes.is_empty() && found.problems.is_empty());
    }

    #[test]
    fn a_saved_theme_comes_back_and_ids_stay_unique() {
        let dir = temp_dir("save");
        let first = save_custom_theme_to(&dir, mine("Ocean")).unwrap();
        let second = save_custom_theme_to(&dir, mine("Ocean")).unwrap();
        // A name that is a built-in id must not shadow the built-in theme.
        let shadow = save_custom_theme_to(&dir, mine("Nord")).unwrap();

        assert_eq!(first.id, "ocean");
        assert_eq!(second.id, "ocean-2");
        assert_eq!(shadow.id, "nord-2");
        let found = load_custom_themes_from(&dir);
        let ids: Vec<_> = found.themes.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, ["nord-2", "ocean", "ocean-2"]);
        assert!(found.problems.is_empty(), "{:?}", found.problems);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn files_that_cannot_be_used_are_skipped_and_explained() {
        let dir = temp_dir("bad");
        save_custom_theme_to(&dir, mine("Good")).unwrap();
        fs::write(dir.join("broken.toml"), "this is not a theme").unwrap();
        fs::write(dir.join("binary.toml"), [0xff, 0xfe, 0x00]).unwrap();
        fs::write(dir.join("notes.txt"), "ignored").unwrap();
        fs::write(dir.join("huge.toml"), vec![b'#'; MAX_THEME_BYTES + 1]).unwrap();
        // Another file claiming a built-in id, and one repeating an id already loaded.
        let dracula = built_in_theme("dracula").unwrap().to_toml().unwrap();
        fs::write(dir.join("copy-of-dracula.toml"), dracula).unwrap();
        fs::copy(dir.join("good.toml"), dir.join("zz-good-copy.toml")).unwrap();

        let found = load_custom_themes_from(&dir);

        let ids: Vec<_> = found.themes.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, ["good"]);
        assert_eq!(found.problems.len(), 5, "{:?}", found.problems);
        for name in [
            "broken.toml",
            "binary.toml",
            "huge.toml",
            "copy-of-dracula.toml",
            "zz-good-copy.toml",
        ] {
            assert!(
                found.problems.iter().any(|p| p.contains(name)),
                "{name}: {:?}",
                found.problems
            );
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_folder_that_is_a_directory_named_like_a_theme_is_not_read() {
        let dir = temp_dir("dirs");
        fs::create_dir_all(dir.join("sneaky.toml")).unwrap();

        let found = load_custom_themes_from(&dir);

        assert!(found.themes.is_empty());
        assert_eq!(found.problems.len(), 1, "{:?}", found.problems);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn only_so_many_custom_themes_are_kept() {
        let dir = temp_dir("many");
        let base = built_in_theme("nord").unwrap();
        for n in 0..MAX_CUSTOM_THEMES + 2 {
            let mut theme = base.clone();
            theme.id = format!("t{n:03}");
            theme.name = format!("T {n}");
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join(format!("t{n:03}.toml")), theme.to_toml().unwrap()).unwrap();
        }

        let found = load_custom_themes_from(&dir);
        assert_eq!(found.themes.len(), MAX_CUSTOM_THEMES);
        assert_eq!(found.problems.len(), 2);
        assert!(
            save_custom_theme_to(&dir, mine("One more"))
                .unwrap_err()
                .contains("Delete one")
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn only_custom_themes_can_be_deleted_and_never_outside_the_folder() {
        let dir = temp_dir("delete");
        let saved = save_custom_theme_to(&dir, mine("Ocean")).unwrap();
        let outside = dir
            .parent()
            .unwrap()
            .join(format!("termzip-outside-{}.toml", std::process::id()));
        fs::write(&outside, "keep me").unwrap();

        for id in [
            "nord",
            "termzip-dark",
            "",
            "../x",
            "ocean/../../x",
            "Ocean",
            "ghost",
        ] {
            assert!(delete_custom_theme_from(&dir, id).is_err(), "{id:?}");
        }
        assert!(outside.exists());
        assert!(dir.join("ocean.toml").exists());

        delete_custom_theme_from(&dir, &saved.id).unwrap();
        assert!(!dir.join("ocean.toml").exists());
        let _ = fs::remove_file(&outside);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_picked_file_must_be_a_small_regular_file() {
        let dir = temp_dir("picked");
        fs::create_dir_all(&dir).unwrap();
        let ok = dir.join("ok.conf");
        fs::write(&ok, "background #000000").unwrap();
        let huge = dir.join("huge.conf");
        fs::write(&huge, vec![b'x'; MAX_THEME_BYTES + 1]).unwrap();

        assert_eq!(read_theme_file(&ok).unwrap(), b"background #000000");
        assert!(read_theme_file(&huge).unwrap_err().contains("larger than"));
        assert!(read_theme_file(&dir).unwrap_err().contains("not a file"));
        assert!(
            read_theme_file(&dir.join("missing"))
                .unwrap_err()
                .contains("Could not open")
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
