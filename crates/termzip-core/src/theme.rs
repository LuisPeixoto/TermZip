use serde::{Deserialize, Serialize};

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

    pub fn palette(self) -> Theme {
        match self {
            Self::Dark => Theme::dark(),
            Self::Light => Theme::light(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    pub background: Rgb,
    pub surface: Rgb,
    pub elevated: Rgb,
    pub border: Rgb,
    pub foreground: Rgb,
    pub muted: Rgb,
    pub accent: Rgb,
    pub accent_soft: Rgb,
    pub success: Rgb,
    pub warning: Rgb,
    pub error: Rgb,
    pub ansi: [Rgb; 16],
}

impl Theme {
    pub const fn dark() -> Self {
        Self {
            background: Rgb::new(12, 14, 19),
            surface: Rgb::new(17, 20, 27),
            elevated: Rgb::new(25, 29, 38),
            border: Rgb::new(42, 47, 59),
            foreground: Rgb::new(226, 230, 239),
            muted: Rgb::new(139, 148, 166),
            accent: Rgb::new(157, 139, 255),
            accent_soft: Rgb::new(46, 41, 74),
            success: Rgb::new(111, 215, 174),
            warning: Rgb::new(242, 190, 111),
            error: Rgb::new(255, 112, 130),
            ansi: [
                Rgb::new(36, 41, 52),
                Rgb::new(255, 112, 130),
                Rgb::new(111, 215, 174),
                Rgb::new(242, 190, 111),
                Rgb::new(126, 169, 255),
                Rgb::new(193, 154, 255),
                Rgb::new(94, 218, 200),
                Rgb::new(202, 209, 223),
                Rgb::new(107, 115, 132),
                Rgb::new(255, 137, 151),
                Rgb::new(137, 231, 192),
                Rgb::new(255, 211, 132),
                Rgb::new(153, 188, 255),
                Rgb::new(210, 178, 255),
                Rgb::new(119, 232, 216),
                Rgb::new(245, 247, 251),
            ],
        }
    }

    pub const fn light() -> Self {
        Self {
            background: Rgb::new(245, 246, 249),
            surface: Rgb::new(255, 255, 255),
            elevated: Rgb::new(238, 240, 245),
            border: Rgb::new(222, 225, 233),
            foreground: Rgb::new(36, 40, 52),
            muted: Rgb::new(105, 113, 129),
            accent: Rgb::new(105, 82, 219),
            accent_soft: Rgb::new(237, 232, 255),
            success: Rgb::new(30, 137, 98),
            warning: Rgb::new(164, 103, 24),
            error: Rgb::new(198, 58, 76),
            ansi: [
                Rgb::new(36, 41, 52),
                Rgb::new(198, 58, 76),
                Rgb::new(30, 137, 98),
                Rgb::new(164, 103, 24),
                Rgb::new(49, 92, 184),
                Rgb::new(126, 67, 180),
                Rgb::new(0, 123, 132),
                Rgb::new(218, 221, 228),
                Rgb::new(105, 113, 129),
                Rgb::new(210, 72, 89),
                Rgb::new(35, 155, 112),
                Rgb::new(183, 121, 32),
                Rgb::new(63, 111, 211),
                Rgb::new(145, 91, 193),
                Rgb::new(0, 145, 153),
                Rgb::new(255, 255, 255),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Rgb, ThemeKind};

    #[test]
    fn themes_have_distinct_surfaces_and_readable_foregrounds() {
        let dark = ThemeKind::Dark.palette();
        let light = ThemeKind::Light.palette();

        assert_ne!(dark.background, light.background);
        assert_ne!(dark.foreground, light.foreground);
        assert_eq!(dark.ansi.len(), 16);
        assert_eq!(light.ansi[2], Rgb::new(30, 137, 98));
    }

    #[test]
    fn theme_toggle_is_reversible() {
        assert_eq!(ThemeKind::Dark.toggle(), ThemeKind::Light);
        assert_eq!(ThemeKind::Light.toggle(), ThemeKind::Dark);
    }
}
