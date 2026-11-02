//! Snippets for the command palette: which ones apply in a folder, what they ask for, and the text they become.
//!
//! The settings are read from disk on every call, like a new session does, so a snippet edited in Preferences
//! (or by hand) is there the next time the palette opens. The parsing and the rendering live in `termzip-core`;
//! this is only the doorway.

use std::collections::BTreeMap;

use serde::Serialize;
use termzip_core::{Parameter, Settings};

/// What the palette needs to list a snippet and to ask for its values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnippetInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub parameters: Vec<Parameter>,
}

/// The snippets that apply in `cwd`, in the order they were saved.
fn infos(settings: &Settings, cwd: Option<&str>) -> Vec<SnippetInfo> {
    settings
        .snippets
        .iter()
        .filter(|snippet| snippet.applies_in(cwd))
        .filter_map(|snippet| {
            Some(SnippetInfo {
                id: snippet.id.clone(),
                name: snippet.name.clone(),
                description: snippet.description.clone(),
                parameters: termzip_core::parameters(&snippet.command).ok()?,
            })
        })
        .collect()
}

fn render_by_id(
    settings: &Settings,
    id: &str,
    values: &BTreeMap<String, String>,
) -> Result<String, String> {
    let snippet = settings
        .snippets
        .iter()
        .find(|snippet| snippet.id == id)
        .ok_or("That snippet no longer exists. Open the palette again.")?;
    termzip_core::render(snippet, values)
}

#[tauri::command]
pub fn list_snippets(cwd: Option<String>) -> Result<Vec<SnippetInfo>, String> {
    let settings = Settings::load().map_err(|error| error.to_string())?;
    Ok(infos(&settings, cwd.as_deref()))
}

/// The text to put at the prompt. It never contains a line break, so it cannot run by itself.
#[tauri::command]
pub fn render_snippet(id: String, values: BTreeMap<String, String>) -> Result<String, String> {
    let settings = Settings::load().map_err(|error| error.to_string())?;
    render_by_id(&settings, &id, &values)
}

#[cfg(test)]
mod tests {
    use termzip_core::Snippet;

    use super::*;

    fn settings() -> Settings {
        let snippet = |name: &str, command: &str, folder: Option<&str>| Snippet {
            name: name.to_owned(),
            command: command.to_owned(),
            folder: folder.map(str::to_owned),
            ..Snippet::default()
        };
        Settings {
            snippets: vec![
                snippet("Status", "git status", None),
                snippet("Checkout", "git checkout {{branch:main}}", None),
                snippet("Deploy", "make deploy", Some("/srv/app")),
            ],
            ..Settings::default()
        }
        .normalized()
    }

    #[test]
    fn a_snippet_lists_what_it_asks_for() {
        let listed = infos(&settings(), None);
        let checkout = listed.iter().find(|info| info.id == "checkout").unwrap();
        assert_eq!(checkout.parameters.len(), 1);
        assert_eq!(checkout.parameters[0].name, "branch");
        assert_eq!(checkout.parameters[0].default.as_deref(), Some("main"));
        assert!(listed[0].parameters.is_empty());
    }

    #[test]
    fn a_snippet_with_a_folder_is_listed_only_inside_it() {
        let ids = |cwd| -> Vec<String> {
            infos(&settings(), cwd)
                .into_iter()
                .map(|info| info.id)
                .collect()
        };
        assert_eq!(ids(None), ["status", "checkout"]);
        assert_eq!(ids(Some("/elsewhere")), ["status", "checkout"]);
        assert_eq!(ids(Some("/srv/app/api")), ["status", "checkout", "deploy"]);
    }

    #[test]
    fn rendering_fills_the_values_and_never_ends_the_line() {
        let values = BTreeMap::from([("branch".to_owned(), "dev\nrm -rf /".to_owned())]);
        let text = render_by_id(&settings(), "checkout", &values).unwrap();
        assert_eq!(text, "git checkout devrm -rf /");
        assert!(!text.contains('\n'));
    }

    #[test]
    fn rendering_an_unknown_snippet_says_so() {
        let problem = render_by_id(&settings(), "gone", &BTreeMap::new()).unwrap_err();
        assert!(problem.contains("no longer exists"));
    }

    #[test]
    fn a_missing_value_is_reported_as_a_sentence() {
        let settings = Settings {
            snippets: vec![Snippet {
                name: "Say".to_owned(),
                command: "echo {{what}}".to_owned(),
                ..Snippet::default()
            }],
            ..Settings::default()
        }
        .normalized();
        assert_eq!(
            render_by_id(&settings, "say", &BTreeMap::new()).unwrap_err(),
            "Fill in what first."
        );
    }
}
