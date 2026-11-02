mod appearance;
mod keybindings;
mod profile;
mod settings;
mod snippet;
mod storage;
mod text;
mod theme;
mod workspace;

pub use appearance::CursorStyle;
pub use keybindings::{MAX_KEYBINDINGS, normalize_chord, validate_keybindings};
pub use profile::{DEFAULT_PROFILE_ID, LaunchProfile, Profile, slug, validate_profiles};
pub use settings::{HOME_ENV, Settings, SettingsError, data_dir, integration_dir, settings_path};
pub use snippet::{
    MAX_PARAMETER_NAME_CHARS, MAX_PARAMETER_VALUE_BYTES, MAX_PARAMETERS, MAX_SNIPPET_COMMAND_BYTES,
    MAX_SNIPPET_DESCRIPTION_CHARS, MAX_SNIPPET_FOLDER_BYTES, MAX_SNIPPET_NAME_CHARS, MAX_SNIPPETS,
    Parameter, Snippet, check_command, parameters, render, validate_snippets,
};
pub use storage::write_atomic;
pub use text::{clean_typed, hides_or_reorders};
pub use theme::{
    Chrome, Color, Colors, CustomThemes, DEFAULT_DARK_ID, DEFAULT_LIGHT_ID, MAX_CUSTOM_THEMES,
    MAX_THEME_BYTES, Theme, ThemeKind, built_in_theme, built_in_themes, delete_custom_theme_from,
    import_theme, load_custom_themes, load_custom_themes_from, read_theme_file,
    save_custom_theme_to, themes_dir,
};
pub use workspace::{Direction, PaneNode, Tab, Workspace, workspace_path};
