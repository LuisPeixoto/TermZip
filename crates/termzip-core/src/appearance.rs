//! How the terminal looks besides its colors: the cursor, the spacing and the window.

use serde::{Deserialize, Serialize};

pub const DEFAULT_LINE_HEIGHT: f32 = 1.3;
pub const MIN_LINE_HEIGHT: f32 = 1.0;
pub const MAX_LINE_HEIGHT: f32 = 2.0;
pub const DEFAULT_LETTER_SPACING: f32 = 0.0;
pub const MIN_LETTER_SPACING: f32 = 0.0;
pub const MAX_LETTER_SPACING: f32 = 4.0;
pub const MIN_BACKGROUND_OPACITY: f32 = 0.5;
pub const MAX_BACKGROUND_OPACITY: f32 = 1.0;

/// The shape of the terminal cursor.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CursorStyle {
    #[default]
    Block,
    Bar,
    Underline,
}

/// Clamps `value` into `min..=max`, and a value that is not a number becomes `default`.
pub(crate) fn bounded(value: f32, min: f32, max: f32, default: f32) -> f32 {
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        default
    }
}

#[cfg(test)]
mod tests {
    use super::bounded;

    #[test]
    fn values_are_clamped_and_nonsense_gets_the_default() {
        assert_eq!(bounded(5.0, 1.0, 2.0, 1.3), 2.0);
        assert_eq!(bounded(-5.0, 1.0, 2.0, 1.3), 1.0);
        assert_eq!(bounded(1.5, 1.0, 2.0, 1.3), 1.5);
        assert_eq!(bounded(f32::NAN, 1.0, 2.0, 1.3), 1.3);
        assert_eq!(bounded(f32::INFINITY, 1.0, 2.0, 1.3), 1.3);
    }
}
