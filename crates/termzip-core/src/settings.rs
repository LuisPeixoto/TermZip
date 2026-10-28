use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ThemeKind;
use crate::profile::{
    DEFAULT_PROFILE_ID, LaunchProfile, Profile, normalize_profiles, validate_profiles,
};
use crate::storage::{set_aside, write_atomic};

pub const DEFAULT_FONT_SIZE: f32 = 14.0;
pub const DEFAULT_SCROLLBACK_LINES: usize = 10_000;
pub const MIN_FONT_SIZE: f32 = 10.0;
pub const MAX_FONT_SIZE: f32 = 28.0;
pub const MIN_SCROLLBACK_LINES: usize = 1_000;
pub const MAX_SCROLLBACK_LINES: usize = 200_000;

/// Environment variable that moves everything TermZip stores, so tests and demos never touch the user's data.
pub const HOME_ENV: &str = "TERMZIP_HOME";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub theme: ThemeKind,
    pub font_size: f32,
    pub font_family: Option<String>,
    pub scrollback_lines: usize,
    /// The shell of the built-in Default profile.
    pub shell: Option<String>,
    /// The working directory of the built-in Default profile.
    pub working_directory: Option<String>,
    /// Reopen the tabs, splits and folders of the last run.
    pub restore_session: bool,
    /// The profile a new tab starts with; `None` is the built-in one.
    pub default_profile: Option<String>,
    /// Kept last: TOML writes arrays of tables after the plain values.
    pub profiles: Vec<Profile>,
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
            restore_session: true,
            default_profile: None,
            profiles: Vec::new(),
        }
    }
}

impl Settings {
    pub fn load() -> Result<Self, SettingsError> {
        Self::load_from(&settings_path()?)
    }

    /// Reads the settings at `path`. A file that cannot be parsed is renamed to
    /// `<name>.invalid` and the defaults are used, so the next save cannot destroy it.
    pub fn load_from(path: &Path) -> Result<Self, SettingsError> {
        match fs::read_to_string(path) {
            Ok(contents) => match toml::from_str::<Self>(&contents) {
                Ok(settings) => Ok(settings.normalized()),
                Err(_) => {
                    set_aside(path);
                    Ok(Self::default())
                }
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(source) => Err(SettingsError::Read {
                path: path.to_owned(),
                source,
            }),
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
        self.profiles = normalize_profiles(self.profiles);
        let known: Vec<&str> = self.profiles.iter().map(|p| p.id.as_str()).collect();
        self.default_profile = self
            .default_profile
            .take()
            .filter(|id| known.contains(&id.as_str()));
        self
    }

    /// Problems that deserve an explanation instead of a silent clean-up, as a sentence to show.
    pub fn validate(&self) -> Result<(), String> {
        validate_profiles(&self.profiles)
    }

    /// What a new session starts with. `requested` is a profile id; an unknown or missing one falls
    /// back to the default profile, and anything a profile leaves empty comes from the built-in one.
    pub fn launch_profile(&self, requested: Option<&str>) -> LaunchProfile {
        let find = |id: &str| self.profiles.iter().find(|profile| profile.id == id);
        let chosen = requested
            .and_then(find)
            .or_else(|| self.default_profile.as_deref().and_then(find));
        match chosen {
            Some(profile) => LaunchProfile {
                id: profile.id.clone(),
                shell: profile.shell.clone().or_else(|| self.shell.clone()),
                working_directory: profile
                    .working_directory
                    .clone()
                    .or_else(|| self.working_directory.clone()),
                env: profile.env.clone(),
            },
            None => LaunchProfile {
                id: DEFAULT_PROFILE_ID.to_owned(),
                shell: self.shell.clone(),
                working_directory: self.working_directory.clone(),
                env: Default::default(),
            },
        }
    }

    pub fn save(&self) -> Result<(), SettingsError> {
        self.save_to(&settings_path()?)
    }

    /// Writes the settings next to `path` and renames them into place, so a crash
    /// mid-write cannot leave a half-written file behind.
    pub fn save_to(&self, path: &Path) -> Result<(), SettingsError> {
        let contents = toml::to_string_pretty(&self.clone().normalized())?;
        write_atomic(path, contents.as_bytes())
    }

    pub fn effective_shell(&self) -> Option<PathBuf> {
        self.shell.as_deref().map(PathBuf::from)
    }

    pub fn effective_working_directory(&self) -> Option<PathBuf> {
        self.working_directory.as_deref().map(PathBuf::from)
    }
}

/// Everything TermZip stores lives here: `$TERMZIP_HOME`, or `termzip` inside the OS configuration folder.
pub fn data_dir() -> Result<PathBuf, SettingsError> {
    data_dir_with(std::env::var_os(HOME_ENV), dirs::config_dir())
}

fn data_dir_with(
    home: Option<OsString>,
    config_dir: Option<PathBuf>,
) -> Result<PathBuf, SettingsError> {
    match home.filter(|home| !home.is_empty()) {
        Some(home) => Ok(PathBuf::from(home)),
        None => config_dir
            .map(|root| root.join("termzip"))
            .ok_or(SettingsError::ConfigDirectoryUnavailable),
    }
}

pub fn settings_path() -> Result<PathBuf, SettingsError> {
    Ok(data_dir()?.join("config.toml"))
}

/// Directory that holds the bundled shell-integration scripts.
pub fn integration_dir() -> Result<PathBuf, SettingsError> {
    Ok(data_dir()?.join("shell-integration"))
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
    #[error("{0}")]
    Invalid(String),
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::{Path, PathBuf};

    use super::{DEFAULT_FONT_SIZE, MAX_SCROLLBACK_LINES, Profile, Settings, data_dir_with};

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("termzip-settings-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn sibling(path: &Path, suffix: &str) -> PathBuf {
        let mut name = path.as_os_str().to_owned();
        name.push(suffix);
        PathBuf::from(name)
    }

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

    #[test]
    fn the_home_variable_overrides_the_data_folder() {
        let from_env = data_dir_with(Some(OsString::from("/tmp/lab")), Some("/cfg".into()));
        assert_eq!(from_env.unwrap(), PathBuf::from("/tmp/lab"));

        // Joined, not spelled out: the separator differs between platforms.
        let expected = PathBuf::from("/cfg").join("termzip");

        let default = data_dir_with(None, Some("/cfg".into()));
        assert_eq!(default.unwrap(), expected);

        let blank = data_dir_with(Some(OsString::from("")), Some("/cfg".into()));
        assert_eq!(blank.unwrap(), expected);

        assert!(data_dir_with(None, None).is_err());
    }

    #[test]
    fn a_broken_config_is_kept_aside_instead_of_being_overwritten() {
        let dir = temp_dir("broken");
        let path = dir.join("config.toml");
        std::fs::write(&path, "theme = 'light'\nthis is not = toml").unwrap();

        let loaded = Settings::load_from(&path).unwrap();
        assert_eq!(loaded, Settings::default());

        let backup = sibling(&path, ".invalid");
        assert_eq!(
            std::fs::read_to_string(&backup).unwrap(),
            "theme = 'light'\nthis is not = toml"
        );

        loaded.save_to(&path).unwrap();
        assert!(backup.exists(), "saving must not touch the backup");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn saving_replaces_the_file_atomically_and_leaves_no_temporary_file() {
        let dir = temp_dir("atomic");
        let path = dir.join("config.toml");
        Settings::default().save_to(&path).unwrap();
        let changed = Settings {
            theme: crate::ThemeKind::Light,
            font_size: 18.0,
            ..Settings::default()
        };
        changed.save_to(&path).unwrap();

        assert_eq!(Settings::load_from(&path).unwrap(), changed);
        let leftovers: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(leftovers, vec![OsString::from("config.toml")]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_config_loads_the_defaults_without_creating_anything() {
        let dir = temp_dir("missing");
        let path = dir.join("config.toml");

        assert_eq!(Settings::load_from(&path).unwrap(), Settings::default());
        assert!(!path.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn profile(name: &str) -> Profile {
        Profile {
            name: name.to_owned(),
            ..Profile::default()
        }
    }

    #[test]
    fn a_config_from_0_1_loads_with_the_new_fields_defaulted() {
        let settings =
            Settings::from_toml("theme = 'light'\nfont_size = 16.0\nshell = '/bin/zsh'\n");

        assert_eq!(settings.theme, crate::ThemeKind::Light);
        assert!(settings.restore_session);
        assert!(settings.profiles.is_empty());
        assert_eq!(settings.default_profile, None);
    }

    #[test]
    fn profiles_survive_a_round_trip_through_toml() {
        let mut work = profile("Work");
        work.shell = Some("/bin/bash".to_owned());
        work.env.insert("AWS_PROFILE".to_owned(), "work".to_owned());
        let settings = Settings {
            shell: Some("/bin/zsh".to_owned()),
            default_profile: Some("work".to_owned()),
            profiles: vec![work, profile("Scratch")],
            ..Settings::default()
        }
        .normalized();

        let text = toml::to_string_pretty(&settings).expect("profiles must serialize");
        assert!(
            text.contains("[[profiles]]") && text.contains("[profiles.env]"),
            "{text}"
        );
        assert_eq!(Settings::from_toml(&text), settings);
    }

    #[test]
    fn a_default_profile_that_does_not_exist_is_forgotten() {
        let settings = Settings {
            default_profile: Some("ghost".to_owned()),
            profiles: vec![profile("Work")],
            ..Settings::default()
        }
        .normalized();
        assert_eq!(settings.default_profile, None);
    }

    #[test]
    fn a_new_session_resolves_its_profile_against_the_settings() {
        let mut work = profile("Work");
        work.working_directory = Some("/srv/work".to_owned());
        work.env.insert("MODE".to_owned(), "work".to_owned());
        let settings = Settings {
            shell: Some("/bin/zsh".to_owned()),
            working_directory: Some("/home/me".to_owned()),
            default_profile: Some("work".to_owned()),
            profiles: vec![work, profile("Plain")],
            ..Settings::default()
        }
        .normalized();

        // The default profile applies when nothing is requested, and fills in what it leaves empty.
        let by_default = settings.launch_profile(None);
        assert_eq!(by_default.id, "work");
        assert_eq!(by_default.shell.as_deref(), Some("/bin/zsh"));
        assert_eq!(by_default.working_directory.as_deref(), Some("/srv/work"));
        assert_eq!(by_default.env.get("MODE").map(String::as_str), Some("work"));

        // A profile that sets nothing inherits everything from the built-in one.
        let plain = settings.launch_profile(Some("plain"));
        assert_eq!(plain.working_directory.as_deref(), Some("/home/me"));
        assert!(plain.env.is_empty());

        // An unknown id falls back to the default profile instead of failing the new tab.
        assert_eq!(settings.launch_profile(Some("ghost")).id, "work");
    }

    #[test]
    fn without_profiles_the_built_in_one_is_used() {
        let settings = Settings {
            shell: Some("/bin/fish".to_owned()),
            ..Settings::default()
        };
        let launch = settings.launch_profile(Some("anything"));
        assert_eq!(launch.id, "default");
        assert_eq!(launch.shell.as_deref(), Some("/bin/fish"));
    }
}
