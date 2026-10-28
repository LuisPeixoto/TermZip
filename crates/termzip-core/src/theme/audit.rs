//! The checks a theme has to pass to be listed in the gallery.
//!
//! They are about legibility, not taste: the text, the cursor and the colored output must be readable
//! on the background. Imported themes are not held to them, but the same function can describe their
//! weak spots.

use super::{Color, Theme, ThemeKind};

/// WCAG AA for body text.
pub const MIN_TEXT_CONTRAST: f64 = 4.5;
/// WCAG for user-interface components, which is what a cursor is.
pub const MIN_CURSOR_CONTRAST: f64 = 3.0;
/// Colored output (red errors, green passes, ...) must stay distinguishable from the background. Light
/// themes cannot reach 3:1 with a yellow that still looks yellow, so the bar is lower than for text.
pub const MIN_COLOR_CONTRAST: f64 = 2.0;
/// Bright black is where shells draw suggestions and comments; it may be dim but never invisible.
pub const MIN_DIM_CONTRAST: f64 = 1.5;

const NAMES: [&str; 8] = [
    "black", "red", "green", "yellow", "blue", "magenta", "cyan", "white",
];

fn ansi_name(index: usize) -> String {
    let name = NAMES[index % 8];
    if index < 8 {
        name.to_owned()
    } else {
        format!("bright {name}")
    }
}

impl Theme {
    /// What keeps this theme out of the gallery, as sentences. Empty means it passes.
    pub fn audit(&self) -> Vec<String> {
        let colors = &self.colors;
        let background = colors.background;
        let ratio = |color: Color| color.contrast(background);
        let mut problems = Vec::new();

        if ThemeKind::of_background(background) != self.appearance {
            problems.push("the appearance does not match the background".to_owned());
        }
        if ratio(colors.foreground) < MIN_TEXT_CONTRAST {
            problems.push(format!(
                "the text is {:.1}:1 on the background (needs {MIN_TEXT_CONTRAST}:1)",
                ratio(colors.foreground)
            ));
        }
        if ratio(colors.cursor) < MIN_CURSOR_CONTRAST {
            problems.push(format!(
                "the cursor is {:.1}:1 on the background (needs {MIN_CURSOR_CONTRAST}:1)",
                ratio(colors.cursor)
            ));
        }
        for index in (1..=6).chain(9..=14) {
            let found = ratio(colors.ansi[index]);
            if found < MIN_COLOR_CONTRAST {
                problems.push(format!(
                    "{} is {found:.1}:1 on the background (needs {MIN_COLOR_CONTRAST}:1)",
                    ansi_name(index)
                ));
            }
        }
        let dim = ratio(colors.ansi[8]);
        if dim < MIN_DIM_CONTRAST {
            problems.push(format!(
                "bright black is {dim:.1}:1 on the background, so suggestions and comments vanish (needs {MIN_DIM_CONTRAST}:1)"
            ));
        }
        problems
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Solarized Dark, as its authors published it: bright black is the background color.
    const SOLARIZED_DARK: &str = r##"
name = "Solarized Dark"
appearance = "dark"
[colors]
background = "#002b36"
foreground = "#839496"
cursor = "#93a1a1"
selection = "#073642"
ansi = [
  "#073642", "#dc322f", "#859900", "#b58900", "#268bd2", "#d33682", "#2aa198", "#eee8d5",
  "#002b36", "#cb4b16", "#586e75", "#657b83", "#839496", "#6c71c4", "#93a1a1", "#fdf6e3",
]
"##;

    #[test]
    fn a_palette_whose_bright_black_is_the_background_is_caught() {
        let theme = Theme::from_toml(SOLARIZED_DARK).unwrap();
        let problems = theme.audit();

        assert!(
            problems.iter().any(|p| p.starts_with("bright black")),
            "{problems:?}"
        );
    }

    #[test]
    fn low_contrast_text_and_colors_are_named() {
        let mut theme = Theme::from_toml(SOLARIZED_DARK).unwrap();
        theme.colors.ansi[8] = Color::rgb(0x58, 0x6e, 0x75);
        theme.colors.foreground = Color::rgb(0x58, 0x6e, 0x75);
        theme.colors.ansi[3] = Color::rgb(0x01, 0x30, 0x3b);
        let problems = theme.audit();

        assert!(
            problems.iter().any(|p| p.starts_with("the text")),
            "{problems:?}"
        );
        assert!(
            problems.iter().any(|p| p.starts_with("yellow")),
            "{problems:?}"
        );
    }

    #[test]
    fn a_light_background_called_dark_is_caught() {
        let mut theme = Theme::from_toml(SOLARIZED_DARK).unwrap();
        theme.appearance = ThemeKind::Light;

        assert!(
            theme
                .audit()
                .iter()
                .any(|p| p.contains("appearance does not match"))
        );
    }
}
