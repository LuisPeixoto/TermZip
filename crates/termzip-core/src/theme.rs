use serde::{Deserialize, Serialize};

/// The two shipped interface themes. Colors live in the frontend design tokens so
/// the webview and the terminal renderer always share one palette.
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
}

#[cfg(test)]
mod tests {
    use super::ThemeKind;

    #[test]
    fn theme_toggle_is_reversible() {
        assert_eq!(ThemeKind::Dark.toggle(), ThemeKind::Light);
        assert_eq!(ThemeKind::Light.toggle(), ThemeKind::Dark);
    }
}
