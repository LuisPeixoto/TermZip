//! Link patterns from plugins: text in the terminal output that becomes a link.
//!
//! The patterns are matched with the `regex` crate, which runs in time linear in the line whatever the
//! pattern, so a plugin cannot make the terminal slow by choosing a bad one. The address of a link comes from
//! the plugin's template with the matched text filled in; that text comes from the terminal output, so it is
//! percent-encoded (nothing in it can end the path, start a query or change the host, which a template
//! may not put a placeholder in anyway).

use regex::{Regex, RegexBuilder};
use termzip_plugin_api::{MAX_LINKS, Manifest};

/// Only this much of a line is searched.
const MAX_LINE_BYTES: usize = 8 * 1024;
/// At most this many links are found in one line.
const MAX_MATCHES: usize = 32;
const PATTERN_SIZE_LIMIT: usize = 1 << 20;

struct Rule {
    plugin: String,
    regex: Regex,
    template: String,
}

/// A link found in a line. `start` and `end` count UTF-16 units, which is how the webview indexes its strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkMatch {
    pub plugin: String,
    pub start: usize,
    pub end: usize,
    pub url: String,
}

#[derive(Default)]
pub struct LinkMatcher {
    rules: Vec<Rule>,
}

/// Everything but the unreserved characters is encoded, including `/`, so a match is always one path segment.
fn encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// Fills `{0}`, `{1}`… of the template with the (encoded) matched text. A group that did not take part in the
/// match is empty.
fn fill(template: &str, captures: &regex::Captures<'_>) -> String {
    let mut out = String::with_capacity(template.len() + 16);
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let inner = &rest[open + 1..];
        let Some(close) = inner.find('}') else {
            out.push_str(&rest[open..]);
            return out;
        };
        match inner[..close].parse::<usize>() {
            Ok(n) => out.push_str(
                &captures
                    .get(n)
                    .map_or(String::new(), |m| encode(m.as_str())),
            ),
            Err(_) => out.push_str(&rest[open..open + close + 2]),
        }
        rest = &inner[close + 1..];
    }
    out.push_str(rest);
    out
}

impl LinkMatcher {
    /// The rules of these plugins (the enabled ones). A pattern that does not compile is skipped: manifests are
    /// checked when a plugin is installed, so that only happens to a file that was changed since.
    pub fn new<'a>(manifests: impl IntoIterator<Item = &'a Manifest>) -> Self {
        let mut rules = Vec::new();
        for manifest in manifests {
            for link in manifest.links.iter().take(MAX_LINKS) {
                if let Ok(regex) = RegexBuilder::new(&link.pattern)
                    .size_limit(PATTERN_SIZE_LIMIT)
                    .build()
                {
                    rules.push(Rule {
                        plugin: manifest.id.clone(),
                        regex,
                        template: link.url.clone(),
                    });
                }
            }
        }
        Self { rules }
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// The links in a line, left to right. Where two matches overlap the earlier one wins, and a rule listed
    /// first wins a tie.
    pub fn find(&self, line: &str) -> Vec<LinkMatch> {
        if self.rules.is_empty() {
            return Vec::new();
        }
        let mut end = line.len().min(MAX_LINE_BYTES);
        while !line.is_char_boundary(end) {
            end -= 1;
        }
        let line = &line[..end];

        let mut found: Vec<(usize, usize, usize, String)> = Vec::new(); // byte start, byte end, rule, url
        for (position, rule) in self.rules.iter().enumerate() {
            for captures in rule.regex.captures_iter(line).take(MAX_MATCHES) {
                let whole = captures.get(0).expect("group 0 is the whole match");
                found.push((
                    whole.start(),
                    whole.end(),
                    position,
                    fill(&rule.template, &captures),
                ));
            }
        }
        found.sort_by_key(|(start, end, position, _)| (*start, std::cmp::Reverse(*end), *position));

        let mut links: Vec<LinkMatch> = Vec::new();
        let mut taken_until = 0;
        for (start, stop, position, url) in found {
            if start < taken_until || links.len() >= MAX_MATCHES {
                continue;
            }
            taken_until = stop;
            links.push(LinkMatch {
                plugin: self.rules[position].plugin.clone(),
                start: line[..start].encode_utf16().count(),
                end: line[..stop].encode_utf16().count(),
                url,
            });
        }
        links
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use termzip_plugin_api::{LinkSpec, Permission};

    fn plugin(id: &str, rules: &[(&str, &str)]) -> Manifest {
        Manifest {
            api: 1,
            id: id.to_owned(),
            name: id.to_owned(),
            version: "1.0.0".to_owned(),
            description: String::new(),
            author: None,
            license: None,
            homepage: None,
            permissions: vec![Permission::Blocks],
            commands: Vec::new(),
            links: rules
                .iter()
                .map(|(pattern, url)| LinkSpec {
                    pattern: (*pattern).to_owned(),
                    url: (*url).to_owned(),
                })
                .collect(),
        }
    }

    fn matcher(rules: &[(&str, &str)]) -> LinkMatcher {
        LinkMatcher::new(&[plugin("p", rules)])
    }

    #[test]
    fn a_pattern_becomes_a_link_with_its_groups_in_the_address() {
        let matcher = matcher(&[
            ("#([0-9]+)", "https://github.com/me/repo/issues/{1}"),
            ("JIRA-[0-9]+", "https://jira.example.com/browse/{0}"),
        ]);

        let links = matcher.find("fixes #42 and JIRA-7");
        assert_eq!(links.len(), 2);
        assert_eq!(
            links[0],
            LinkMatch {
                plugin: "p".to_owned(),
                start: 6,
                end: 9,
                url: "https://github.com/me/repo/issues/42".to_owned()
            }
        );
        assert_eq!((links[1].start, links[1].end), (14, 20));
        assert_eq!(links[1].url, "https://jira.example.com/browse/JIRA-7");
    }

    #[test]
    fn text_from_the_terminal_cannot_change_where_a_link_goes() {
        let matcher = matcher(&[(r"id=(\S+)", "https://example.com/item/{1}?ref=term")]);

        for (line, encoded) in [
            ("id=a/b", "a%2Fb"),
            ("id=../../admin", "..%2F..%2Fadmin"),
            ("id=x?y#z", "x%3Fy%23z"),
            ("id=100%", "100%25"),
            ("id=évil", "%C3%A9vil"),
            ("id=a@evil.com", "a%40evil.com"),
            ("id=\u{202e}rtl", "%E2%80%AErtl"),
        ] {
            let links = matcher.find(line);
            assert_eq!(
                links[0].url,
                format!("https://example.com/item/{encoded}?ref=term"),
                "{line}"
            );
        }
    }

    #[test]
    fn a_group_that_did_not_take_part_is_empty() {
        let matcher = matcher(&[("ticket(-([0-9]+))?", "https://t.example/{0}/{2}")]);
        let links = matcher.find("ticket and ticket-9");

        assert_eq!(links[0].url, "https://t.example/ticket/");
        assert_eq!(links[1].url, "https://t.example/ticket-9/9");
    }

    #[test]
    fn positions_count_utf16_units_like_the_webview_does() {
        let matcher = matcher(&[("#([0-9]+)", "https://x.test/{1}")]);

        // "é" is one unit, the emoji is two (a surrogate pair) but four bytes.
        let links = matcher.find("é😀 #12");
        assert_eq!((links[0].start, links[0].end), (4, 7));
        let line = "é😀 #12";
        let units: Vec<u16> = line.encode_utf16().collect();
        assert_eq!(
            String::from_utf16(&units[links[0].start..links[0].end]).unwrap(),
            "#12"
        );
    }

    #[test]
    fn overlapping_matches_keep_the_earlier_and_then_the_longer() {
        let matcher = matcher(&[
            ("abc", "https://x.test/short"),
            ("abcdef", "https://x.test/long"),
            ("cde", "https://x.test/mid"),
        ]);

        let links = matcher.find("abcdef");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].url, "https://x.test/long");
    }

    #[test]
    fn several_plugins_share_the_line() {
        let a = plugin("a", &[("#([0-9]+)", "https://a.test/{1}")]);
        let b = plugin("b", &[("BUG-([0-9]+)", "https://b.test/{1}")]);
        let matcher = LinkMatcher::new([&a, &b]);

        let links = matcher.find("#1 BUG-2");
        assert_eq!(
            links.iter().map(|l| l.plugin.as_str()).collect::<Vec<_>>(),
            ["a", "b"]
        );
    }

    #[test]
    fn a_pathological_pattern_cannot_slow_a_line_down() {
        let matcher = matcher(&[("(a+)+$", "https://x.test/{0}")]);
        let line = format!("{}!", "a".repeat(8000));

        let started = std::time::Instant::now();
        let links = matcher.find(&line);
        assert!(
            started.elapsed() < std::time::Duration::from_secs(2),
            "{:?}",
            started.elapsed()
        );
        assert!(links.is_empty());
    }

    #[test]
    fn long_lines_and_many_matches_are_bounded() {
        let matcher = matcher(&[("x", "https://x.test/{0}")]);

        assert_eq!(matcher.find(&"x ".repeat(5000)).len(), MAX_MATCHES);
        // Only the start of an enormous line is searched, never cut inside a character.
        let line = format!("{}é#9", "é".repeat(MAX_LINE_BYTES));
        let tail = LinkMatcher::new(&[plugin("p", &[("#9", "https://x.test/{0}")])]);
        assert!(tail.find(&line).is_empty());
    }

    #[test]
    fn nothing_matches_without_rules_and_broken_ones_are_skipped() {
        assert!(LinkMatcher::default().find("#1").is_empty());
        assert!(LinkMatcher::default().is_empty());

        let matcher = matcher(&[
            ("(", "https://x.test/"),
            ("#([0-9]+)", "https://x.test/{1}"),
        ]);
        assert_eq!(matcher.find("#5").len(), 1);
    }
}
