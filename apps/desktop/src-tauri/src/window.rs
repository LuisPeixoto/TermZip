//! The translucent window: what shows through it is blurred by the system where that is possible.
//!
//! The window is always created transparent (a window cannot change that later); the page paints
//! itself opaque unless the user lowers the opacity, so nothing changes for anyone who does not.

use tauri::utils::config::WindowEffectsConfig;

/// Whether this system can blur what is behind a window: vibrancy on macOS, acrylic on Windows.
pub const BLUR_SUPPORTED: bool = cfg!(any(target_os = "macos", target_os = "windows"));

#[cfg(target_os = "macos")]
fn blur_effect() -> Option<WindowEffectsConfig> {
    use tauri::window::{Effect, EffectState, EffectsBuilder};

    Some(
        EffectsBuilder::new()
            .effect(Effect::UnderWindowBackground)
            .state(EffectState::FollowsWindowActiveState)
            .build(),
    )
}

#[cfg(target_os = "windows")]
fn blur_effect() -> Option<WindowEffectsConfig> {
    use tauri::window::{Effect, EffectsBuilder};

    Some(EffectsBuilder::new().effect(Effect::Acrylic).build())
}

/// Linux compositors decide for themselves; transparency works where they allow it, without blur.
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn blur_effect() -> Option<WindowEffectsConfig> {
    None
}

/// The effect to apply: the system's blur when asked for and available, nothing otherwise.
pub fn effects(blur: bool) -> Option<WindowEffectsConfig> {
    if blur { blur_effect() } else { None }
}

#[cfg(test)]
mod tests {
    use super::{BLUR_SUPPORTED, effects};

    #[test]
    fn blur_is_only_applied_when_asked_for_and_supported() {
        assert!(effects(false).is_none());
        assert_eq!(effects(true).is_some(), BLUR_SUPPORTED);
    }
}
