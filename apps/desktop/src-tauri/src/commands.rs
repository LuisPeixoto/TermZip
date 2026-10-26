//! Tauri commands exposed to the webview.

use tauri::State;

use crate::sessions::{SessionInfo, Sessions};

const DEFAULT_ROWS: u16 = 24;
const DEFAULT_COLUMNS: u16 = 80;

#[tauri::command]
pub fn get_settings() -> Result<termzip_core::Settings, String> {
    termzip_core::Settings::load().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn save_settings(settings: termzip_core::Settings) -> Result<termzip_core::Settings, String> {
    let settings = settings.clone().normalized();
    settings.save().map_err(|error| error.to_string())?;
    Ok(settings)
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

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub platform: String,
    pub config_path: String,
}

#[tauri::command]
pub fn create_session(
    app: tauri::AppHandle,
    sessions: State<'_, Sessions>,
    rows: Option<u16>,
    columns: Option<u16>,
    working_directory: Option<String>,
    shell: Option<String>,
) -> Result<SessionInfo, String> {
    let integration_dir = termzip_core::integration_dir().map_err(|error| error.to_string())?;
    sessions
        .create(
            &app,
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
pub fn take_backlog(sessions: State<'_, Sessions>, session_id: u32) -> Result<String, String> {
    let handle = sessions
        .get(session_id)
        .map_err(|error| error.to_string())?;
    Ok(handle.take_backlog())
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
