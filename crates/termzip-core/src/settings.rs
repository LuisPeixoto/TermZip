use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ThemeKind;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub theme: ThemeKind,
    pub font_size: f32,
    pub scrollback_lines: usize,
    pub shell: Option<String>,
    pub working_directory: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: ThemeKind::Dark,
            font_size: 15.0,
            scrollback_lines: 10_000,
            shell: None,
            working_directory: None,
        }
    }
}

impl Settings {
    pub fn load() -> Result<Self, SettingsError> {
        let path = settings_path()?;
        match fs::read_to_string(&path) {
            Ok(contents) => Ok(toml::from_str(&contents)?),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(source) => Err(SettingsError::Read { path, source }),
        }
    }

    pub fn save(&self) -> Result<(), SettingsError> {
        let path = settings_path()?;
        let parent = path.parent().ok_or(SettingsError::InvalidPath)?;
        fs::create_dir_all(parent).map_err(|source| SettingsError::Write {
            path: parent.to_owned(),
            source,
        })?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(parent, fs::Permissions::from_mode(0o700)).map_err(|source| {
                SettingsError::Write {
                    path: parent.to_owned(),
                    source,
                }
            })?;
        }

        let contents = toml::to_string_pretty(self)?;
        fs::write(&path, contents).map_err(|source| SettingsError::Write {
            path: path.clone(),
            source,
        })?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).map_err(|source| {
                SettingsError::Write {
                    path: path.clone(),
                    source,
                }
            })?;
        }

        Ok(())
    }
}

pub fn settings_path() -> Result<PathBuf, SettingsError> {
    let root = dirs::config_dir().ok_or(SettingsError::ConfigDirectoryUnavailable)?;
    Ok(root.join("termzip").join("config.toml"))
}

#[derive(Debug, Error)]
pub enum SettingsError {
    #[error("the operating system did not provide a configuration directory")]
    ConfigDirectoryUnavailable,
    #[error("could not determine the configuration file path")]
    InvalidPath,
    #[error("could not read settings from {path}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("could not write settings to {path}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("settings are not valid TOML")]
    Parse(#[from] toml::de::Error),
    #[error("settings could not be encoded as TOML")]
    Encode(#[from] toml::ser::Error),
}

#[cfg(test)]
mod tests {
    use super::Settings;

    #[test]
    fn defaults_are_safe_and_portable() {
        let settings = Settings::default();

        assert_eq!(settings.font_size, 15.0);
        assert_eq!(settings.scrollback_lines, 10_000);
        assert!(settings.shell.is_none());
        assert!(settings.working_directory.is_none());
    }

    #[test]
    fn missing_fields_in_existing_config_use_defaults() {
        let settings: Settings = toml::from_str("theme = 'light'\n").unwrap();

        assert_eq!(settings.theme, crate::ThemeKind::Light);
        assert_eq!(settings.font_size, 15.0);
    }
}
