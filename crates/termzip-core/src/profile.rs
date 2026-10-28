//! Profiles: a named shell, working directory, and environment that a new tab can start with.

use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::appearance::CursorStyle;
use crate::settings::{MAX_FONT_SIZE, MIN_FONT_SIZE};

pub const MAX_PROFILES: usize = 32;
pub const MAX_NAME_CHARS: usize = 64;
pub const MAX_ENV_VARS: usize = 64;
pub const MAX_ENV_KEY_CHARS: usize = 128;
pub const MAX_ENV_VALUE_BYTES: usize = 4 * 1024;
const MAX_THEME_ID_CHARS: usize = 64;

/// The built-in profile: the top-level `shell` and `working_directory` of the settings.
pub const DEFAULT_PROFILE_ID: &str = "default";

/// Variables TermZip sets itself; a profile must not override what the integration depends on.
const RESERVED_ENV: [&str; 5] = [
    "TERM",
    "COLORTERM",
    "TERM_PROGRAM",
    "TERM_PROGRAM_VERSION",
    "ZDOTDIR",
];
const RESERVED_ENV_PREFIX: &str = "TERMZIP_";

/// Fields left empty inherit from the built-in profile, so a profile only says what differs.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Profile {
    /// Stable identifier (`a-z`, `0-9`, `-`). Derived from the name when empty.
    pub id: String,
    pub name: String,
    pub shell: Option<String>,
    pub working_directory: Option<String>,
    /// The id of the theme the terminal of this profile uses; the app's theme when empty.
    pub theme: Option<String>,
    pub font_size: Option<f32>,
    pub font_family: Option<String>,
    pub cursor_style: Option<CursorStyle>,
    /// Extra environment variables. Kept last: TOML writes tables after plain values.
    pub env: BTreeMap<String, String>,
}

/// What a session actually starts with, after the profile was resolved against the settings.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LaunchProfile {
    pub id: String,
    pub shell: Option<String>,
    pub working_directory: Option<String>,
    pub env: BTreeMap<String, String>,
}

pub fn is_reserved_env(key: &str) -> bool {
    let upper = key.to_ascii_uppercase();
    RESERVED_ENV.contains(&upper.as_str()) || upper.starts_with(RESERVED_ENV_PREFIX)
}

pub fn is_valid_env_key(key: &str) -> bool {
    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    key.len() <= MAX_ENV_KEY_CHARS
        && (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn is_valid_env_value(value: &str) -> bool {
    value.len() <= MAX_ENV_VALUE_BYTES && !value.contains('\0')
}

/// The first problem found, as a sentence the preferences dialog can show as it is.
pub fn validate_profiles(profiles: &[Profile]) -> Result<(), String> {
    if profiles.len() > MAX_PROFILES {
        return Err(format!("At most {MAX_PROFILES} profiles are supported."));
    }
    for profile in profiles {
        let name = profile.name.trim();
        if name.is_empty() {
            return Err("Every profile needs a name.".to_owned());
        }
        if name.chars().count() > MAX_NAME_CHARS {
            return Err(format!(
                "The profile name “{name}” is longer than {MAX_NAME_CHARS} characters."
            ));
        }
        if profile.id == DEFAULT_PROFILE_ID {
            return Err(format!(
                "The profile id “{DEFAULT_PROFILE_ID}” is reserved."
            ));
        }
        if profile.env.len() > MAX_ENV_VARS {
            return Err(format!(
                "The profile “{name}” has more than {MAX_ENV_VARS} environment variables."
            ));
        }
        for (key, value) in &profile.env {
            if !is_valid_env_key(key) {
                return Err(format!(
                    "“{key}” in the profile “{name}” is not a valid variable name (letters, digits and _, not starting with a digit)."
                ));
            }
            if is_reserved_env(key) {
                return Err(format!(
                    "The profile “{name}” cannot set {key}: TermZip manages it."
                ));
            }
            if !is_valid_env_value(value) {
                return Err(format!(
                    "The value of {key} in the profile “{name}” is too long or contains a NUL character."
                ));
            }
        }
    }
    Ok(())
}

/// `Work Laptop!` becomes `work-laptop`.
pub fn slug(text: &str) -> String {
    let mut out = String::new();
    for character in text.chars() {
        if character.is_ascii_alphanumeric() {
            out.push(character.to_ascii_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    out.trim_end_matches('-').to_owned()
}

fn blank_to_none(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

/// Cleans what the file or the dialog handed over: bounds every field, drops what cannot be used,
/// and gives every profile a unique id. It never fails; `validate_profiles` reports problems first.
pub(crate) fn normalize_profiles(profiles: Vec<Profile>) -> Vec<Profile> {
    let mut taken: HashSet<String> = HashSet::new();
    let mut out = Vec::new();
    for mut profile in profiles.into_iter().take(MAX_PROFILES) {
        let name: String = profile.name.trim().chars().take(MAX_NAME_CHARS).collect();
        profile.name = if name.is_empty() {
            "Profile".to_owned()
        } else {
            name
        };
        profile.shell = blank_to_none(profile.shell);
        profile.working_directory = blank_to_none(profile.working_directory);
        profile.theme = profile
            .theme
            .map(|theme| {
                slug(&theme)
                    .chars()
                    .take(MAX_THEME_ID_CHARS)
                    .collect::<String>()
            })
            .filter(|theme| !theme.is_empty());
        profile.font_family = blank_to_none(profile.font_family);
        profile.font_size = profile
            .font_size
            .filter(|size| size.is_finite())
            .map(|size| size.clamp(MIN_FONT_SIZE, MAX_FONT_SIZE));
        profile.env = std::mem::take(&mut profile.env)
            .into_iter()
            .filter(|(key, value)| {
                is_valid_env_key(key) && !is_reserved_env(key) && is_valid_env_value(value)
            })
            .take(MAX_ENV_VARS)
            .collect();

        let mut base = slug(&profile.id);
        if base.is_empty() || base == DEFAULT_PROFILE_ID {
            base = slug(&profile.name);
        }
        if base.is_empty() || base == DEFAULT_PROFILE_ID {
            base = "profile".to_owned();
        }
        let mut id = base.clone();
        let mut number = 2;
        while !taken.insert(id.clone()) {
            id = format!("{base}-{number}");
            number += 1;
        }
        profile.id = id;
        out.push(profile);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(name: &str) -> Profile {
        Profile {
            name: name.to_owned(),
            ..Profile::default()
        }
    }

    fn with_env(name: &str, key: &str, value: &str) -> Profile {
        let mut profile = profile(name);
        profile.env.insert(key.to_owned(), value.to_owned());
        profile
    }

    #[test]
    fn ids_come_from_the_name_and_stay_unique() {
        let cleaned = normalize_profiles(vec![
            profile("Work Laptop!"),
            profile("work laptop"),
            profile("Work   Laptop"),
        ]);
        let ids: Vec<_> = cleaned.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["work-laptop", "work-laptop-2", "work-laptop-3"]);
    }

    #[test]
    fn the_built_in_id_is_never_handed_out() {
        let mut shady = profile("Default");
        shady.id = "default".to_owned();
        let cleaned = normalize_profiles(vec![shady, profile("!!!")]);
        assert_eq!(cleaned[0].id, "profile");
        assert_eq!(cleaned[1].id, "profile-2");
        assert_ne!(cleaned[0].id, DEFAULT_PROFILE_ID);
    }

    #[test]
    fn fields_are_trimmed_bounded_and_filled_in() {
        let mut messy = profile("   ");
        messy.shell = Some("  ".to_owned());
        messy.working_directory = Some("  /srv  ".to_owned());
        let mut long = profile(&"n".repeat(200));
        long.id = "Custom ID".to_owned();

        let cleaned = normalize_profiles(vec![messy, long]);

        assert_eq!(cleaned[0].name, "Profile");
        assert_eq!(cleaned[0].shell, None);
        assert_eq!(cleaned[0].working_directory.as_deref(), Some("/srv"));
        assert_eq!(cleaned[1].name.chars().count(), MAX_NAME_CHARS);
        assert_eq!(cleaned[1].id, "custom-id");
    }

    #[test]
    fn appearance_overrides_are_cleaned_and_empty_ones_inherit() {
        let mut styled = profile("Styled");
        styled.theme = Some("  Tokyo Night!  ".to_owned());
        styled.font_size = Some(400.0);
        styled.font_family = Some("   ".to_owned());
        styled.cursor_style = Some(CursorStyle::Bar);
        let mut broken = profile("Broken");
        broken.theme = Some("???".to_owned());
        broken.font_size = Some(f32::NAN);

        let cleaned = normalize_profiles(vec![styled, broken, profile("Plain")]);

        assert_eq!(cleaned[0].theme.as_deref(), Some("tokyo-night"));
        assert_eq!(cleaned[0].font_size, Some(MAX_FONT_SIZE));
        assert_eq!(cleaned[0].font_family, None);
        assert_eq!(cleaned[0].cursor_style, Some(CursorStyle::Bar));
        assert_eq!(cleaned[1].theme, None);
        assert_eq!(cleaned[1].font_size, None);
        assert_eq!(cleaned[2].theme, None);
        assert_eq!(cleaned[2].cursor_style, None);
    }

    #[test]
    fn only_the_first_profiles_are_kept() {
        let many = (0..MAX_PROFILES + 10)
            .map(|n| profile(&format!("p{n}")))
            .collect();
        assert_eq!(normalize_profiles(many).len(), MAX_PROFILES);
    }

    #[test]
    fn unusable_variables_are_dropped_on_cleaning() {
        let mut messy = profile("Dev");
        for (key, value) in [
            ("GOOD", "1"),
            ("1BAD", "x"),
            ("with space", "x"),
            ("TERM", "dumb"),
            ("termzip_home", "/x"),
            ("ZDOTDIR", "/x"),
            ("NUL", "a\0b"),
        ] {
            messy.env.insert(key.to_owned(), value.to_owned());
        }
        let cleaned = normalize_profiles(vec![messy]);
        let keys: Vec<_> = cleaned[0].env.keys().map(String::as_str).collect();
        assert_eq!(keys, ["GOOD"]);
    }

    #[test]
    fn validation_explains_the_first_problem() {
        assert!(validate_profiles(&[profile("Ok")]).is_ok());
        assert!(
            validate_profiles(&[profile("  ")])
                .unwrap_err()
                .contains("name")
        );

        let bad_key = validate_profiles(&[with_env("Dev", "1A", "x")]).unwrap_err();
        assert!(
            bad_key.contains("1A") && bad_key.contains("variable name"),
            "{bad_key}"
        );

        let reserved = validate_profiles(&[with_env("Dev", "TERM_PROGRAM", "x")]).unwrap_err();
        assert!(reserved.contains("TermZip manages"), "{reserved}");
        assert!(validate_profiles(&[with_env("Dev", "termzip_home", "x")]).is_err());

        let long = "v".repeat(MAX_ENV_VALUE_BYTES + 1);
        assert!(validate_profiles(&[with_env("Dev", "BIG", &long)]).is_err());
        assert!(validate_profiles(&[with_env("Dev", "NUL", "a\0b")]).is_err());

        let mut reserved_id = profile("x");
        reserved_id.id = DEFAULT_PROFILE_ID.to_owned();
        assert!(validate_profiles(&[reserved_id]).is_err());

        let too_many: Vec<_> = (0..=MAX_PROFILES)
            .map(|n| profile(&format!("p{n}")))
            .collect();
        assert!(validate_profiles(&too_many).is_err());
    }

    #[test]
    fn variable_names_follow_the_shell_rules() {
        assert!(is_valid_env_key("PATH"));
        assert!(is_valid_env_key("_x1"));
        assert!(!is_valid_env_key(""));
        assert!(!is_valid_env_key("9x"));
        assert!(!is_valid_env_key("A-B"));
        assert!(!is_valid_env_key(&"A".repeat(MAX_ENV_KEY_CHARS + 1)));
        assert!(is_reserved_env("zdotdir"));
        assert!(!is_reserved_env("TERMINAL"));
    }
}
