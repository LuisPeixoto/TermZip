//! Tauri commands exposed to the webview.

use serde::{Deserialize, Serialize};
use tauri::State;
use tauri::ipc::Channel;

use crate::sessions::{SessionEvent, SessionInfo, Sessions};

const DEFAULT_ROWS: u16 = 24;
const DEFAULT_COLUMNS: u16 = 80;

/// Webview-facing settings use camelCase; the TOML file keeps snake_case.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsPayload {
    pub theme: termzip_core::ThemeKind,
    pub font_size: f32,
    pub font_family: Option<String>,
    pub scrollback_lines: usize,
    pub shell: Option<String>,
    pub working_directory: Option<String>,
}

impl From<termzip_core::Settings> for SettingsPayload {
    fn from(settings: termzip_core::Settings) -> Self {
        Self {
            theme: settings.theme,
            font_size: settings.font_size,
            font_family: settings.font_family,
            scrollback_lines: settings.scrollback_lines,
            shell: settings.shell,
            working_directory: settings.working_directory,
        }
    }
}

impl From<SettingsPayload> for termzip_core::Settings {
    fn from(payload: SettingsPayload) -> Self {
        Self {
            theme: payload.theme,
            font_size: payload.font_size,
            font_family: payload.font_family,
            scrollback_lines: payload.scrollback_lines,
            shell: payload.shell,
            working_directory: payload.working_directory,
        }
        .normalized()
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
    let settings = termzip_core::Settings::from(settings);
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

/// Starts a shell. Output, block updates and the exit all arrive on `on_event`.
#[tauri::command]
pub fn create_session(
    sessions: State<'_, Sessions>,
    on_event: Channel<SessionEvent>,
    rows: Option<u16>,
    columns: Option<u16>,
    working_directory: Option<String>,
    shell: Option<String>,
) -> Result<SessionInfo, String> {
    let integration_dir = termzip_core::integration_dir().map_err(|error| error.to_string())?;
    sessions
        .create(
            on_event,
            rows.unwrap_or(DEFAULT_ROWS).max(1),
            columns.unwrap_or(DEFAULT_COLUMNS).max(1),
            working_directory,
            shell,
            &integration_dir,
        )
        .map_err(|error| error.to_string())
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
