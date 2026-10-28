//! Theme commands: the gallery, and importing and exporting theme files.
//!
//! The file dialogs are opened from here, never from the webview, so only what the user picks is ever
//! read, and a file is only written where the user chose to put it.

use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;
use termzip_core::{CustomThemes, Theme};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeEntry {
    /// Built-in themes cannot be deleted.
    pub built_in: bool,
    /// Serialized like the theme file (snake_case keys), so it matches `docs/themes.md`.
    pub theme: Theme,
}

#[derive(Debug, Clone, Serialize)]
pub struct ThemeList {
    pub themes: Vec<ThemeEntry>,
    /// One sentence for every custom theme file that was left out.
    pub problems: Vec<String>,
}

/// The built-in themes, then the custom ones.
fn collect(custom: CustomThemes) -> ThemeList {
    let themes = termzip_core::built_in_themes()
        .into_iter()
        .map(|theme| ThemeEntry {
            built_in: true,
            theme,
        })
        .chain(custom.themes.into_iter().map(|theme| ThemeEntry {
            built_in: false,
            theme,
        }))
        .collect();
    ThemeList {
        themes,
        problems: custom.problems,
    }
}

fn find(id: &str) -> Option<Theme> {
    collect(termzip_core::load_custom_themes())
        .themes
        .into_iter()
        .map(|entry| entry.theme)
        .find(|theme| theme.id == id)
}

#[tauri::command]
pub fn list_themes() -> ThemeList {
    collect(termzip_core::load_custom_themes())
}

/// Asks for a theme file, converts it and keeps it in the themes folder. `None` when the dialog is
/// cancelled. Async so the dialog runs off the main thread, which a blocking dialog requires.
#[tauri::command]
pub async fn import_theme(app: AppHandle) -> Result<Option<ThemeEntry>, String> {
    // No filter: Ghostty and Kitty themes often have no extension at all.
    let Some(picked) = app
        .dialog()
        .file()
        .set_title("Import a theme")
        .blocking_pick_file()
    else {
        return Ok(None);
    };
    let path = picked.into_path().map_err(|error| error.to_string())?;
    let bytes = termzip_core::read_theme_file(&path)?;
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let theme = termzip_core::import_theme(&name, &bytes)?;
    let dir = termzip_core::themes_dir().map_err(|error| error.to_string())?;
    let theme = termzip_core::save_custom_theme_to(&dir, theme)?;
    Ok(Some(ThemeEntry {
        built_in: false,
        theme,
    }))
}

/// Writes a theme in TermZip's format where the user chooses. `false` when the dialog is cancelled.
#[tauri::command]
pub async fn export_theme(app: AppHandle, id: String) -> Result<bool, String> {
    let theme = find(&id).ok_or("That theme no longer exists.")?;
    let text = theme.to_toml()?;
    let Some(picked) = app
        .dialog()
        .file()
        .set_title("Export the theme")
        .set_file_name(format!("{}.toml", theme.id))
        .add_filter("TermZip theme", &["toml"])
        .blocking_save_file()
    else {
        return Ok(false);
    };
    let path = picked.into_path().map_err(|error| error.to_string())?;
    // A plain write: the folder is the user's, so its permissions are left alone.
    std::fs::write(&path, text).map_err(|error| format!("Could not write the file: {error}."))?;
    Ok(true)
}

#[tauri::command]
pub fn delete_theme(id: String) -> Result<(), String> {
    let dir = termzip_core::themes_dir().map_err(|error| error.to_string())?;
    termzip_core::delete_custom_theme_from(&dir, &id)
}

#[cfg(test)]
mod tests {
    use termzip_core::{CustomThemes, built_in_theme};

    use super::collect;

    #[test]
    fn built_in_themes_come_first_and_cannot_be_deleted() {
        let mut mine = built_in_theme("nord").unwrap();
        mine.id = "mine".to_owned();
        let list = collect(CustomThemes {
            themes: vec![mine],
            problems: vec!["“broken.toml” is not a theme.".to_owned()],
        });

        let flags: Vec<_> = list.themes.iter().map(|entry| entry.built_in).collect();
        assert!(flags[..flags.len() - 1].iter().all(|&built_in| built_in));
        assert!(!flags[flags.len() - 1]);
        assert_eq!(list.themes[0].theme.id, "termzip-dark");
        assert_eq!(list.themes.last().unwrap().theme.id, "mine");
        assert_eq!(list.problems.len(), 1);
    }

    /// The webview's types depend on this exact shape.
    #[test]
    fn the_json_shape_is_what_the_webview_expects() {
        let json = serde_json::to_value(collect(CustomThemes::default())).unwrap();
        let first = &json["themes"][0];

        assert_eq!(first["builtIn"], true);
        assert_eq!(first["theme"]["id"], "termzip-dark");
        assert_eq!(first["theme"]["appearance"], "dark");
        assert_eq!(first["theme"]["colors"]["background"], "#09090b");
        assert_eq!(first["theme"]["colors"]["cursor_text"], "#09090b");
        assert_eq!(
            first["theme"]["colors"]["ansi"].as_array().unwrap().len(),
            16
        );
        assert_eq!(first["theme"]["chrome"]["line_strong"], "#3f3f46");
        // Unset colors are left out rather than sent as null.
        assert!(first["theme"]["colors"].get("accent").is_none());
        assert!(json["problems"].as_array().unwrap().is_empty());
    }
}
