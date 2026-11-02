//! Readers for the files that name a project's tasks. Each takes the text of a file and returns the names it
//! found; none touches the disk, and none panics on hostile input (a repository you cloned is not trusted).
//!
//! A name ends up in a command that is typed at the prompt, so only plain names are kept: letters, digits and
//! `. _ : / @ + -`, at most [`MAX_NAME_CHARS`] characters and never starting with `-` (which a tool would read as
//! an option). Anything else is skipped, not repaired.

use serde_json::Value;

/// The longest task name that is offered.
pub const MAX_NAME_CHARS: usize = 64;

/// Whether `name` is safe to type after a tool's name.
pub fn is_plain_name(name: &str) -> bool {
    !name.is_empty()
        && name.chars().count() <= MAX_NAME_CHARS
        && !name.starts_with('-')
        && name.chars().all(|c| {
            c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | ':' | '/' | '@' | '+' | '-')
        })
}

fn push_unique(names: &mut Vec<String>, name: &str) {
    if is_plain_name(name) && !names.iter().any(|known| known == name) {
        names.push(name.to_owned());
    }
}

fn object_keys(text: &str, path: &[&str]) -> Vec<String> {
    let Ok(mut value) = serde_json::from_str::<Value>(text) else {
        return Vec::new();
    };
    for key in path {
        let Some(next) = value.get_mut(*key) else {
            return Vec::new();
        };
        value = next.take();
    }
    let Value::Object(map) = value else {
        return Vec::new();
    };
    let mut names = Vec::new();
    for key in map.keys() {
        push_unique(&mut names, key);
    }
    names
}

/// The `scripts` of a `package.json`, by name.
pub fn package_scripts(text: &str) -> Vec<String> {
    object_keys(text, &["scripts"])
}

/// Which package manager a `package.json` asks for in its `packageManager` field (`pnpm@9.1.0`), when it does
/// and the name is one this crate knows.
pub fn package_manager_field(text: &str) -> Option<&'static str> {
    let value: Value = serde_json::from_str(text).ok()?;
    let field = value.get("packageManager")?.as_str()?;
    match field.split('@').next()? {
        "npm" => Some("npm"),
        "pnpm" => Some("pnpm"),
        "yarn" => Some("yarn"),
        "bun" => Some("bun"),
        _ => None,
    }
}

/// The `tasks` of a `deno.json`, by name. A file with comments (`deno.jsonc`) is not plain JSON and gives none.
pub fn deno_tasks(text: &str) -> Vec<String> {
    object_keys(text, &["tasks"])
}

/// The `[alias]` entries of a Cargo config file, by name.
pub fn cargo_aliases(text: &str) -> Vec<String> {
    let Ok(table) = toml::from_str::<toml::Table>(text) else {
        return Vec::new();
    };
    let mut names = Vec::new();
    if let Some(toml::Value::Table(aliases)) = table.get("alias") {
        for name in aliases.keys() {
            push_unique(&mut names, name);
        }
    }
    names
}

/// The targets of a Makefile that a person would run: plain names at the start of a line. Special targets
/// (`.PHONY`), pattern rules (`%.o`), file paths (`out/app`), variable assignments (`CC := gcc`) and the lines of a
/// recipe are left out.
pub fn makefile_targets(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in text.lines() {
        // A recipe line starts with a Tab; a comment, a blank line and anything indented are not rules.
        if line.is_empty() || line.starts_with(['\t', ' ', '#']) {
            continue;
        }
        let Some(colon) = line.find(':') else {
            continue;
        };
        let (before, after) = (&line[..colon], &line[colon + 1..]);
        // `CC := gcc`, `CC ::= gcc`, `CC = a:b`: a variable, not a target.
        if after.starts_with('=') || after.starts_with(":=") || before.contains('=') {
            continue;
        }
        for word in before.split_whitespace() {
            if word.starts_with('.') || word.contains(['%', '$', '/', '(']) {
                continue;
            }
            push_unique(&mut names, word);
        }
    }
    names
}

/// The recipes of a justfile that a person would run: the names at the start of a line, before the parameters.
/// Private recipes (`_name`, or after `[private]`) and the settings, aliases, variables and imports are left out.
pub fn justfile_recipes(text: &str) -> Vec<String> {
    const KEYWORDS: [&str; 6] = ["set", "alias", "export", "import", "mod", "unexport"];
    let mut names = Vec::new();
    let mut private = false;
    for line in text.lines() {
        if line.is_empty() || line.starts_with([' ', '\t', '#']) {
            continue;
        }
        if line.starts_with('[') {
            // `[private]`, `[no-cd]`, `[group('x')]`: attributes of the recipe on the next line.
            private |= line.contains("private");
            continue;
        }
        let was_private = std::mem::take(&mut private);
        let Some(colon) = line.find(':') else {
            continue;
        };
        let head = line[..colon].trim_start_matches('@');
        // `name := value` is a variable, and a `=` before the colon is a parameter default or an assignment.
        if line[colon + 1..].starts_with('=') {
            continue;
        }
        let Some(name) = head.split_whitespace().next() else {
            continue;
        };
        if KEYWORDS.contains(&name) && head.split_whitespace().count() > 1 {
            continue;
        }
        if name.contains('=') || was_private || name.starts_with('_') {
            continue;
        }
        push_unique(&mut names, name);
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_names_are_letters_digits_and_a_few_marks() {
        for good in [
            "build",
            "test:unit",
            "lint-fix",
            "docs/api",
            "@scope+x",
            "v1.2",
        ] {
            assert!(is_plain_name(good), "{good}");
        }
        for bad in [
            "", "-rf", "a b", "a;b", "a$(b)", "a`b`", "a|b", "a&b", "a\nb", "a>b", "é",
        ] {
            assert!(!is_plain_name(bad), "{bad:?}");
        }
        assert!(is_plain_name(&"a".repeat(MAX_NAME_CHARS)));
        assert!(!is_plain_name(&"a".repeat(MAX_NAME_CHARS + 1)));
    }

    #[test]
    fn scripts_are_read_by_name_and_unsafe_names_are_skipped() {
        let text = r#"{"scripts": {"build": "vite build", "x; rm -rf /": "no", "$(evil)": "no", "test:unit": "vitest", "-flag": "no"}}"#;
        let mut names = package_scripts(text);
        names.sort();
        assert_eq!(names, ["build", "test:unit"]);
    }

    #[test]
    fn a_package_json_without_scripts_or_that_is_not_json_gives_nothing() {
        assert!(package_scripts(r#"{"name": "x"}"#).is_empty());
        assert!(package_scripts(r#"{"scripts": ["a"]}"#).is_empty());
        assert!(package_scripts("not json").is_empty());
        assert!(package_scripts("").is_empty());
        assert!(package_scripts(&"[".repeat(100_000)).is_empty());
    }

    #[test]
    fn the_package_manager_field_names_a_known_manager() {
        let field =
            |value: &str| package_manager_field(&format!(r#"{{"packageManager": "{value}"}}"#));
        assert_eq!(field("pnpm@9.1.0"), Some("pnpm"));
        assert_eq!(field("yarn@4.0.0+sha256.abc"), Some("yarn"));
        assert_eq!(field("bun@1.1.0"), Some("bun"));
        assert_eq!(field("npm@10"), Some("npm"));
        assert_eq!(field("evil@1; rm -rf /"), None);
        assert_eq!(package_manager_field("{}"), None);
    }

    #[test]
    fn deno_tasks_are_read_by_name() {
        assert_eq!(
            deno_tasks(r#"{"tasks": {"dev": "deno run main.ts"}}"#),
            ["dev"]
        );
        assert!(deno_tasks("{ // comment\n}").is_empty());
    }

    #[test]
    fn cargo_aliases_are_read_by_name() {
        let text = "[alias]\nxtask = \"run --package xtask --\"\nb = [\"build\", \"--release\"]\n\n[build]\njobs = 4\n";
        assert_eq!(cargo_aliases(text), ["b", "xtask"]);
        assert!(cargo_aliases("[alias]\n= broken").is_empty());
        assert!(cargo_aliases("[build]\njobs = 4").is_empty());
    }

    #[test]
    fn makefile_targets_skip_what_is_not_a_task() {
        let text = "\
CC := gcc
CFLAGS ?= -O2
VERSION = 1:2
.PHONY: all build test
include config.mk
ifeq ($(OS),Windows_NT)
endif

all: build test
\t@echo hi
build test: deps
\tcc -o app main.c
%.o: %.c
\t$(CC) -c $<
out/app: main.c
lint:; echo lint
# clean: not a rule
  indented: no
$(TARGET): x
";
        assert_eq!(makefile_targets(text), ["all", "build", "test", "lint"]);
    }

    #[test]
    fn justfile_recipes_skip_settings_variables_and_private_recipes() {
        let text = "\
set shell := [\"bash\", \"-c\"]
version := \"1.0\"
export RUST_LOG := \"debug\"
alias b := build

# Build it
build target=\"debug\":
    cargo build

@test *args:
    cargo test {{args}}

[private]
helper:
    echo hi

_hidden:
    echo no

[no-cd]
deploy env: build
    ./deploy {{env}}
";
        assert_eq!(justfile_recipes(text), ["build", "test", "deploy"]);
    }

    #[test]
    fn hostile_text_never_panics() {
        let nasty = [
            "".to_owned(),
            ":".to_owned(),
            "::::".to_owned(),
            "\0\0\0".to_owned(),
            "a".repeat(300_000),
            ":a".repeat(100_000),
            "[".repeat(50_000),
            "é:\u{202e}x\n".repeat(1_000),
            "\r\n:\r\n".repeat(10_000),
        ];
        for text in &nasty {
            let _ = (
                package_scripts(text),
                deno_tasks(text),
                cargo_aliases(text),
                makefile_targets(text),
                justfile_recipes(text),
            );
        }
    }
}
