//! A color as it appears in theme files: `#rrggbb`, with an optional alpha as `#rrggbbaa`.

use std::fmt;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// The same color without transparency.
    pub const fn opaque(self) -> Self {
        Self { a: 255, ..self }
    }

    /// Reads `#rgb`, `#rrggbb` or `#rrggbbaa`; the `#` may be left out (Ghostty writes bare hex).
    pub fn parse(text: &str) -> Option<Self> {
        let digits = text.trim().strip_prefix('#').unwrap_or(text.trim());
        if !digits.is_ascii() || !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return None;
        }
        let pair = |at: usize| u8::from_str_radix(&digits[at..at + 2], 16).ok();
        let nibble = |at: usize| {
            u8::from_str_radix(&digits[at..at + 1], 16)
                .ok()
                .map(|value| value * 17)
        };
        match digits.len() {
            3 => Some(Self::rgb(nibble(0)?, nibble(1)?, nibble(2)?)),
            6 => Some(Self::rgb(pair(0)?, pair(2)?, pair(4)?)),
            8 => Some(Self::rgba(pair(0)?, pair(2)?, pair(4)?, pair(6)?)),
            _ => None,
        }
    }

    /// `#rrggbb`, or `#rrggbbaa` when the color is not opaque.
    pub fn to_hex(self) -> String {
        if self.a == 255 {
            format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            format!("#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, self.a)
        }
    }

    /// Relative luminance as defined by WCAG 2.x; the alpha channel is ignored.
    pub fn luminance(self) -> f64 {
        let channel = |value: u8| {
            let value = f64::from(value) / 255.0;
            if value <= 0.039_28 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(self.r) + 0.7152 * channel(self.g) + 0.0722 * channel(self.b)
    }

    /// WCAG contrast ratio, from 1 (identical) to 21 (black on white).
    pub fn contrast(self, other: Self) -> f64 {
        let (a, b) = (self.luminance(), other.luminance());
        let (light, dark) = if a >= b { (a, b) } else { (b, a) };
        (light + 0.05) / (dark + 0.05)
    }
}

impl fmt::Display for Color {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_hex())
    }
}

impl Serialize for Color {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct HexVisitor;

        impl Visitor<'_> for HexVisitor {
            type Value = Color;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a color such as \"#1e1e2e\"")
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<Color, E> {
                Color::parse(value)
                    .ok_or_else(|| E::custom(format!("“{value}” is not a color such as #1e1e2e")))
            }
        }

        deserializer.deserialize_str(HexVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::Color;

    #[test]
    fn colors_are_read_in_every_spelling_a_theme_file_uses() {
        assert_eq!(Color::parse("#1e1e2e"), Some(Color::rgb(0x1e, 0x1e, 0x2e)));
        assert_eq!(Color::parse("1E1E2E"), Some(Color::rgb(0x1e, 0x1e, 0x2e)));
        assert_eq!(Color::parse("#fa0"), Some(Color::rgb(0xff, 0xaa, 0x00)));
        assert_eq!(
            Color::parse("  #2563eb1f "),
            Some(Color::rgba(0x25, 0x63, 0xeb, 0x1f))
        );
    }

    #[test]
    fn anything_else_is_refused() {
        for bad in [
            "",
            "#",
            "#12",
            "#12345",
            "#1234567",
            "#gg0000",
            "red",
            "#ff 00 00",
            "#ééé",
            "0x112233",
        ] {
            assert_eq!(Color::parse(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn hex_round_trips_and_keeps_alpha_only_when_needed() {
        assert_eq!(Color::rgb(1, 2, 3).to_hex(), "#010203");
        assert_eq!(Color::rgba(1, 2, 3, 128).to_hex(), "#01020380");
        assert_eq!(
            Color::parse(&Color::rgba(9, 8, 7, 6).to_hex()),
            Some(Color::rgba(9, 8, 7, 6))
        );
    }

    #[test]
    fn contrast_follows_wcag() {
        let black = Color::rgb(0, 0, 0);
        let white = Color::rgb(255, 255, 255);
        assert!((black.contrast(white) - 21.0).abs() < 1e-9);
        assert!((white.contrast(black) - 21.0).abs() < 1e-9);
        assert!((white.contrast(white) - 1.0).abs() < 1e-9);
        // #777 on white is the textbook case that falls just short of AA (4.5:1) for body text.
        let grey = Color::rgb(0x77, 0x77, 0x77);
        assert!((grey.contrast(white) - 4.48).abs() < 0.01);
    }
}
