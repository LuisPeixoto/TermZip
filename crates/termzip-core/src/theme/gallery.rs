//! The themes that ship with TermZip.
//!
//! Every file in `themes/` is audited by the tests below: a palette that is hard to read is left out
//! of the gallery instead of being quietly adjusted, because the colors are their authors'. Solarized
//! is the known example (its bright black is the background color).

use super::Theme;

pub const DEFAULT_DARK_ID: &str = "termzip-dark";
pub const DEFAULT_LIGHT_ID: &str = "termzip-light";

const FILES: [&str; 12] = [
    include_str!("../../themes/termzip-dark.toml"),
    include_str!("../../themes/termzip-light.toml"),
    include_str!("../../themes/catppuccin-mocha.toml"),
    include_str!("../../themes/catppuccin-latte.toml"),
    include_str!("../../themes/dracula.toml"),
    include_str!("../../themes/nord.toml"),
    include_str!("../../themes/gruvbox-dark.toml"),
    include_str!("../../themes/gruvbox-light.toml"),
    include_str!("../../themes/tokyo-night.toml"),
    include_str!("../../themes/one-dark.toml"),
    include_str!("../../themes/rose-pine.toml"),
    include_str!("../../themes/rose-pine-dawn.toml"),
];

/// The built-in themes, TermZip's own first. A file that cannot be read is skipped (the tests make
/// sure there is none), so the gallery can never be the reason the app does not start.
pub fn built_in_themes() -> Vec<Theme> {
    FILES
        .iter()
        .filter_map(|text| Theme::from_toml(text).ok())
        .collect()
}

pub fn built_in_theme(id: &str) -> Option<Theme> {
    built_in_themes().into_iter().find(|theme| theme.id == id)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::ThemeKind;

    #[test]
    fn every_built_in_theme_can_be_read() {
        for (position, text) in FILES.iter().enumerate() {
            if let Err(error) = Theme::from_toml(text) {
                panic!("built-in theme {position} cannot be read: {error}");
            }
        }
        assert_eq!(built_in_themes().len(), FILES.len());
    }

    #[test]
    fn ids_are_unique_and_the_defaults_exist_for_both_appearances() {
        let themes = built_in_themes();
        let ids: HashSet<_> = themes.iter().map(|theme| theme.id.as_str()).collect();
        assert_eq!(ids.len(), themes.len(), "duplicate theme id");

        let dark = built_in_theme(DEFAULT_DARK_ID).unwrap();
        let light = built_in_theme(DEFAULT_LIGHT_ID).unwrap();
        assert_eq!(dark.appearance, ThemeKind::Dark);
        assert_eq!(light.appearance, ThemeKind::Light);
    }

    #[test]
    fn the_gallery_offers_both_appearances() {
        let themes = built_in_themes();
        for kind in [ThemeKind::Dark, ThemeKind::Light] {
            let count = themes
                .iter()
                .filter(|theme| theme.appearance == kind)
                .count();
            assert!(count >= 4, "{kind:?} has only {count} themes");
        }
    }

    #[test]
    fn every_theme_credits_its_authors_with_a_license_and_a_source() {
        for theme in built_in_themes() {
            let credited = |value: &Option<String>| value.as_deref().is_some_and(|v| !v.is_empty());
            assert!(credited(&theme.author), "{} has no author", theme.id);
            assert!(credited(&theme.license), "{} has no license", theme.id);
            assert!(
                theme
                    .source
                    .as_deref()
                    .is_some_and(|s| s.starts_with("https://")),
                "{} has no source link",
                theme.id
            );
        }
    }

    #[test]
    fn every_theme_is_legible() {
        for theme in built_in_themes() {
            let problems = theme.audit();
            assert!(problems.is_empty(), "{}: {problems:?}", theme.id);
        }
    }

    #[test]
    fn the_two_termzip_themes_have_no_chrome_left_to_guess() {
        for id in [DEFAULT_DARK_ID, DEFAULT_LIGHT_ID] {
            let chrome = built_in_theme(id).unwrap().chrome;
            assert!(
                chrome.panel.is_some()
                    && chrome.panel_2.is_some()
                    && chrome.elevated.is_some()
                    && chrome.hover.is_some()
                    && chrome.line.is_some()
                    && chrome.line_strong.is_some()
                    && chrome.muted.is_some()
                    && chrome.faint.is_some()
                    && chrome.accent_soft.is_some(),
                "{id} must reproduce the interface exactly"
            );
        }
    }
}
