//! Themes written for other terminals, read into TermZip's model.
//!
//! The file is user-chosen input, so every reader is bounded (the caller's size limit, no recursion
//! of our own) and answers with a sentence instead of a panic. Anything a format leaves out comes from
//! the TermZip palette of the same appearance, so an imported theme is always complete.

use std::array;
use std::io::Cursor;

use plist::{Dictionary, Value};

use super::{
    Color, Colors, DEFAULT_DARK_ID, DEFAULT_LIGHT_ID, MAX_THEME_BYTES, Theme, ThemeKind,
    built_in_theme,
};

const UNRECOGNISED: &str = "TermZip could not recognise that file. It reads TermZip themes (.toml), iTerm2 (.itermcolors), Windows Terminal (.json), Ghostty, Kitty and Alacritty (.toml).";

/// What a format told us. Everything is optional until `finish` fills the gaps.
#[derive(Default)]
struct Draft {
    name: Option<String>,
    background: Option<Color>,
    foreground: Option<Color>,
    cursor: Option<Color>,
    cursor_text: Option<Color>,
    selection: Option<Color>,
    ansi: [Option<Color>; 16],
}

impl Draft {
    fn is_empty(&self) -> bool {
        self.background.is_none()
            && self.foreground.is_none()
            && self.cursor.is_none()
            && self.selection.is_none()
            && self.ansi.iter().all(Option::is_none)
    }

    fn finish(self, fallback_name: &str, format: &str) -> Result<Theme, String> {
        let background = self
            .background
            .ok_or_else(|| format!("That {format} theme does not set a background color."))?
            .opaque();
        let foreground = self
            .foreground
            .ok_or_else(|| format!("That {format} theme does not set a foreground color."))?
            .opaque();
        let appearance = ThemeKind::of_background(background);
        let base = built_in_theme(match appearance {
            ThemeKind::Dark => DEFAULT_DARK_ID,
            ThemeKind::Light => DEFAULT_LIGHT_ID,
        })
        .ok_or("TermZip's own theme is missing, so the import cannot fill in the colors.")?;

        let ansi = array::from_fn(|index| {
            self.ansi[index]
                .map(Color::opaque)
                .unwrap_or(base.colors.ansi[index])
        });
        Theme {
            id: String::new(),
            name: self.name.unwrap_or_else(|| fallback_name.to_owned()),
            appearance,
            author: None,
            license: None,
            source: None,
            colors: Colors {
                background,
                foreground,
                cursor: self.cursor.map_or(foreground, Color::opaque),
                cursor_text: self.cursor_text.map(Color::opaque),
                // The theme's own selection color, or a faint wash of the text color.
                selection: self.selection.unwrap_or(Color::rgba(
                    foreground.r,
                    foreground.g,
                    foreground.b,
                    0x40,
                )),
                accent: None,
                ansi,
            },
            chrome: Default::default(),
        }
        .normalized()
    }
}

/// `catppuccin-mocha_kitty.conf` becomes `catppuccin mocha kitty`.
fn name_from_file(file_name: &str) -> String {
    let base = file_name.rsplit(['/', '\\']).next().unwrap_or(file_name);
    let stem = match base.rsplit_once('.') {
        Some((stem, _)) if !stem.is_empty() => stem,
        _ => base,
    };
    let spaced: String = stem
        .chars()
        .map(|c| if c == '-' || c == '_' { ' ' } else { c })
        .collect();
    let spaced = spaced.trim();
    if spaced.is_empty() {
        "Imported theme".to_owned()
    } else {
        spaced.to_owned()
    }
}

/// A color in one of the spellings the formats use. `strict` colors must be valid; the optional
/// ones (cursor, selection) may be a word such as `CellForeground` or `none`, which is ignored.
fn color_value(what: &str, value: &str, strict: bool) -> Result<Option<Color>, String> {
    let cleaned = value.trim().trim_matches('"').trim();
    let digits = cleaned.strip_prefix("0x").unwrap_or(cleaned);
    match Color::parse(digits) {
        Some(color) => Ok(Some(color)),
        None if strict => Err(format!(
            "“{value}” is not a color for {what}; expected something like #1e1e2e."
        )),
        None => Ok(None),
    }
}

/// Reads a theme file of any supported format. `file_name` names the theme when the file does not.
pub fn import_theme(file_name: &str, bytes: &[u8]) -> Result<Theme, String> {
    if bytes.len() > MAX_THEME_BYTES {
        return Err(format!(
            "That theme file is larger than {} KiB.",
            MAX_THEME_BYTES / 1024
        ));
    }
    let fallback = name_from_file(file_name);
    if bytes.starts_with(b"bplist") {
        return from_plist(bytes, &fallback);
    }
    let text = std::str::from_utf8(bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes))
        .map_err(|_| "That file is not text. Theme files are UTF-8 text.".to_owned())?;

    let head = text.trim_start();
    if head.starts_with("<?xml") || head.starts_with("<plist") || head.starts_with("<!DOCTYPE") {
        return from_plist(text.as_bytes(), &fallback);
    }
    // `[` alone starts a TOML table; a JSON list of schemes has an object right after it.
    if head.starts_with('{')
        || head
            .strip_prefix('[')
            .is_some_and(|rest| rest.trim_start().starts_with('{'))
    {
        return from_windows_terminal(text, &fallback);
    }
    if let Ok(table) = text.parse::<toml::Table>()
        && let Some(colors) = table.get("colors").and_then(toml::Value::as_table)
    {
        if colors.contains_key("ansi") {
            return Theme::from_toml(text);
        }
        if ["primary", "normal", "bright", "cursor", "selection"]
            .iter()
            .any(|section| colors.contains_key(*section))
        {
            return from_alacritty(colors, &fallback);
        }
    }
    from_lines(text, &fallback)
}

// --- iTerm2 -------------------------------------------------------------------------------------

fn number(value: &Value) -> Option<f64> {
    value
        .as_real()
        .or_else(|| value.as_signed_integer().map(|n| n as f64))
        .or_else(|| value.as_unsigned_integer().map(|n| n as f64))
}

fn unit_to_byte(key: &str, component: &str, value: f64) -> Result<u8, String> {
    if !value.is_finite() {
        return Err(format!(
            "“{key}” has a {component} component that is not a number."
        ));
    }
    Ok((value.clamp(0.0, 1.0) * 255.0).round() as u8)
}

fn iterm_color(dict: &Dictionary, key: &str) -> Result<Option<Color>, String> {
    let Some(entry) = dict.get(key) else {
        return Ok(None);
    };
    let parts = entry
        .as_dictionary()
        .ok_or_else(|| format!("“{key}” is not a color."))?;
    let component = |name: &str| -> Result<f64, String> {
        parts
            .get(&format!("{name} Component"))
            .and_then(number)
            .ok_or_else(|| format!("“{key}” is missing its {} component.", name.to_lowercase()))
    };
    let red = unit_to_byte(key, "red", component("Red")?)?;
    let green = unit_to_byte(key, "green", component("Green")?)?;
    let blue = unit_to_byte(key, "blue", component("Blue")?)?;
    let alpha = match parts.get("Alpha Component").and_then(number) {
        Some(value) => unit_to_byte(key, "alpha", value)?,
        None => 255,
    };
    Ok(Some(Color::rgba(red, green, blue, alpha)))
}

fn from_plist(bytes: &[u8], fallback: &str) -> Result<Theme, String> {
    let value = Value::from_reader(Cursor::new(bytes))
        .map_err(|_| "That is not a readable iTerm2 color scheme.".to_owned())?;
    let dict = value
        .as_dictionary()
        .ok_or("That property list is not an iTerm2 color scheme.")?;

    let mut draft = Draft {
        background: iterm_color(dict, "Background Color")?,
        foreground: iterm_color(dict, "Foreground Color")?,
        cursor: iterm_color(dict, "Cursor Color")?,
        cursor_text: iterm_color(dict, "Cursor Text Color")?,
        selection: iterm_color(dict, "Selection Color")?,
        ..Draft::default()
    };
    for (index, slot) in draft.ansi.iter_mut().enumerate() {
        *slot = iterm_color(dict, &format!("Ansi {index} Color"))?;
    }
    if draft.is_empty() {
        return Err("That property list has no iTerm2 colors in it.".to_owned());
    }
    draft.finish(fallback, "iTerm2")
}

// --- Windows Terminal ---------------------------------------------------------------------------

const WINDOWS_TERMINAL_ANSI: [&str; 16] = [
    "black",
    "red",
    "green",
    "yellow",
    "blue",
    "purple",
    "cyan",
    "white",
    "brightBlack",
    "brightRed",
    "brightGreen",
    "brightYellow",
    "brightBlue",
    "brightPurple",
    "brightCyan",
    "brightWhite",
];

/// The scheme to read: the file itself, or one of the `schemes` of a whole `settings.json`, the one
/// named like the file when there is one.
fn pick_scheme<'a>(
    json: &'a serde_json::Value,
    wanted: &str,
) -> Result<&'a serde_json::Map<String, serde_json::Value>, String> {
    let list = match json {
        serde_json::Value::Array(list) => Some(list),
        serde_json::Value::Object(map) => map.get("schemes").and_then(serde_json::Value::as_array),
        _ => None,
    };
    if let Some(list) = list {
        let schemes: Vec<_> = list
            .iter()
            .filter_map(serde_json::Value::as_object)
            .collect();
        let named = schemes.iter().find(|scheme| {
            scheme
                .get("name")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|name| name.eq_ignore_ascii_case(wanted))
        });
        return named
            .or(schemes.first())
            .copied()
            .ok_or_else(|| "That file has no color schemes in it.".to_owned());
    }
    json.as_object()
        .ok_or_else(|| "That JSON is not a Windows Terminal color scheme.".to_owned())
}

fn from_windows_terminal(text: &str, fallback: &str) -> Result<Theme, String> {
    let json: serde_json::Value = serde_json::from_str(text).map_err(|_| {
        "That is not plain JSON. A Windows Terminal settings file with comments must have them removed first."
            .to_owned()
    })?;
    let scheme = pick_scheme(&json, fallback)?;
    let read = |key: &str, strict: bool| -> Result<Option<Color>, String> {
        match scheme.get(key) {
            None => Ok(None),
            Some(serde_json::Value::String(value)) => color_value(key, value, strict),
            Some(_) => Err(format!("“{key}” should be a color such as #1e1e2e.")),
        }
    };

    let mut draft = Draft {
        name: scheme
            .get("name")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
        background: read("background", true)?,
        foreground: read("foreground", true)?,
        cursor: read("cursorColor", false)?,
        selection: read("selectionBackground", false)?,
        ..Draft::default()
    };
    for (slot, key) in draft.ansi.iter_mut().zip(WINDOWS_TERMINAL_ANSI) {
        *slot = read(key, true)?;
    }
    if draft.is_empty() {
        return Err("That JSON does not look like a Windows Terminal color scheme.".to_owned());
    }
    draft.finish(fallback, "Windows Terminal")
}

// --- Alacritty ----------------------------------------------------------------------------------

const ALACRITTY_ANSI: [&str; 8] = [
    "black", "red", "green", "yellow", "blue", "magenta", "cyan", "white",
];

fn from_alacritty(colors: &toml::Table, fallback: &str) -> Result<Theme, String> {
    let read = |section: &str, key: &str, strict: bool| -> Result<Option<Color>, String> {
        let value = colors
            .get(section)
            .and_then(toml::Value::as_table)
            .and_then(|table| table.get(key));
        match value {
            None => Ok(None),
            Some(toml::Value::String(value)) => {
                color_value(&format!("colors.{section}.{key}"), value, strict)
            }
            Some(_) => Err(format!(
                "colors.{section}.{key} should be a color such as \"#1e1e2e\"."
            )),
        }
    };

    let mut draft = Draft {
        background: read("primary", "background", true)?,
        foreground: read("primary", "foreground", true)?,
        cursor: read("cursor", "cursor", false)?,
        cursor_text: read("cursor", "text", false)?,
        selection: read("selection", "background", false)?,
        ..Draft::default()
    };
    for (index, key) in ALACRITTY_ANSI.iter().enumerate() {
        draft.ansi[index] = read("normal", key, true)?;
        draft.ansi[index + 8] = read("bright", key, true)?;
    }
    if draft.is_empty() {
        return Err("That Alacritty file does not set any colors.".to_owned());
    }
    draft.finish(fallback, "Alacritty")
}

// --- Ghostty and Kitty --------------------------------------------------------------------------

/// Both are one setting per line: Ghostty writes `key = value` (with `palette = 3=#f9e2af`), Kitty
/// writes `key value` (with `color3 #f9e2af`), so one reader serves both.
fn from_lines(text: &str, fallback: &str) -> Result<Theme, String> {
    let mut draft = Draft::default();
    let mut recognised = false;

    for (number, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let split = line
            .split_once('=')
            .or_else(|| line.split_once(char::is_whitespace));
        let Some((key, value)) = split else { continue };
        let key = key.trim().to_ascii_lowercase().replace('-', "_");
        let value = value.trim();
        let at = |error: String| format!("Line {}: {error}", number + 1);

        let mut ansi_slot = |index: usize, value: &str| -> Result<(), String> {
            // Entries past the 16th are the 256-color extension, which TermZip does not theme.
            if index < 16 {
                draft.ansi[index] = color_value(&format!("color {index}"), value, true)?;
            }
            Ok(())
        };

        match key.as_str() {
            "background" => {
                draft.background = color_value("the background", value, true).map_err(at)?
            }
            "foreground" => {
                draft.foreground = color_value("the foreground", value, true).map_err(at)?
            }
            "cursor" | "cursor_color" => {
                draft.cursor = color_value("the cursor", value, false).map_err(at)?
            }
            "cursor_text" | "cursor_text_color" => {
                draft.cursor_text = color_value("the cursor text", value, false).map_err(at)?;
            }
            "selection_background" => {
                draft.selection = color_value("the selection", value, false).map_err(at)?;
            }
            "palette" => {
                let Some((index, color)) = value.split_once('=') else {
                    return Err(at(format!("“{value}” should look like 3=#f9e2af.")));
                };
                let Ok(index) = index.trim().parse::<usize>() else {
                    return Err(at(format!("“{}” is not a palette position.", index.trim())));
                };
                ansi_slot(index, color).map_err(at)?;
            }
            other => match other.strip_prefix("color").map(str::parse::<usize>) {
                Some(Ok(index)) => ansi_slot(index, value).map_err(at)?,
                _ => continue,
            },
        }
        recognised = true;
    }

    if !recognised || draft.is_empty() {
        return Err(UNRECOGNISED.to_owned());
    }
    draft.finish(fallback, "Ghostty or Kitty")
}
