//! Updates, only when asked.
//!
//! TermZip never looks for an update by itself. It happens when the person presses **Check for updates** in
//! Preferences, or at startup if they turned that on (it is off by default). Nothing is installed without a second
//! click on **Install and restart**.
//!
//! The network use is here, in Rust, and not in the page: the webview has no updater permission and a CSP that
//! forbids it. The update is a file downloaded over HTTPS and **checked against the public key built into the app**
//! (minisign) before anything is replaced; a file that was changed, or signed by anyone else, is refused. The address
//! it asks is fixed in the configuration (the latest release of this project on GitHub).

use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_updater::{Update, UpdaterExt};

/// In a debug build only, the address of a `latest.json` to ask instead (to try updates against a local server).
#[cfg(debug_assertions)]
pub const ENDPOINT_ENV: &str = "TERMZIP_UPDATE_ENDPOINT";
/// The release notes shown are cut at this many characters, and shown as plain text.
const MAX_NOTES_CHARS: usize = 2000;

/// The update found by the last check, waiting for the person to install it.
#[derive(Default)]
pub struct UpdateState(Mutex<Option<Update>>);

/// What the page shows about an update that is available.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    pub current_version: String,
    pub notes: Option<String>,
    /// The date the release says, as `YYYY-MM-DD`.
    pub date: Option<String>,
}

/// Release notes as safe plain text: no control characters other than line breaks, cut to a limit. They come from a
/// file on the network, so they are never shown as markup.
pub fn clean_notes(notes: &str) -> Option<String> {
    let text: String = notes
        .chars()
        .filter(|c| *c == '\n' || !c.is_control())
        .filter(|c| !termzip_core::hides_or_reorders(*c))
        .take(MAX_NOTES_CHARS)
        .collect();
    let text = text.trim().to_owned();
    (!text.is_empty()).then_some(text)
}

fn info_of(update: &Update) -> UpdateInfo {
    UpdateInfo {
        version: update.version.clone(),
        current_version: update.current_version.clone(),
        notes: update.body.as_deref().and_then(clean_notes),
        date: update.date.map(|date| date.date().to_string()),
    }
}

/// A sentence for what the updater said, in terms a person can act on.
fn explain_text(text: &str) -> String {
    let lower = text.to_ascii_lowercase();
    if lower.contains("signature") || lower.contains("minisign") {
        "The update did not pass its signature check, so it was not installed. Nothing on your computer was changed."
            .to_owned()
    } else if lower.contains("valid release json") {
        "The update server answered, but with no usable information about a release. Try again later.".to_owned()
    } else if lower.contains("error sending request")
        || lower.contains("dns")
        || lower.contains("connect")
        || lower.contains("timed out")
    {
        "Could not reach the update server. Check your connection and try again.".to_owned()
    } else {
        format!("Could not check for updates: {text}")
    }
}

fn explain(error: &tauri_plugin_updater::Error) -> String {
    explain_text(&error.to_string())
}

fn updater(app: &AppHandle) -> Result<tauri_plugin_updater::Updater, String> {
    #[cfg(debug_assertions)]
    if let Ok(endpoint) = std::env::var(ENDPOINT_ENV)
        && !endpoint.is_empty()
    {
        let url = endpoint
            .parse()
            .map_err(|_| format!("{ENDPOINT_ENV} is not an address."))?;
        return app
            .updater_builder()
            .endpoints(vec![url])
            .map_err(|error| error.to_string())?
            .build()
            .map_err(|error| error.to_string());
    }
    app.updater().map_err(|error| error.to_string())
}

/// Asks the update server whether a newer version exists. `None` when this one is the newest.
#[tauri::command]
pub async fn check_for_update(
    app: AppHandle,
    state: State<'_, UpdateState>,
) -> Result<Option<UpdateInfo>, String> {
    let found = updater(&app)?
        .check()
        .await
        .map_err(|error| explain(&error))?;
    let info = found.as_ref().map(info_of);
    *state
        .0
        .lock()
        .map_err(|_| "update state lock poisoned".to_owned())? = found;
    Ok(info)
}

/// Downloads the update found by the last check, checks its signature, installs it and restarts the app.
#[tauri::command]
pub async fn install_update(app: AppHandle, state: State<'_, UpdateState>) -> Result<(), String> {
    let update = state
        .0
        .lock()
        .map_err(|_| "update state lock poisoned".to_owned())?
        .take()
        .ok_or("There is no update to install. Check for updates first.")?;
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|error| explain(&error))?;
    app.restart()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_are_plain_text_cut_to_a_limit() {
        assert_eq!(
            clean_notes("  Fixes a bug.\n\n- and another\n").as_deref(),
            Some("Fixes a bug.\n\n- and another")
        );
        assert_eq!(clean_notes("   \n  "), None);
        assert_eq!(clean_notes(""), None);
        assert_eq!(
            clean_notes(&"x".repeat(5000)).unwrap().chars().count(),
            MAX_NOTES_CHARS
        );
    }

    #[test]
    fn notes_cannot_carry_control_or_hidden_characters() {
        let notes = clean_notes("fine\u{1b}[31m red\u{7}\r\n\u{202e}reversed\u{200b}").unwrap();

        assert!(
            !notes.chars().any(|c| c.is_control() && c != '\n'),
            "{notes:?}"
        );
        assert!(!notes.contains('\u{202e}') && !notes.contains('\u{200b}'));
        assert!(notes.contains("fine") && notes.contains("reversed"));
    }

    #[test]
    fn markup_in_notes_stays_text() {
        // The page shows it as text, so there is nothing to strip; it must only arrive unchanged.
        assert_eq!(
            clean_notes("<script>alert(1)</script> **bold**").as_deref(),
            Some("<script>alert(1)</script> **bold**")
        );
    }

    #[test]
    fn what_the_updater_says_becomes_a_sentence_a_person_can_act_on() {
        let signature = explain_text("Signature verification failed");
        assert!(
            signature.contains("signature check")
                && signature.contains("Nothing on your computer was changed")
        );
        assert!(
            explain_text("error sending request for url (https://x)").contains("Could not reach")
        );
        assert!(
            explain_text("Could not fetch a valid release JSON from the remote")
                .contains("no usable information")
        );
        assert_eq!(
            explain_text("something odd"),
            "Could not check for updates: something odd"
        );
    }

    #[test]
    fn a_real_updater_error_is_explained_too() {
        let error = tauri_plugin_updater::Error::ReleaseNotFound;
        assert!(explain(&error).contains("no usable information"));
        let error = tauri_plugin_updater::Error::EmptyEndpoints;
        assert!(explain(&error).starts_with("Could not check for updates:"));
    }
}
