use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ThemeKind;

pub const DEFAULT_FONT_SIZE: f32 = 14.0;
pub const DEFAULT_SCROLLBACK_LINES: usize = 10_000;
pub const MIN_FONT_SIZE: f32 = 10.0;
pub const MAX_FONT_SIZE: f32 = 28.0;
pub const MIN_SCROLLBACK_LINES: usize = 1_000;
pub const MAX_SCROLLBACK_LINES: usize = 200_000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub theme: ThemeKind,
    pub font_size: f32,
    pub font_family: Option<String>,
    pub scrollback_lines: usize,
    pub shell: Option<String>,
    pub working_directory: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: ThemeKind::Dark,
            font_size: DEFAULT_FONT_SIZE,
            font_family: None,
            scrollback_lines: DEFAULT_SCROLLBACK_LINES,
            shell: None,
            working_directory: None,
        }
    }
}

impl Settings {
    pub fn load() -> Result<Self, SettingsError> {
        let path = settings_path()?;
        match fs::read_to_string(&path) {
            Ok(contents) => Ok(Self::from_toml(&contents)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(source) => Err(SettingsError::Read { path, source }),
        }
    }

    pub fn from_toml(contents: &str) -> Self {
        toml::from_str(contents)
            .map(Self::normalized)
            .unwrap_or_default()
    }

    pub fn normalized(mut self) -> Self {
        self.font_size = if self.font_size.is_finite() {
            self.font_size.clamp(MIN_FONT_SIZE, MAX_FONT_SIZE)
        } else {
            DEFAULT_FONT_SIZE
        };
        self.scrollback_lines = self
            .scrollback_lines
            .clamp(MIN_SCROLLBACK_LINES, MAX_SCROLLBACK_LINES);
        self.font_family = self
            .font_family
            .map(|family| family.trim().to_owned())
            .filter(|family| !family.is_empty());
        self.shell = self
            .shell
            .map(|shell| shell.trim().to_owned())
            .filter(|shell| !shell.is_empty());
        self.working_directory = self
            .working_directory
            .map(|directory| directory.trim().to_owned())
            .filter(|directory| !directory.is_empty());
        self
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

        let contents = toml::to_string_pretty(&self.clone().normalized())?;
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

    pub fn effective_shell(&self) -> Option<PathBuf> {
        self.shell.as_deref().map(PathBuf::from)
    }

    pub fn effective_working_directory(&self) -> Option<PathBuf> {
        self.working_directory.as_deref().map(PathBuf::from)
    }
}

pub fn settings_path() -> Result<PathBuf, SettingsError> {
    let root = dirs::config_dir().ok_or(SettingsError::ConfigDirectoryUnavailable)?;
    Ok(root.join("termzip").join("config.toml"))
}

/// Directory that holds the bundled shell-integration scripts.
pub fn integration_dir() -> Result<PathBuf, SettingsError> {
    let root = dirs::config_dir().ok_or(SettingsError::ConfigDirectoryUnavailable)?;
    Ok(root.join("termzip").join("shell-integration"))
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
    #[error("settings could not be encoded as TOML")]
    Encode(#[from] toml::ser::Error),
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_FONT_SIZE, MAX_SCROLLBACK_LINES, Settings};

    #[test]
    fn defaults_are_safe_and_portable() {
        let settings = Settings::default();

        assert_eq!(settings.font_size, DEFAULT_FONT_SIZE);
        assert_eq!(settings.scrollback_lines, 10_000);
        assert!(settings.shell.is_none());
        assert!(settings.working_directory.is_none());
        assert!(settings.font_family.is_none());
    }

    #[test]
    fn missing_fields_in_existing_config_use_defaults() {
        let settings = Settings::from_toml("theme = 'light'\n");

        assert_eq!(settings.theme, crate::ThemeKind::Light);
        assert_eq!(settings.font_size, DEFAULT_FONT_SIZE);
    }

    #[test]
    fn invalid_existing_config_falls_back_to_defaults() {
        let settings = Settings::from_toml("this is not = toml = at all");

        assert_eq!(settings, Settings::default());
    }

    #[test]
    fn values_are_clamped_and_blank_paths_are_removed() {
        let settings = Settings {
            font_size: 200.0,
            scrollback_lines: 500_000,
            shell: Some("   ".to_owned()),
            working_directory: Some("  /tmp  ".to_owned()),
            ..Settings::default()
        }
        .normalized();

        assert_eq!(settings.font_size, super::MAX_FONT_SIZE);
        assert_eq!(settings.scrollback_lines, MAX_SCROLLBACK_LINES);
        assert!(settings.shell.is_none());
        assert_eq!(settings.working_directory.as_deref(), Some("/tmp"));
    }
}
