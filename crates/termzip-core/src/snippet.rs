//! Snippets: commands worth keeping, with optional `{{parameters}}`.
//!
//! A snippet is text that TermZip puts at the prompt. It is never run for you: no line break survives, so the text
//! waits for you to read it, edit it and press Enter yourself. A snippet may be limited to a folder, and then it
//! is offered only while a pane is in that folder or below it.
//!
//! In the command, `{{name}}` asks for a value and `{{name:default}}` asks with a value filled in. A literal `{{`
//! (a Go template, a Helm chart) is written `\{{`.

use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::paths::depth_if_inside;
use crate::profile::slug;
use crate::text::clean_typed;

pub const MAX_SNIPPETS: usize = 128;
pub const MAX_SNIPPET_NAME_CHARS: usize = 64;
pub const MAX_SNIPPET_COMMAND_BYTES: usize = 4 * 1024;
pub const MAX_SNIPPET_DESCRIPTION_CHARS: usize = 200;
pub const MAX_SNIPPET_FOLDER_BYTES: usize = 1024;
pub const MAX_PARAMETERS: usize = 16;
pub const MAX_PARAMETER_NAME_CHARS: usize = 32;
pub const MAX_PARAMETER_VALUE_BYTES: usize = 1024;

/// A command to keep. `id` is stable (`a-z`, `0-9`, `-`) and derived from the name when empty.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Snippet {
    pub id: String,
    pub name: String,
    pub command: String,
    pub description: Option<String>,
    /// Offered only in this folder and below; `~` stands for the home folder. Everywhere when empty.
    pub folder: Option<String>,
}

/// A value a snippet asks for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub default: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
enum Piece<'a> {
    Text(&'a str),
    /// A literal `{{`, written `\{{`.
    Open,
    Parameter(Parameter),
}

fn is_valid_parameter_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    name.len() <= MAX_PARAMETER_NAME_CHARS
        && (first.is_ascii_lowercase() || first == '_')
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// Splits a command into text, escaped braces and parameters.
fn pieces(command: &str) -> Result<Vec<Piece<'_>>, String> {
    let mut out = Vec::new();
    let mut rest = command;
    loop {
        let Some(at) = rest.find("{{") else {
            if !rest.is_empty() {
                out.push(Piece::Text(rest));
            }
            return Ok(out);
        };
        if rest[..at].ends_with('\\') {
            // `\{{`: the backslash goes, the braces stay.
            let before = &rest[..at - 1];
            if !before.is_empty() {
                out.push(Piece::Text(before));
            }
            out.push(Piece::Open);
            rest = &rest[at + 2..];
            continue;
        }
        if at > 0 {
            out.push(Piece::Text(&rest[..at]));
        }
        let after = &rest[at + 2..];
        let Some(end) = after.find("}}") else {
            return Err(
                "A “{{” in the command has no closing “}}”. Write \\{{ for a literal one."
                    .to_owned(),
            );
        };
        let inside = &after[..end];
        let (name, default) = match inside.split_once(':') {
            Some((name, default)) => (name, Some(default.to_owned())),
            None => (inside, None),
        };
        if !is_valid_parameter_name(name) {
            return Err(format!(
                "“{name}” is not a valid parameter name (lowercase letters, digits and _, not starting with a digit, up to {MAX_PARAMETER_NAME_CHARS} characters). Write \\{{{{ for a literal “{{{{”."
            ));
        }
        out.push(Piece::Parameter(Parameter {
            name: name.to_owned(),
            default,
        }));
        rest = &after[end + 2..];
    }
}

/// The values a command asks for, in the order they first appear. A name used twice is asked once, with the
/// default of its first use.
pub fn parameters(command: &str) -> Result<Vec<Parameter>, String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for piece in pieces(command)? {
        if let Piece::Parameter(parameter) = piece
            && seen.insert(parameter.name.clone())
        {
            out.push(parameter);
        }
    }
    if out.len() > MAX_PARAMETERS {
        return Err(format!(
            "A snippet can ask for at most {MAX_PARAMETERS} values."
        ));
    }
    Ok(out)
}

/// The text to type: the parameters filled in (a value that was not given uses its default), then cleaned of
/// control characters, so nothing in a value can press Enter or start an escape sequence.
pub fn render(snippet: &Snippet, values: &BTreeMap<String, String>) -> Result<String, String> {
    let asked = parameters(&snippet.command)?;
    for (name, value) in values {
        if value.len() > MAX_PARAMETER_VALUE_BYTES {
            return Err(format!(
                "The value of {name} is longer than {MAX_PARAMETER_VALUE_BYTES} bytes."
            ));
        }
    }
    let value_of = |name: &str| -> Result<&str, String> {
        if let Some(value) = values.get(name) {
            return Ok(value);
        }
        asked
            .iter()
            .find(|parameter| parameter.name == name)
            .and_then(|parameter| parameter.default.as_deref())
            .ok_or_else(|| format!("Fill in {name} first."))
    };
    let mut out = String::new();
    for piece in pieces(&snippet.command)? {
        match piece {
            Piece::Text(text) => out.push_str(text),
            Piece::Open => out.push_str("{{"),
            Piece::Parameter(parameter) => out.push_str(value_of(&parameter.name)?),
        }
    }
    Ok(clean_typed(&out, MAX_SNIPPET_COMMAND_BYTES))
}

impl Snippet {
    /// Whether the snippet is offered in `cwd`. A snippet without a folder is offered everywhere; one with a
    /// folder needs a known working directory inside it.
    pub fn applies_in(&self, cwd: Option<&str>) -> bool {
        let Some(folder) = self.folder.as_deref() else {
            return true;
        };
        let Some(cwd) = cwd else {
            return false;
        };
        depth_if_inside(folder, cwd).is_some()
    }
}

fn has_control(text: &str) -> bool {
    text.chars().any(char::is_control)
}

/// What a command asks for, or the sentence that says why it cannot be a snippet. The dialog calls this while
/// the command is typed; saving checks the same things.
pub fn check_command(command: &str) -> Result<Vec<Parameter>, String> {
    if command.len() > MAX_SNIPPET_COMMAND_BYTES {
        return Err(format!(
            "The command is longer than {MAX_SNIPPET_COMMAND_BYTES} bytes."
        ));
    }
    if has_control(command) {
        return Err(
            "The command must be one line, without Tab or other control characters.".to_owned(),
        );
    }
    parameters(command)
}

/// The first problem found, as a sentence the preferences dialog can show as it is.
pub fn validate_snippets(snippets: &[Snippet]) -> Result<(), String> {
    if snippets.len() > MAX_SNIPPETS {
        return Err(format!("At most {MAX_SNIPPETS} snippets are supported."));
    }
    for snippet in snippets {
        let name = snippet.name.trim();
        if name.is_empty() {
            return Err("Every snippet needs a name.".to_owned());
        }
        if name.chars().count() > MAX_SNIPPET_NAME_CHARS {
            return Err(format!(
                "The snippet name “{name}” is longer than {MAX_SNIPPET_NAME_CHARS} characters."
            ));
        }
        if snippet.command.trim().is_empty() {
            return Err(format!("The snippet “{name}” has no command."));
        }
        check_command(&snippet.command).map_err(|problem| format!("In “{name}”: {problem}"))?;
        if let Some(description) = &snippet.description
            && description.chars().count() > MAX_SNIPPET_DESCRIPTION_CHARS
        {
            return Err(format!(
                "The description of “{name}” is longer than {MAX_SNIPPET_DESCRIPTION_CHARS} characters."
            ));
        }
        if let Some(folder) = &snippet.folder
            && (folder.len() > MAX_SNIPPET_FOLDER_BYTES || has_control(folder))
        {
            return Err(format!("The folder of “{name}” is not usable."));
        }
    }
    Ok(())
}

fn blank_to_none(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

/// Cleans what the file or the dialog handed over: bounds every field, drops what cannot be used and gives
/// every snippet a unique id. It never fails; `validate_snippets` reports problems first.
pub(crate) fn normalize_snippets(snippets: Vec<Snippet>) -> Vec<Snippet> {
    let mut taken: HashSet<String> = HashSet::new();
    let mut out = Vec::new();
    for mut snippet in snippets.into_iter().take(MAX_SNIPPETS) {
        let name: String = snippet
            .name
            .trim()
            .chars()
            .take(MAX_SNIPPET_NAME_CHARS)
            .collect();
        snippet.name = if name.is_empty() {
            "Snippet".to_owned()
        } else {
            name
        };
        // A command is one line: whatever is not a printable character is dropped, and a command that is
        // then empty or does not parse is not a snippet.
        snippet.command = clean_typed(&snippet.command, MAX_SNIPPET_COMMAND_BYTES)
            .trim()
            .to_owned();
        if snippet.command.is_empty() || parameters(&snippet.command).is_err() {
            continue;
        }
        snippet.description = blank_to_none(snippet.description).map(|text| {
            text.chars()
                .filter(|c| !c.is_control())
                .take(MAX_SNIPPET_DESCRIPTION_CHARS)
                .collect()
        });
        snippet.folder = blank_to_none(snippet.folder)
            .filter(|folder| folder.len() <= MAX_SNIPPET_FOLDER_BYTES && !has_control(folder));

        let mut base = slug(&snippet.id);
        if base.is_empty() {
            base = slug(&snippet.name);
        }
        if base.is_empty() {
            base = "snippet".to_owned();
        }
        let mut id = base.clone();
        let mut number = 2;
        while !taken.insert(id.clone()) {
            id = format!("{base}-{number}");
            number += 1;
        }
        snippet.id = id;
        out.push(snippet);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snippet(command: &str) -> Snippet {
        Snippet {
            name: "Test".to_owned(),
            command: command.to_owned(),
            ..Snippet::default()
        }
    }

    fn values(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    #[test]
    fn a_command_without_parameters_asks_for_nothing() {
        assert_eq!(parameters("git status").unwrap(), []);
        assert_eq!(
            render(&snippet("git status"), &BTreeMap::new()).unwrap(),
            "git status"
        );
    }

    #[test]
    fn parameters_come_in_order_and_are_asked_once() {
        let asked = parameters("git checkout {{branch:main}} && echo {{branch}} {{msg}}").unwrap();
        assert_eq!(
            asked,
            [
                Parameter {
                    name: "branch".to_owned(),
                    default: Some("main".to_owned())
                },
                Parameter {
                    name: "msg".to_owned(),
                    default: None
                },
            ]
        );
    }

    #[test]
    fn values_fill_the_parameters_and_defaults_fill_the_rest() {
        let s = snippet("git checkout {{branch:main}} -- {{path}}");
        assert_eq!(
            render(&s, &values(&[("path", "src")])).unwrap(),
            "git checkout main -- src"
        );
        assert_eq!(
            render(&s, &values(&[("branch", "dev"), ("path", "a")])).unwrap(),
            "git checkout dev -- a"
        );
    }

    #[test]
    fn an_empty_value_is_a_value() {
        let s = snippet("ls {{flags:-l}}");
        assert_eq!(render(&s, &values(&[("flags", "")])).unwrap(), "ls ");
    }

    #[test]
    fn a_missing_value_without_a_default_is_asked_for() {
        let problem = render(&snippet("echo {{what}}"), &BTreeMap::new()).unwrap_err();
        assert_eq!(problem, "Fill in what first.");
    }

    #[test]
    fn a_default_may_contain_a_colon() {
        let asked = parameters("curl {{url:http://localhost:8080}}").unwrap();
        assert_eq!(asked[0].default.as_deref(), Some("http://localhost:8080"));
    }

    #[test]
    fn escaped_braces_are_literal() {
        let s = snippet(r"docker inspect --format '\{{.State}}' {{name}}");
        assert_eq!(parameters(&s.command).unwrap().len(), 1);
        assert_eq!(
            render(&s, &values(&[("name", "web")])).unwrap(),
            "docker inspect --format '{{.State}}' web"
        );
    }

    #[test]
    fn a_value_cannot_run_the_command() {
        let s = snippet("echo {{x}}");
        let typed = render(&s, &values(&[("x", "a\nrm -rf /\r\u{1b}[201~\t")])).unwrap();
        assert_eq!(typed, "echo arm -rf /[201~");
        assert!(!typed.chars().any(char::is_control));
    }

    #[test]
    fn a_value_cannot_hide_text() {
        let typed = render(&snippet("echo {{x}}"), &values(&[("x", "a\u{202E}b")])).unwrap();
        assert_eq!(typed, "echo ab");
    }

    #[test]
    fn a_value_that_is_too_long_is_refused() {
        let long = "x".repeat(MAX_PARAMETER_VALUE_BYTES + 1);
        assert!(render(&snippet("echo {{x}}"), &values(&[("x", &long)])).is_err());
    }

    #[test]
    fn the_rendered_text_is_bounded() {
        let big = "y".repeat(MAX_PARAMETER_VALUE_BYTES);
        let s = snippet(&format!(
            "{} {{{{a}}}} {{{{b}}}} {{{{c}}}} {{{{d}}}} {{{{e}}}}",
            "z".repeat(3800)
        ));
        let typed = render(
            &s,
            &values(&[
                ("a", &big),
                ("b", &big),
                ("c", &big),
                ("d", &big),
                ("e", &big),
            ]),
        )
        .unwrap();
        assert!(typed.len() <= MAX_SNIPPET_COMMAND_BYTES);
    }

    #[test]
    fn malformed_parameters_are_explained() {
        assert!(parameters("echo {{oops").unwrap_err().contains("closing"));
        assert!(
            parameters("echo {{Bad}}")
                .unwrap_err()
                .contains("valid parameter name")
        );
        assert!(parameters("echo {{}}").is_err());
        assert!(parameters("echo {{1x}}").is_err());
        assert!(parameters("echo {{.State}}").unwrap_err().contains("\\{{"));
    }

    #[test]
    fn too_many_parameters_are_refused() {
        let command: String = (0..=MAX_PARAMETERS)
            .map(|n| format!("{{{{p{n}}}}} "))
            .collect();
        assert!(parameters(&command).is_err());
        let ok: String = (0..MAX_PARAMETERS)
            .map(|n| format!("{{{{p{n}}}}} "))
            .collect();
        assert_eq!(parameters(&ok).unwrap().len(), MAX_PARAMETERS);
    }

    #[test]
    fn a_snippet_without_a_folder_applies_everywhere() {
        assert!(snippet("ls").applies_in(None));
        assert!(snippet("ls").applies_in(Some("/anywhere")));
    }

    #[test]
    fn a_snippet_with_a_folder_applies_inside_it() {
        let mut s = snippet("make");
        s.folder = Some("/srv/app".to_owned());
        assert!(s.applies_in(Some("/srv/app")));
        assert!(s.applies_in(Some("/srv/app/src/deep")));
        assert!(!s.applies_in(Some("/srv/application")));
        assert!(!s.applies_in(Some("/srv")));
        assert!(!s.applies_in(None));
    }

    #[test]
    fn validation_explains_each_problem() {
        let named = |name: &str, command: &str| Snippet {
            name: name.to_owned(),
            command: command.to_owned(),
            ..Snippet::default()
        };
        assert!(
            validate_snippets(&[named("", "ls")])
                .unwrap_err()
                .contains("needs a name")
        );
        assert!(
            validate_snippets(&[named("a", "  ")])
                .unwrap_err()
                .contains("no command")
        );
        assert!(
            validate_snippets(&[named("a", "ls\nrm x")])
                .unwrap_err()
                .contains("one line")
        );
        assert!(
            validate_snippets(&[named("a", "ls\tx")])
                .unwrap_err()
                .contains("one line")
        );
        assert!(
            validate_snippets(&[named("a", "echo {{")])
                .unwrap_err()
                .contains("In “a”")
        );
        assert!(
            validate_snippets(&[named(&"n".repeat(65), "ls")])
                .unwrap_err()
                .contains("longer")
        );
        assert!(validate_snippets(&[named("a", &"x".repeat(5000))]).is_err());
        let many: Vec<Snippet> = (0..=MAX_SNIPPETS).map(|_| named("a", "ls")).collect();
        assert!(validate_snippets(&many).is_err());
        assert!(validate_snippets(&[named("a", "ls {{x}}")]).is_ok());
    }

    #[test]
    fn normalizing_makes_ids_unique_and_drops_what_cannot_be_used() {
        let named = |name: &str, command: &str| Snippet {
            name: name.to_owned(),
            command: command.to_owned(),
            ..Snippet::default()
        };
        let cleaned = normalize_snippets(vec![
            named("Git Status", "git status"),
            named("Git Status", "git status -s"),
            named("Empty", "   "),
            named("Broken", "echo {{"),
            named("Lines", "a\nb"),
        ]);
        let ids: Vec<&str> = cleaned.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, ["git-status", "git-status-2", "lines"]);
        assert_eq!(cleaned[2].command, "ab");
    }

    #[test]
    fn normalizing_never_exceeds_the_limits() {
        let many: Vec<Snippet> = (0..300).map(|n| snippet(&format!("echo {n}"))).collect();
        assert_eq!(normalize_snippets(many).len(), MAX_SNIPPETS);
        let long = Snippet {
            name: "n".repeat(200),
            command: "x".repeat(10_000),
            description: Some("d".repeat(900)),
            folder: Some("f".repeat(5000)),
            ..Snippet::default()
        };
        let cleaned = normalize_snippets(vec![long]).remove(0);
        assert_eq!(cleaned.name.chars().count(), MAX_SNIPPET_NAME_CHARS);
        assert_eq!(cleaned.command.len(), MAX_SNIPPET_COMMAND_BYTES);
        assert_eq!(
            cleaned.description.unwrap().chars().count(),
            MAX_SNIPPET_DESCRIPTION_CHARS
        );
        assert_eq!(cleaned.folder, None);
    }

    #[test]
    fn checking_a_command_gives_its_parameters_or_a_sentence() {
        assert_eq!(check_command("git status").unwrap(), []);
        assert_eq!(check_command("echo {{a}} {{b:x}}").unwrap().len(), 2);
        assert!(check_command("ls\nrm x").unwrap_err().contains("one line"));
        assert!(check_command("echo {{").unwrap_err().contains("closing"));
        assert!(
            check_command(&"x".repeat(5000))
                .unwrap_err()
                .contains("longer")
        );
    }
}
