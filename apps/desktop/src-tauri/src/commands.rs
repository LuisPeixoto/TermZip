//! Tauri commands exposed to the webview.

use serde::{Deserialize, Serialize};
use tauri::State;
use tauri::ipc::Channel;

use crate::sessions::{SessionEvent, SessionInfo, SessionRequest, Sessions};

const DEFAULT_ROWS: u16 = 24;
const DEFAULT_COLUMNS: u16 = 80;

/// Webview-facing settings use camelCase; the TOML file keeps snake_case.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsPayload {
    pub theme: termzip_core::ThemeKind,
    pub dark_theme: String,
    pub light_theme: String,
    pub cursor_style: termzip_core::CursorStyle,
    pub cursor_blink: bool,
    pub line_height: f32,
    pub letter_spacing: f32,
    pub background_opacity: f32,
    pub background_blur: bool,
    pub font_size: f32,
    pub font_family: Option<String>,
    pub scrollback_lines: usize,
    pub shell: Option<String>,
    pub working_directory: Option<String>,
    pub restore_session: bool,
    pub default_profile: Option<String>,
    pub keybindings: std::collections::BTreeMap<String, String>,
    pub profiles: Vec<ProfilePayload>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilePayload {
    pub id: String,
    pub name: String,
    pub shell: Option<String>,
    pub working_directory: Option<String>,
    pub theme: Option<String>,
    pub font_size: Option<f32>,
    pub font_family: Option<String>,
    pub cursor_style: Option<termzip_core::CursorStyle>,
    pub env: std::collections::BTreeMap<String, String>,
}

impl From<termzip_core::Profile> for ProfilePayload {
    fn from(profile: termzip_core::Profile) -> Self {
        Self {
            id: profile.id,
            name: profile.name,
            shell: profile.shell,
            working_directory: profile.working_directory,
            theme: profile.theme,
            font_size: profile.font_size,
            font_family: profile.font_family,
            cursor_style: profile.cursor_style,
            env: profile.env,
        }
    }
}

impl From<ProfilePayload> for termzip_core::Profile {
    fn from(payload: ProfilePayload) -> Self {
        Self {
            id: payload.id,
            name: payload.name,
            shell: payload.shell,
            working_directory: payload.working_directory,
            theme: payload.theme,
            font_size: payload.font_size,
            font_family: payload.font_family,
            cursor_style: payload.cursor_style,
            env: payload.env,
        }
    }
}

impl From<termzip_core::Settings> for SettingsPayload {
    fn from(settings: termzip_core::Settings) -> Self {
        Self {
            theme: settings.theme,
            dark_theme: settings.dark_theme,
            light_theme: settings.light_theme,
            cursor_style: settings.cursor_style,
            cursor_blink: settings.cursor_blink,
            line_height: settings.line_height,
            letter_spacing: settings.letter_spacing,
            background_opacity: settings.background_opacity,
            background_blur: settings.background_blur,
            keybindings: settings.keybindings,
            font_size: settings.font_size,
            font_family: settings.font_family,
            scrollback_lines: settings.scrollback_lines,
            shell: settings.shell,
            working_directory: settings.working_directory,
            restore_session: settings.restore_session,
            default_profile: settings.default_profile,
            profiles: settings.profiles.into_iter().map(Into::into).collect(),
        }
    }
}

impl SettingsPayload {
    /// The settings exactly as the dialog sent them, before the clean-up that drops what cannot be
    /// used, so `validate` can still explain it.
    fn into_raw(self) -> termzip_core::Settings {
        termzip_core::Settings {
            theme: self.theme,
            dark_theme: self.dark_theme,
            light_theme: self.light_theme,
            cursor_style: self.cursor_style,
            cursor_blink: self.cursor_blink,
            line_height: self.line_height,
            letter_spacing: self.letter_spacing,
            background_opacity: self.background_opacity,
            background_blur: self.background_blur,
            font_size: self.font_size,
            font_family: self.font_family,
            scrollback_lines: self.scrollback_lines,
            shell: self.shell,
            working_directory: self.working_directory,
            restore_session: self.restore_session,
            default_profile: self.default_profile,
            keybindings: self.keybindings,
            profiles: self.profiles.into_iter().map(Into::into).collect(),
        }
    }
}

#[tauri::command]
pub fn get_settings() -> Result<SettingsPayload, String> {
    termzip_core::Settings::load()
        .map(SettingsPayload::from)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn save_settings(settings: SettingsPayload) -> Result<SettingsPayload, String> {
    // Explain a bad profile or shortcut as it is, before the clean-up quietly drops what cannot be used.
    let raw = settings.into_raw();
    raw.validate()?;
    let settings = raw.normalized();
    settings.save().map_err(|error| error.to_string())?;
    Ok(SettingsPayload::from(settings))
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub platform: String,
    pub config_path: String,
}

#[tauri::command]
pub fn app_info() -> Result<AppInfo, String> {
    Ok(AppInfo {
        version: env!("CARGO_PKG_VERSION").to_owned(),
        platform: std::env::consts::OS.to_owned(),
        config_path: termzip_core::settings_path()
            .map(|path| path.display().to_string())
            .unwrap_or_default(),
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellOption {
    pub path: String,
    pub label: String,
    pub default: bool,
    pub integrated: bool,
}

#[tauri::command]
pub fn detect_shells() -> Vec<ShellOption> {
    use std::collections::BTreeMap;
    use std::path::PathBuf;
    use termzip_shell::ShellKind;

    fn candidate_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(shell) = std::env::var_os("SHELL") {
            paths.push(PathBuf::from(shell));
        }

        #[cfg(unix)]
        {
            if let Ok(contents) = std::fs::read_to_string("/etc/shells") {
                for line in contents.lines() {
                    let line = line.trim();
                    if line.starts_with('#') || line.is_empty() {
                        continue;
                    }
                    paths.push(PathBuf::from(line));
                }
            }
            for path in [
                "/bin/zsh",
                "/bin/bash",
                "/usr/bin/fish",
                "/usr/local/bin/fish",
                "/opt/homebrew/bin/fish",
                "/usr/bin/pwsh",
                "/usr/local/bin/pwsh",
            ] {
                paths.push(PathBuf::from(path));
            }
        }

        #[cfg(windows)]
        {
            if let Some(shell) = std::env::var_os("COMSPEC") {
                paths.push(PathBuf::from(shell));
            }
            for name in ["pwsh.exe", "powershell.exe", "cmd.exe"] {
                let found = std::env::var_os("PATH").and_then(|value| {
                    std::env::split_paths(&value)
                        .map(|directory| directory.join(name))
                        .find(|candidate| candidate.is_file())
                });
                if let Some(found) = found {
                    paths.push(found);
                }
            }
        }
        paths
    }

    let default = termzip_shell::default_shell();
    let mut seen: BTreeMap<String, ShellOption> = BTreeMap::new();
    for path in candidate_paths() {
        if !path.is_file() && !path.is_absolute() {
            continue;
        }
        let canonical = path.canonicalize().unwrap_or(path);
        let shown = canonical.display().to_string();
        if seen.contains_key(&shown) {
            continue;
        }
        let kind = termzip_shell::detect_shell_kind(&canonical);
        if kind == ShellKind::Other && canonical.extension().is_some_and(|ext| ext == "app") {
            continue;
        }
        seen.insert(
            shown.clone(),
            ShellOption {
                path: shown.clone(),
                label: format!("{} · {}", kind.display_name(), shown),
                default: same_path(&canonical, &default),
                integrated: kind.integrates(),
            },
        );
    }

    let mut shells: Vec<_> = seen.into_values().collect();
    shells.sort_by(|a, b| {
        b.default
            .cmp(&a.default)
            .then_with(|| a.label.to_lowercase().cmp(&b.label.to_lowercase()))
    });
    shells
}

fn same_path(a: &std::path::Path, b: &std::path::Path) -> bool {
    a == b
        || a.canonicalize().ok() == b.canonicalize().ok()
        || a.file_name()
            .zip(b.file_name())
            .is_some_and(|(a, b)| a == b)
}

/// What a new session starts with, once its profile was resolved against the saved settings.
struct SessionStart {
    shell: Option<String>,
    working_directory: Option<String>,
    env: std::collections::BTreeMap<String, String>,
}

/// `restored` is the folder a pane was in when the layout was saved: it wins while it still exists,
/// otherwise the profile's own folder applies.
fn resolve_start(
    settings: &termzip_core::Settings,
    profile_id: Option<&str>,
    restored: Option<&str>,
) -> SessionStart {
    let launch = settings.launch_profile(profile_id);
    let restored = restored
        .filter(|directory| !directory.is_empty() && std::path::Path::new(directory).is_dir())
        .map(str::to_owned);
    SessionStart {
        shell: launch.shell,
        working_directory: restored.or(launch.working_directory),
        env: launch.env,
    }
}

/// Starts a shell. Output, block updates and the exit all arrive on `on_event`. The profile, its
/// shell, folder and environment are resolved here from the saved settings.
#[tauri::command]
pub fn create_session(
    sessions: State<'_, Sessions>,
    on_event: Channel<SessionEvent>,
    rows: Option<u16>,
    columns: Option<u16>,
    profile_id: Option<String>,
    restore_directory: Option<String>,
) -> Result<SessionInfo, String> {
    let settings = termzip_core::Settings::load().map_err(|error| error.to_string())?;
    let start = resolve_start(
        &settings,
        profile_id.as_deref(),
        restore_directory.as_deref(),
    );
    let integration_dir = termzip_core::integration_dir().map_err(|error| error.to_string())?;
    sessions
        .create(
            on_event,
            SessionRequest {
                rows: rows.unwrap_or(DEFAULT_ROWS).max(1),
                columns: columns.unwrap_or(DEFAULT_COLUMNS).max(1),
                working_directory: start.working_directory,
                shell: start.shell,
                env: start.env,
            },
            &integration_dir,
        )
        .map_err(|error| error.to_string())
}

/// Opens what was clicked in the terminal: a URL in the browser, a file in its default application.
/// The target is validated first (see `links.rs`); a program is never started, its folder is opened.
#[tauri::command]
pub fn open_link(target: String, cwd: Option<String>) -> Result<(), String> {
    use crate::links::LinkTarget;

    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .map(std::path::PathBuf::from);
    let cwd = cwd.map(std::path::PathBuf::from);
    let opened = match crate::links::resolve(&target, cwd.as_deref(), home.as_deref())? {
        LinkTarget::Url(url) => open::that_detached(url),
        LinkTarget::Open(path) => open::that_detached(path),
        LinkTarget::Reveal(path) => match path.parent() {
            Some(folder) => open::that_detached(folder),
            None => return Err("That program has no folder to show.".to_owned()),
        },
    };
    opened.map_err(|error| format!("Could not open it: {error}"))
}

/// The layout saved by the last run, or nothing when restore is off or there is none.
#[tauri::command]
pub fn load_workspace() -> Result<Option<termzip_core::Workspace>, String> {
    let settings = termzip_core::Settings::load().map_err(|error| error.to_string())?;
    if !settings.restore_session {
        return Ok(None);
    }
    termzip_core::Workspace::load().map_err(|error| error.to_string())
}

/// Saves the layout. With restore off nothing is kept, and a layout saved earlier is deleted.
#[tauri::command]
pub fn save_workspace(workspace: termzip_core::Workspace) -> Result<(), String> {
    let settings = termzip_core::Settings::load().map_err(|error| error.to_string())?;
    if !settings.restore_session {
        return termzip_core::Workspace::forget().map_err(|error| error.to_string());
    }
    workspace.save().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn destroy_session(sessions: State<'_, Sessions>, session_id: u32) -> Result<(), String> {
    sessions.destroy(session_id);
    Ok(())
}

#[tauri::command]
pub fn write_session(
    sessions: State<'_, Sessions>,
    session_id: u32,
    data: String,
) -> Result<(), String> {
    let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, data.as_bytes())
        .map_err(|error| format!("invalid session payload: {error}"))?;
    sessions
        .get(session_id)
        .and_then(|handle| handle.write(&bytes))
        .map_err(|error| error.to_string())
}

/// The webview finished drawing `bytes` of output. This is what lets a flooding shell keep going.
#[tauri::command]
pub fn ack_session(
    sessions: State<'_, Sessions>,
    session_id: u32,
    bytes: u64,
) -> Result<(), String> {
    sessions
        .get(session_id)
        .map(|handle| handle.acknowledge(usize::try_from(bytes).unwrap_or(usize::MAX)))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn resize_session(
    sessions: State<'_, Sessions>,
    session_id: u32,
    rows: u16,
    columns: u16,
) -> Result<(), String> {
    sessions
        .get(session_id)
        .and_then(|handle| handle.resize(rows.max(1), columns.max(1)))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn list_blocks(
    sessions: State<'_, Sessions>,
    session_id: u32,
) -> Result<Vec<termzip_shell::Block>, String> {
    sessions
        .blocks_snapshot(session_id)
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use termzip_core::{Profile, Settings};

    use super::resolve_start;

    fn settings() -> Settings {
        let mut work = Profile {
            name: "Work".to_owned(),
            working_directory: Some("/srv/work".to_owned()),
            ..Profile::default()
        };
        work.env.insert("MODE".to_owned(), "work".to_owned());
        Settings {
            shell: Some("/bin/zsh".to_owned()),
            working_directory: Some("/home/me".to_owned()),
            profiles: vec![work],
            ..Settings::default()
        }
        .normalized()
    }

    #[test]
    fn a_restored_folder_that_still_exists_wins_over_the_profile() {
        let here = std::env::temp_dir();
        let start = resolve_start(&settings(), Some("work"), here.to_str());
        assert_eq!(start.working_directory.as_deref(), here.to_str());
        assert_eq!(start.env.get("MODE").map(String::as_str), Some("work"));
        assert_eq!(start.shell.as_deref(), Some("/bin/zsh"));
    }

    #[test]
    fn a_restored_folder_that_is_gone_falls_back_to_the_profile() {
        let start = resolve_start(&settings(), Some("work"), Some("/definitely/not/here"));
        assert_eq!(start.working_directory.as_deref(), Some("/srv/work"));
    }

    #[test]
    fn a_blank_restored_folder_is_ignored() {
        let start = resolve_start(&settings(), None, Some(""));
        assert_eq!(start.working_directory.as_deref(), Some("/home/me"));
    }

    #[test]
    fn an_unknown_profile_still_starts_a_shell() {
        let start = resolve_start(&settings(), Some("ghost"), None);
        assert_eq!(start.working_directory.as_deref(), Some("/home/me"));
        assert!(start.env.is_empty());
    }
}
