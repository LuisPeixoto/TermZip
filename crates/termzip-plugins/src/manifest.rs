//! Reading the manifest out of a plugin file and checking everything about it that can be checked without
//! running it.
//!
//! The file is untrusted input. It is size-limited, must be a binary core module with exactly one
//! manifest, may import nothing (a plugin cannot call out of its sandbox), and must export what the
//! API needs for what it declares. Every refusal is a sentence the interface can show.

use std::collections::{BTreeMap, HashSet};

use regex::RegexBuilder;
use termzip_plugin_api::{
    API_VERSION, MANIFEST_SECTION, MAX_COMMANDS, MAX_LINKS, MAX_MANIFEST_BYTES, MAX_NAME_CHARS,
    MAX_PATTERN_CHARS, MAX_PLUGIN_BYTES, Manifest, Permission,
};
use wasmparser::{Encoding, ExternalKind, Parser, Payload};

const MAX_DESCRIPTION_CHARS: usize = 280;
const MAX_CREDIT_CHARS: usize = 128;
const MAX_URL_CHARS: usize = 512;
/// The compiled size a link pattern may take; patterns run on every line of output.
const PATTERN_SIZE_LIMIT: usize = 1 << 20;

/// A plugin file that passed every check: its manifest and the exports it provides.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
    pub manifest: Manifest,
}

/// A slug: `a-z`, `0-9` and single `-` between them.
fn is_slug(text: &str) -> bool {
    !text.is_empty()
        && text.chars().count() <= MAX_NAME_CHARS
        && text.split('-').all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        })
}

fn too_long(what: &str, limit: usize) -> String {
    format!("{what} is longer than {limit} characters.")
}

/// `{0}` stands for the whole match and `{1}`, `{2}`… for groups; nothing else may use braces.
fn check_url_template(template: &str, groups: usize) -> Result<(), String> {
    if template.chars().count() > MAX_URL_CHARS {
        return Err(too_long("A link address", MAX_URL_CHARS));
    }
    if !(template.starts_with("https://") || template.starts_with("http://")) {
        return Err(format!(
            "The link address “{template}” must start with http:// or https://."
        ));
    }
    if template
        .chars()
        .any(|c| c.is_whitespace() || c.is_control())
    {
        return Err(format!(
            "The link address “{template}” has a space or a control character in it."
        ));
    }
    // A placeholder in the host would let text from the terminal choose which site the link goes to.
    let after_scheme = template
        .split_once("://")
        .map_or(template, |(_, rest)| rest);
    let authority = after_scheme.split(['/', '?', '#']).next().unwrap_or("");
    if authority.contains(['{', '}']) {
        return Err(format!(
            "The link address “{template}” has a placeholder in its host name; only the path and the query may change."
        ));
    }
    let mut rest = template;
    while let Some(open) = rest.find(['{', '}']) {
        let tail = &rest[open..];
        let Some(inner) = tail.strip_prefix('{') else {
            return Err(format!("The link address “{template}” has a stray “}}”."));
        };
        let Some(close) = inner.find('}') else {
            return Err(format!(
                "The link address “{template}” has an unclosed “{{”."
            ));
        };
        let number = &inner[..close];
        match number.parse::<usize>() {
            Ok(n) if n <= groups && number.len() <= 2 => {}
            _ => {
                return Err(format!(
                    "The link address “{template}” uses {{{number}}}, but its pattern has {groups} group{}.",
                    if groups == 1 { "" } else { "s" }
                ));
            }
        }
        rest = &inner[close + 1..];
    }
    Ok(())
}

/// Checks a manifest that was read from a file or came from a catalog.
pub fn validate_manifest(manifest: &Manifest) -> Result<(), String> {
    if manifest.api != API_VERSION {
        return Err(format!(
            "This plugin was written for plugin API {}; this TermZip understands API {API_VERSION}.",
            manifest.api
        ));
    }
    if !is_slug(&manifest.id) {
        return Err(format!(
            "The plugin id “{}” must be lowercase letters, digits and “-”, up to {MAX_NAME_CHARS} characters.",
            manifest.id
        ));
    }
    let name = manifest.name.trim();
    if name.is_empty() {
        return Err("The plugin needs a name.".to_owned());
    }
    if name.chars().count() > MAX_NAME_CHARS {
        return Err(too_long("The plugin name", MAX_NAME_CHARS));
    }
    if semver::Version::parse(&manifest.version).is_err() {
        return Err(format!(
            "The version “{}” must look like 1.2.3.",
            manifest.version
        ));
    }
    if manifest.description.chars().count() > MAX_DESCRIPTION_CHARS {
        return Err(too_long("The description", MAX_DESCRIPTION_CHARS));
    }
    for (what, value) in [
        ("The author", &manifest.author),
        ("The license", &manifest.license),
    ] {
        if value
            .as_ref()
            .is_some_and(|v| v.chars().count() > MAX_CREDIT_CHARS)
        {
            return Err(too_long(what, MAX_CREDIT_CHARS));
        }
    }
    if let Some(homepage) = &manifest.homepage
        && !(homepage.starts_with("https://")
            || homepage.starts_with("http://")
            || homepage.chars().count() > MAX_URL_CHARS)
    {
        return Err("The homepage must be an http:// or https:// address.".to_owned());
    }

    let mut seen = HashSet::new();
    for permission in &manifest.permissions {
        if !seen.insert(*permission) {
            return Err(format!("The permission “{permission:?}” is listed twice."));
        }
    }
    let has = |permission: Permission| manifest.permissions.contains(&permission);
    if has(Permission::Output) && !has(Permission::Blocks) {
        return Err("The “output” permission needs the “blocks” permission too.".to_owned());
    }
    if (has(Permission::Type) || has(Permission::Open)) && manifest.commands.is_empty() {
        return Err(
            "The plugin asks to type text or open addresses but has no commands to do it from."
                .to_owned(),
        );
    }

    if manifest.commands.len() > MAX_COMMANDS {
        return Err(format!("At most {MAX_COMMANDS} commands are supported."));
    }
    let mut ids = HashSet::new();
    for command in &manifest.commands {
        if !is_slug(&command.id) {
            return Err(format!(
                "The command id “{}” must be lowercase letters, digits and “-”.",
                command.id
            ));
        }
        if !ids.insert(command.id.as_str()) {
            return Err(format!("The command id “{}” is used twice.", command.id));
        }
        let title = command.title.trim();
        if title.is_empty() {
            return Err(format!("The command “{}” needs a title.", command.id));
        }
        if title.chars().count() > MAX_NAME_CHARS {
            return Err(too_long("A command title", MAX_NAME_CHARS));
        }
    }

    if manifest.links.len() > MAX_LINKS {
        return Err(format!("At most {MAX_LINKS} link patterns are supported."));
    }
    for link in &manifest.links {
        if link.pattern.is_empty() || link.pattern.chars().count() > MAX_PATTERN_CHARS {
            return Err(format!(
                "A link pattern must be between 1 and {MAX_PATTERN_CHARS} characters."
            ));
        }
        let regex = RegexBuilder::new(&link.pattern)
            .size_limit(PATTERN_SIZE_LIMIT)
            .build()
            .map_err(|error| {
                format!(
                    "The link pattern “{}” is not usable: {}",
                    link.pattern,
                    short(&error.to_string())
                )
            })?;
        if regex.is_match("") {
            return Err(format!(
                "The link pattern “{}” matches nothing at all, so it would link every line.",
                link.pattern
            ));
        }
        check_url_template(&link.url, regex.captures_len() - 1)?;
    }

    if manifest.commands.is_empty() && manifest.links.is_empty() && !has(Permission::Blocks) {
        return Err(
            "This plugin does nothing: it has no commands, no link patterns and does not watch commands."
                .to_owned(),
        );
    }
    Ok(())
}

/// The first line of a library error, which is the readable one.
fn short(message: &str) -> String {
    message.lines().last().unwrap_or(message).trim().to_owned()
}

/// Reads a plugin file: structure, manifest, imports and exports. Nothing of it is run.
pub fn read_module(bytes: &[u8]) -> Result<Module, String> {
    if bytes.len() > MAX_PLUGIN_BYTES {
        return Err(format!(
            "That plugin file is larger than {} MiB.",
            MAX_PLUGIN_BYTES / (1024 * 1024)
        ));
    }
    if !bytes.starts_with(b"\0asm") {
        return Err(
            "That is not a WebAssembly file. A plugin is a compiled .wasm module.".to_owned(),
        );
    }

    let mut manifests: Vec<&[u8]> = Vec::new();
    let mut exports: BTreeMap<&str, ExternalKind> = BTreeMap::new();
    for payload in Parser::new(0).parse_all(bytes) {
        let invalid = |error: wasmparser::BinaryReaderError| {
            format!(
                "That is not a valid WebAssembly module: {}.",
                error.message()
            )
        };
        match payload.map_err(invalid)? {
            Payload::Version { encoding, .. } if encoding != Encoding::Module => {
                return Err("A plugin is a core WebAssembly module, not a component.".to_owned());
            }
            Payload::CustomSection(section) if section.name() == MANIFEST_SECTION => {
                manifests.push(section.data());
            }
            Payload::ImportSection(reader) => {
                if let Some(import) = reader.into_iter().next() {
                    let import = import.map_err(invalid)?;
                    return Err(format!(
                        "The plugin imports “{}.{}”, which TermZip does not provide. A plugin cannot call anything outside its sandbox.",
                        import.module, import.name
                    ));
                }
            }
            Payload::ExportSection(reader) => {
                for export in reader {
                    let export = export.map_err(invalid)?;
                    exports.insert(export.name, export.kind);
                }
            }
            _ => {}
        }
    }

    let manifest_bytes = match manifests.as_slice() {
        [] => {
            return Err(format!(
                "That WebAssembly file has no TermZip manifest (the custom section “{MANIFEST_SECTION}”)."
            ));
        }
        [one] => *one,
        _ => return Err("That plugin has more than one manifest.".to_owned()),
    };
    if manifest_bytes.len() > MAX_MANIFEST_BYTES {
        return Err(format!(
            "The plugin's manifest is larger than {} KiB.",
            MAX_MANIFEST_BYTES / 1024
        ));
    }
    let text = std::str::from_utf8(manifest_bytes)
        .map_err(|_| "The plugin's manifest is not UTF-8 text.".to_owned())?;
    let manifest: Manifest = toml::from_str(text).map_err(|error| {
        format!(
            "The plugin's manifest is not valid: {}",
            short(error.message())
        )
    })?;
    validate_manifest(&manifest)?;

    let require = |name: &str, kind: ExternalKind, what: &str| -> Result<(), String> {
        match exports.get(name) {
            Some(found) if *found == kind => Ok(()),
            Some(_) => Err(format!(
                "The plugin exports “{name}”, but it is not {what}."
            )),
            None => Err(format!(
                "The plugin does not export “{name}”, which TermZip needs {}.",
                match name {
                    "memory" | "tz_alloc" => "to pass it data",
                    "tz_on_block" => "to tell it about finished commands",
                    _ => "to run its commands",
                }
            )),
        }
    };
    require("memory", ExternalKind::Memory, "a memory")?;
    require("tz_alloc", ExternalKind::Func, "a function")?;
    if manifest.permissions.contains(&Permission::Blocks) {
        require("tz_on_block", ExternalKind::Func, "a function")?;
    }
    if !manifest.commands.is_empty() {
        require("tz_run_command", ExternalKind::Func, "a function")?;
    }
    Ok(Module { manifest })
}

#[cfg(test)]
pub(crate) mod fixtures {
    //! Plugin files for the tests, written in WebAssembly text and assembled here.

    pub fn escape(text: &str) -> String {
        text.bytes().map(|b| format!("\\{b:02x}")).collect()
    }

    /// A module with this manifest and exactly this body: the test writes every function and export.
    pub fn module(manifest: &str, body: &str) -> Vec<u8> {
        let text = format!(
            "(module\n  (@custom \"termzip-plugin\" \"{}\")\n  {body}\n)",
            escape(manifest)
        );
        wat::parse_str(&text).unwrap_or_else(|error| panic!("{error}\n{text}"))
    }

    /// A module with the exports a plugin that watches commands and has commands needs, doing nothing.
    pub fn plugin(manifest: &str, extra: &str) -> Vec<u8> {
        module(
            manifest,
            &format!(
                r#"(memory (export "memory") 1)
  (func (export "tz_alloc") (param i32) (result i32) (i32.const 1024))
  (func (export "tz_on_block") (param i32 i32) (result i64) (i64.const 0))
  (func (export "tz_run_command") (param i32 i32) (result i64) (i64.const 0))
  {extra}"#
            ),
        )
    }

    pub const MINIMAL: &str = "api = 1\nid = \"hello\"\nname = \"Hello\"\nversion = \"1.0.0\"\npermissions = [\"blocks\"]\n";

    pub fn minimal() -> Vec<u8> {
        plugin(MINIMAL, "")
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::{MINIMAL, minimal, plugin};
    use super::*;
    use termzip_plugin_api::{CommandSpec, LinkSpec};

    fn manifest() -> Manifest {
        toml::from_str(MINIMAL).unwrap()
    }

    fn refused(manifest: Manifest) -> String {
        validate_manifest(&manifest).unwrap_err()
    }

    #[test]
    fn a_plugin_file_is_read_into_its_manifest() {
        let module = read_module(&minimal()).unwrap();

        assert_eq!(module.manifest.id, "hello");
        assert_eq!(module.manifest.permissions, [Permission::Blocks]);
    }

    #[test]
    fn what_is_not_a_plugin_is_explained() {
        assert!(
            read_module(b"")
                .unwrap_err()
                .contains("not a WebAssembly file")
        );
        assert!(
            read_module(b"just text")
                .unwrap_err()
                .contains("not a WebAssembly file")
        );
        // Text format is not accepted: a plugin is compiled.
        assert!(
            read_module(b"(module)")
                .unwrap_err()
                .contains("not a WebAssembly file")
        );
        let truncated = &minimal()[..40];
        assert!(
            read_module(truncated)
                .unwrap_err()
                .contains("not a valid WebAssembly module")
        );
        let huge = vec![0u8; MAX_PLUGIN_BYTES + 1];
        assert!(
            read_module(&huge)
                .unwrap_err()
                .contains("larger than 4 MiB")
        );
    }

    #[test]
    fn a_plugin_without_a_manifest_or_with_two_is_refused() {
        let bare = wat::parse_str("(module (memory (export \"memory\") 1))").unwrap();
        assert!(
            read_module(&bare)
                .unwrap_err()
                .contains("no TermZip manifest")
        );

        let twice = wat::parse_str(
            r#"(module (@custom "termzip-plugin" "a") (@custom "termzip-plugin" "b"))"#,
        )
        .unwrap();
        assert!(
            read_module(&twice)
                .unwrap_err()
                .contains("more than one manifest")
        );
    }

    #[test]
    fn a_manifest_that_cannot_be_read_is_explained() {
        let broken = plugin("api = 1\nid = ", "");
        assert!(
            read_module(&broken)
                .unwrap_err()
                .starts_with("The plugin's manifest is not valid")
        );
        let unknown = plugin(&MINIMAL.replace("blocks", "files"), "");
        assert!(read_module(&unknown).unwrap_err().contains("files"));
        let big = plugin(
            &format!("{MINIMAL}# {}", "x".repeat(MAX_MANIFEST_BYTES)),
            "",
        );
        assert!(
            read_module(&big)
                .unwrap_err()
                .contains("manifest is larger than 16 KiB")
        );
    }

    #[test]
    fn a_plugin_cannot_import_anything_or_be_a_component() {
        // The import has to come before the definitions in text; build it directly.
        let direct = wat::parse_str(
            r#"(module (import "env" "log" (func)) (@custom "termzip-plugin" "x"))"#,
        )
        .unwrap();
        let error = read_module(&direct).unwrap_err();
        assert!(error.contains("imports “env.log”"), "{error}");

        let component = wat::parse_str("(component)").unwrap();
        assert!(
            read_module(&component)
                .unwrap_err()
                .contains("not a component")
        );
    }

    #[test]
    fn the_exports_the_manifest_needs_must_exist() {
        let no_alloc = wat::parse_str(format!(
            r#"(module (@custom "termzip-plugin" "{}") (memory (export "memory") 1))"#,
            MINIMAL
                .bytes()
                .map(|b| format!("\\{b:02x}"))
                .collect::<String>()
        ))
        .unwrap();
        assert!(
            read_module(&no_alloc)
                .unwrap_err()
                .contains("does not export “tz_alloc”")
        );

        // A plugin that watches commands must export tz_on_block; one with commands, tz_run_command.
        let escaped = |text: &str| {
            text.bytes()
                .map(|b| format!("\\{b:02x}"))
                .collect::<String>()
        };
        let without_handler = wat::parse_str(format!(
            r#"(module (@custom "termzip-plugin" "{}") (memory (export "memory") 1)
               (func (export "tz_alloc") (param i32) (result i32) (i32.const 0)))"#,
            escaped(MINIMAL)
        ))
        .unwrap();
        assert!(
            read_module(&without_handler)
                .unwrap_err()
                .contains("tz_on_block")
        );

        let wrong_kind = wat::parse_str(format!(
            r#"(module (@custom "termzip-plugin" "{}") (memory (export "memory") 1)
               (global (export "tz_alloc") i32 (i32.const 0)))"#,
            escaped(MINIMAL)
        ))
        .unwrap();
        assert!(
            read_module(&wrong_kind)
                .unwrap_err()
                .contains("is not a function")
        );
    }

    #[test]
    fn the_identity_is_checked() {
        let mut m = manifest();
        m.api = 2;
        assert!(refused(m).contains("API 2"));

        for bad in [
            "",
            "Hello",
            "has space",
            "-lead",
            "trail-",
            "dou--ble",
            "ünï",
            &"a".repeat(65),
        ] {
            let mut m = manifest();
            m.id = bad.to_owned();
            assert!(refused(m).contains("plugin id"), "{bad:?}");
        }
        let mut m = manifest();
        m.name = "   ".to_owned();
        assert_eq!(refused(m), "The plugin needs a name.");
        let mut m = manifest();
        m.name = "n".repeat(65);
        assert!(refused(m).contains("name is longer"));
        let mut m = manifest();
        m.version = "one".to_owned();
        assert!(refused(m).contains("1.2.3"));
        let mut m = manifest();
        m.homepage = Some("ftp://example.com".to_owned());
        assert!(refused(m).contains("homepage"));
        let mut m = manifest();
        m.description = "d".repeat(281);
        assert!(refused(m).contains("description"));
    }

    #[test]
    fn permissions_must_make_sense() {
        let mut m = manifest();
        m.permissions = vec![Permission::Blocks, Permission::Blocks];
        assert!(refused(m).contains("twice"));

        let mut m = manifest();
        m.permissions = vec![Permission::Output];
        m.links = vec![LinkSpec {
            pattern: "#[0-9]+".to_owned(),
            url: "https://x.test/{0}".to_owned(),
        }];
        assert!(refused(m).contains("needs the “blocks” permission"));

        let mut m = manifest();
        m.permissions = vec![Permission::Blocks, Permission::Type];
        assert!(refused(m).contains("no commands"));

        let mut m = manifest();
        m.permissions = vec![Permission::Blocks, Permission::Open];
        assert!(refused(m).contains("no commands"));
    }

    #[test]
    fn commands_are_checked() {
        let command = |id: &str, title: &str| CommandSpec {
            id: id.to_owned(),
            title: title.to_owned(),
        };
        let mut m = manifest();
        m.commands = vec![command("ok", "Ok"), command("ok", "Again")];
        assert!(refused(m).contains("used twice"));
        let mut m = manifest();
        m.commands = vec![command("Bad Id", "Bad")];
        assert!(refused(m).contains("command id"));
        let mut m = manifest();
        m.commands = vec![command("fine", "  ")];
        assert!(refused(m).contains("needs a title"));
        let mut m = manifest();
        m.commands = (0..=MAX_COMMANDS)
            .map(|n| command(&format!("c{n}"), "C"))
            .collect();
        assert!(refused(m).contains("At most 32 commands"));
    }

    #[test]
    fn link_patterns_and_addresses_are_checked() {
        let link = |pattern: &str, url: &str| LinkSpec {
            pattern: pattern.to_owned(),
            url: url.to_owned(),
        };
        let with = |link: LinkSpec| {
            let mut m = manifest();
            m.links = vec![link];
            validate_manifest(&m)
        };

        assert!(with(link("#([0-9]+)", "https://example.com/issues/{1}")).is_ok());
        assert!(with(link("JIRA-[0-9]+", "https://jira.example.com/browse/{0}")).is_ok());

        assert!(
            with(link("(", "https://x.test"))
                .unwrap_err()
                .contains("not usable")
        );
        assert!(
            with(link("", "https://x.test"))
                .unwrap_err()
                .contains("between 1 and 256")
        );
        assert!(
            with(link(&"a".repeat(257), "https://x.test"))
                .unwrap_err()
                .contains("between 1 and 256")
        );
        // Something that matches the empty string would link every line.
        assert!(
            with(link("x*", "https://x.test"))
                .unwrap_err()
                .contains("matches nothing at all")
        );
        // Look-around and back-references are not supported by the matcher, so the pattern is refused.
        assert!(
            with(link("(?=a)a", "https://x.test"))
                .unwrap_err()
                .contains("not usable")
        );
        assert!(
            with(link(r"(a)\1", "https://x.test"))
                .unwrap_err()
                .contains("not usable")
        );
        // A pattern that would compile to something huge is refused.
        assert!(
            with(link("(a{1000}){1000}", "https://x.test"))
                .unwrap_err()
                .contains("not usable")
        );

        for (url, expected) in [
            ("javascript:alert(1)", "must start with http"),
            ("file:///etc/passwd", "must start with http"),
            ("//x.test", "must start with http"),
            ("https://x.test/a b", "space or a control"),
            ("https://{1}.example.com/", "placeholder in its host"),
            ("https://example.com:{1}/", "placeholder in its host"),
            ("https://x.test/{2}", "has 1 group"),
            ("https://x.test/{a}", "uses {a}"),
            ("https://x.test/{", "unclosed"),
            ("https://x.test/}", "stray"),
        ] {
            let error = with(link("#([0-9]+)", url)).unwrap_err();
            assert!(error.contains(expected), "{url}: {error}");
        }
        let mut m = manifest();
        m.links = (0..=MAX_LINKS)
            .map(|n| link(&format!("t{n}-[0-9]+"), "https://x.test/{0}"))
            .collect();
        assert!(refused(m).contains("At most 16 link patterns"));
    }

    #[test]
    fn a_plugin_that_does_nothing_is_refused() {
        let mut m = manifest();
        m.permissions.clear();
        assert!(refused(m).contains("does nothing"));
    }
}
