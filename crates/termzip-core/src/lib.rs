mod settings;
mod theme;

pub use settings::{Settings, SettingsError, settings_path};
pub use theme::{Rgb, Theme, ThemeKind};
