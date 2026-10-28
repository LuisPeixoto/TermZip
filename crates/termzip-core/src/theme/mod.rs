//! Themes: the colors of the terminal and, derived from them, of the whole interface.
//!
//! A theme is a small TOML file (see `docs/themes.md`). The same model is used for the themes that
//! ship with TermZip, for the ones a user imports, and for the ones that are exported.

mod color;

use serde::{Deserialize, Serialize};

pub use color::Color;

use crate::profile::slug;

pub const MAX_THEME_BYTES: usize = 256 * 1024;
pub const MAX_NAME_CHARS: usize = 64;
const MAX_ID_CHARS: usize = 64;
const MAX_NOTE_CHARS: usize = 256;

/// Whether a theme is dark or light. It decides the interface chrome (`data-theme`), which theme the
/// toolbar toggle picks next, and is what `Settings.theme` stores.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeKind {
    #[default]
    Dark,
    Light,
}

impl ThemeKind {
    pub fn toggle(self) -> Self {
        match self {
            Self::Dark => Self::Light,
            Self::Light => Self::Dark,
        }
    }

    /// The appearance a background color belongs to, by its luminance.
    pub fn of_background(background: Color) -> Self {
        if background.luminance() < 0.4 {
            Self::Dark
        } else {
            Self::Light
        }
    }
}

/// What the terminal draws with.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Colors {
    pub background: Color,
    pub foreground: Color,
    pub cursor: Color,
    /// The character under a block cursor; the background when left out.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor_text: Option<Color>,
    /// May carry an alpha (`#rrggbbaa`) so the text stays readable under it.
    pub selection: Color,
    /// The interface accent; the ANSI blue when left out.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent: Option<Color>,
    /// The 16 ANSI colors, black to bright white.
    pub ansi: [Color; 16],
}

/// Interface colors a theme may set itself. Whatever is left out is derived from the terminal colors,
/// so a theme only needs these when the derived value is not what its author wants.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Chrome {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub panel: Option<Color>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub panel_2: Option<Color>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elevated: Option<Color>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hover: Option<Color>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<Color>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_strong: Option<Color>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub muted: Option<Color>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub faint: Option<Color>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accent_soft: Option<Color>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ok: Option<Color>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warn: Option<Color>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub err: Option<Color>,
}

impl Chrome {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Theme {
    /// Stable identifier (`a-z`, `0-9`, `-`), derived from the name when left out.
    #[serde(default)]
    pub id: String,
    pub name: String,
    pub appearance: ThemeKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    pub colors: Colors,
    #[serde(default, skip_serializing_if = "Chrome::is_empty")]
    pub chrome: Chrome,
}

fn note(value: Option<String>) -> Option<String> {
    value
        .map(|value| {
            value
                .trim()
                .chars()
                .take(MAX_NOTE_CHARS)
                .collect::<String>()
        })
        .filter(|value| !value.is_empty())
}

impl Theme {
    /// Reads a theme written in TermZip's format. The error is a sentence the interface can show.
    pub fn from_toml(text: &str) -> Result<Self, String> {
        if text.len() > MAX_THEME_BYTES {
            return Err(format!(
                "That theme file is larger than {} KiB.",
                MAX_THEME_BYTES / 1024
            ));
        }
        let theme: Self = toml::from_str(text)
            .map_err(|error| format!("That is not a valid TermZip theme: {}", error.message()))?;
        theme.normalized()
    }

    pub fn to_toml(&self) -> Result<String, String> {
        toml::to_string_pretty(self).map_err(|error| format!("Could not write the theme: {error}"))
    }

    /// Bounds every text field and makes sure the theme has a usable name and id.
    pub fn normalized(mut self) -> Result<Self, String> {
        self.name = self.name.trim().chars().take(MAX_NAME_CHARS).collect();
        if self.name.is_empty() {
            return Err("A theme needs a name.".to_owned());
        }
        let mut id: String = slug(&self.id).chars().take(MAX_ID_CHARS).collect();
        if id.is_empty() {
            id = slug(&self.name).chars().take(MAX_ID_CHARS).collect();
        }
        self.id = if id.is_empty() {
            "theme".to_owned()
        } else {
            id
        };
        self.author = note(self.author);
        self.license = note(self.license);
        self.source = note(self.source);
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::{Color, MAX_NAME_CHARS, MAX_THEME_BYTES, Theme, ThemeKind};

    const SAMPLE: &str = r##"
id = "midnight"
name = "Midnight"
appearance = "dark"
author = "Someone"
license = "MIT"
source = "https://example.com/midnight"

[colors]
background = "#0b0c10"
foreground = "#c5c6c7"
cursor = "#66fcf1"
selection = "#45a29e66"
ansi = [
  "#0b0c10", "#e06c75", "#98c379", "#e5c07b", "#61afef", "#c678dd", "#56b6c2", "#abb2bf",
  "#5c6370", "#e06c75", "#98c379", "#e5c07b", "#61afef", "#c678dd", "#56b6c2", "#ffffff",
]

[chrome]
panel = "#101116"
"##;

    #[test]
    fn theme_toggle_is_reversible() {
        assert_eq!(ThemeKind::Dark.toggle(), ThemeKind::Light);
        assert_eq!(ThemeKind::Light.toggle(), ThemeKind::Dark);
    }

    #[test]
    fn the_appearance_follows_the_background() {
        assert_eq!(
            ThemeKind::of_background(Color::rgb(0x1e, 0x1e, 0x2e)),
            ThemeKind::Dark
        );
        assert_eq!(
            ThemeKind::of_background(Color::rgb(0xfd, 0xf6, 0xe3)),
            ThemeKind::Light
        );
        // A mid-tone background is told apart by what reads better on it.
        assert_eq!(
            ThemeKind::of_background(Color::rgb(0x28, 0x2c, 0x34)),
            ThemeKind::Dark
        );
    }

    #[test]
    fn a_theme_file_is_read_with_its_optional_parts() {
        let theme = Theme::from_toml(SAMPLE).unwrap();

        assert_eq!(theme.id, "midnight");
        assert_eq!(theme.appearance, ThemeKind::Dark);
        assert_eq!(theme.license.as_deref(), Some("MIT"));
        assert_eq!(theme.colors.ansi[1], Color::rgb(0xe0, 0x6c, 0x75));
        assert_eq!(theme.colors.selection, Color::rgba(0x45, 0xa2, 0x9e, 0x66));
        assert_eq!(theme.colors.accent, None);
        assert_eq!(theme.colors.cursor_text, None);
        assert_eq!(theme.chrome.panel, Some(Color::rgb(0x10, 0x11, 0x16)));
        assert_eq!(theme.chrome.line, None);
    }

    #[test]
    fn a_theme_survives_a_round_trip_through_toml() {
        let theme = Theme::from_toml(SAMPLE).unwrap();
        let text = theme.to_toml().unwrap();

        assert!(
            !text.contains("accent"),
            "unset colors are not written: {text}"
        );
        assert_eq!(Theme::from_toml(&text).unwrap(), theme);
    }

    #[test]
    fn a_theme_without_chrome_writes_no_chrome_table() {
        let mut theme = Theme::from_toml(SAMPLE).unwrap();
        theme.chrome = Default::default();

        assert!(!theme.to_toml().unwrap().contains("[chrome]"));
    }

    #[test]
    fn the_id_comes_from_the_name_when_it_is_missing_or_unusable() {
        let without_id = SAMPLE.replace("id = \"midnight\"\n", "");
        assert_eq!(Theme::from_toml(&without_id).unwrap().id, "midnight");

        let shouty = SAMPLE.replace("id = \"midnight\"", "id = \"My Theme!!\"");
        assert_eq!(Theme::from_toml(&shouty).unwrap().id, "my-theme");

        let symbols_only = SAMPLE
            .replace("id = \"midnight\"", "id = \"!!!\"")
            .replace("name = \"Midnight\"", "name = \"???\"");
        assert_eq!(Theme::from_toml(&symbols_only).unwrap().id, "theme");
    }

    #[test]
    fn long_text_is_cut_and_a_blank_name_is_refused() {
        let long = SAMPLE.replace("Midnight", &"m".repeat(300));
        let theme = Theme::from_toml(&long).unwrap();
        assert_eq!(theme.name.chars().count(), MAX_NAME_CHARS);

        let blank = SAMPLE.replace("name = \"Midnight\"", "name = \"  \"");
        assert_eq!(
            Theme::from_toml(&blank).unwrap_err(),
            "A theme needs a name."
        );
    }

    #[test]
    fn mistakes_are_explained_in_a_sentence() {
        let bad_color = SAMPLE.replace("#0b0c10\", \"#e06c75", "nope\", \"#e06c75");
        let error = Theme::from_toml(&bad_color).unwrap_err();
        assert!(
            error.starts_with("That is not a valid TermZip theme"),
            "{error}"
        );

        let short = SAMPLE.replace("\"#ffffff\",\n]", "]");
        assert!(
            Theme::from_toml(&short).is_err(),
            "15 colors are not enough"
        );

        let missing = SAMPLE.replace("cursor = \"#66fcf1\"\n", "");
        assert!(Theme::from_toml(&missing).unwrap_err().contains("cursor"));

        assert!(Theme::from_toml("not toml = = at all").is_err());
        assert!(Theme::from_toml("").is_err());
    }

    #[test]
    fn a_file_over_the_limit_is_not_even_parsed() {
        let huge = format!("{SAMPLE}\n# {}", "x".repeat(MAX_THEME_BYTES));
        let error = Theme::from_toml(&huge).unwrap_err();
        assert!(error.contains("larger than"), "{error}");
    }
}
