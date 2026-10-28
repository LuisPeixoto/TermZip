//! OSC parser for the semantic-prompt protocol and TermZip's private markers.

use base64::Engine;

const TERMINATOR_BEL: u8 = 0x07;
const MAX_PENDING_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Marker {
    /// `133;A` — start of the prompt region.
    PromptStart,
    /// `133;B` — end of the prompt, start of the command input region.
    InputStart,
    /// `133;C` — the command is running; output region starts.
    ExecStart,
    /// `133;D[;<code>]` — the command finished.
    ExecEnd(Option<i32>),
    /// `1337;CurrentDir=<path>` — the shell's working directory.
    CurrentDir(String),
    /// `1337;CommandB64=<b64>` or `1337;Command=<text>`.
    Command(Option<String>),
}

/// A marker with the byte range it occupied inside the chunk passed to
/// [`OscParser::feed_ranges`]. Offsets are relative to the current chunk; a
/// marker that began in an earlier chunk starts at 0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkerHit {
    pub start: usize,
    pub end: usize,
    pub marker: Marker,
}

/// Incremental OSC marker scanner. Bytes are never consumed for output — the
/// session forwards the raw PTY stream to the terminal as-is; this type only
/// reports the markers it finds.
#[derive(Debug, Default)]
pub struct OscParser {
    pending: Vec<u8>,
}

impl OscParser {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed bytes and return every complete marker found. Incomplete trailing
    /// sequences are kept for the next call.
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<Marker> {
        self.feed_ranges(bytes)
            .into_iter()
            .map(|hit| hit.marker)
            .collect()
    }

    /// Like [`Self::feed`], but also reports where each marker started and ended
    /// in `bytes`, so callers can attribute the non-marker bytes to regions.
    pub fn feed_ranges(&mut self, bytes: &[u8]) -> Vec<MarkerHit> {
        let prior_len = self.pending.len();
        self.pending.extend_from_slice(bytes);
        if self.pending.len() > MAX_PENDING_BYTES {
            let overflow = self.pending.len() - MAX_PENDING_BYTES;
            self.pending.drain(..overflow);
        }

        let mut hits = Vec::new();
        let mut search_from = 0;
        while let Some(start) = find_intro(&self.pending, search_from) {
            let Some(end) = find_terminator(&self.pending, start) else {
                break;
            };

            let body = &self.pending[start + INTRO_LEN..end];
            let terminator = terminator_len(&self.pending, end);
            let marker_end = end + terminator;
            if let Some(marker) = parse_body(body) {
                // Only markers whose terminator arrived in this chunk can affect
                // the current chunk's byte ranges.
                if marker_end > prior_len {
                    let start_in_chunk = start.saturating_sub(prior_len);
                    hits.push(MarkerHit {
                        start: start_in_chunk,
                        end: marker_end - prior_len,
                        marker,
                    });
                }
            }
            search_from = marker_end;
        }

        if search_from > 0 {
            self.pending.drain(..search_from);
        }
        hits
    }
}

const INTRO: &[u8] = b"\x1b]";
const INTRO_LEN: usize = INTRO.len();

fn find_intro(haystack: &[u8], from: usize) -> Option<usize> {
    haystack[from..]
        .windows(INTRO_LEN)
        .position(|window| window == INTRO)
        .map(|index| index + from)
}

fn find_terminator(haystack: &[u8], from: usize) -> Option<usize> {
    let mut index = from;
    while index < haystack.len() {
        if haystack[index] == TERMINATOR_BEL {
            return Some(index);
        }
        if haystack[index] == 0x1b && haystack.get(index + 1) == Some(&b'\\') {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn terminator_len(haystack: &[u8], terminator: usize) -> usize {
    if haystack.get(terminator + 1) == Some(&b'\\') {
        2
    } else {
        1
    }
}

fn parse_body(body: &[u8]) -> Option<Marker> {
    let text = String::from_utf8_lossy(body);
    let mut parts = text.split(';');
    match parts.next()? {
        "133" => match parts.next()? {
            "A" => Some(Marker::PromptStart),
            "B" => Some(Marker::InputStart),
            "C" => Some(Marker::ExecStart),
            "D" => {
                // The exit code is the final parameter; `aid=` prefixes tolerated.
                let code = text
                    .rsplit(';')
                    .next()
                    .and_then(|last| last.parse::<i32>().ok());
                Some(Marker::ExecEnd(code))
            }
            _ => None,
        },
        "1337" => {
            let (_, rest) = text.split_once(';')?;
            let (kind, payload) = rest.split_once('=')?;
            match kind {
                "CurrentDir" => {
                    (!payload.is_empty()).then(|| Marker::CurrentDir(payload.to_owned()))
                }
                "CommandB64" => {
                    let decoded = base64::engine::general_purpose::STANDARD
                        .decode(payload.trim())
                        .ok()
                        .map(|bytes| String::from_utf8_lossy(&bytes).replace('\n', " "));
                    Some(Marker::Command(decoded))
                }
                "Command" => Some(Marker::Command(Some(payload.to_owned()))),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Reduce a captured byte region to plain text by removing escape sequences.
pub fn strip_ansi(bytes: &[u8]) -> String {
    // Work on bytes and decode once at the end: a multibyte character is several
    // bytes and must not be widened one byte at a time.
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut chars = bytes.iter().copied().peekable();
    while let Some(byte) = chars.next() {
        match byte {
            0x1b => {
                if chars.peek() == Some(&b'[') {
                    chars.next();
                    for next in chars.by_ref() {
                        if (0x40..=0x7e).contains(&next) {
                            break;
                        }
                    }
                } else if chars.peek() == Some(&b']') {
                    chars.next();
                    while let Some(&next) = chars.peek() {
                        chars.next();
                        if next == TERMINATOR_BEL {
                            break;
                        }
                        if next == 0x1b && chars.peek() == Some(&b'\\') {
                            chars.next();
                            break;
                        }
                    }
                }
            }
            0x07 => {}
            0x08 | 0x7f => pop_char(&mut out),
            0x0d => {}
            _ => out.push(byte),
        }
    }
    String::from_utf8_lossy(&out).trim().to_owned()
}

/// Removes the last character, which is the last byte plus its continuation bytes.
fn pop_char(out: &mut Vec<u8>) {
    while out.pop().is_some_and(|byte| (0x80..0xc0).contains(&byte)) {}
}

#[cfg(test)]
mod tests {
    use base64::Engine as _;

    use super::{Marker, MarkerHit, OscParser, strip_ansi};

    fn feed_all(parser: &mut OscParser, chunks: &[&[u8]]) -> Vec<Marker> {
        let mut markers = Vec::new();
        for chunk in chunks {
            markers.extend(parser.feed(chunk));
        }
        markers
    }

    #[test]
    fn semantic_prompt_markers_are_extracted() {
        let mut parser = OscParser::new();
        let markers = parser.feed(
            b"\x1b]133;A\x07 prompt \x1b]133;B\x07 input \x1b]133;C\x07 out \x1b]133;D;3\x07",
        );

        assert_eq!(
            markers,
            vec![
                Marker::PromptStart,
                Marker::InputStart,
                Marker::ExecStart,
                Marker::ExecEnd(Some(3)),
            ]
        );
    }

    #[test]
    fn sequences_split_across_reads_are_still_found() {
        let mut parser = OscParser::new();
        let markers = feed_all(
            &mut parser,
            &[b"\x1b]1", b"33;C\x1b", b"\\hello\x1b]133;D;0\x07"],
        );

        assert_eq!(markers, vec![Marker::ExecStart, Marker::ExecEnd(Some(0))]);
    }

    #[test]
    fn marker_ranges_split_output_regions() {
        let mut parser = OscParser::new();
        let chunk = b"prompt\x1b]133;C\x07output\x1b]133;D;0\x07";
        let hits = parser.feed_ranges(chunk);

        assert_eq!(
            hits[0],
            MarkerHit {
                start: 6,
                end: 14,
                marker: Marker::ExecStart,
            }
        );
        assert_eq!(hits[1].start, 20);
        assert_eq!(hits[1].end, chunk.len());
        // Bytes between the two markers are exactly the output region.
        assert_eq!(&chunk[hits[0].end..hits[1].start], b"output");
    }

    #[test]
    fn markers_starting_in_a_previous_chunk_start_at_zero() {
        let mut parser = OscParser::new();
        let first = parser.feed_ranges(b"\x1b]133;");
        assert!(first.is_empty());
        let hits = parser.feed_ranges(b"C\x07data");

        assert_eq!(hits[0].start, 0);
        assert_eq!(hits[0].end, 2);
        assert_eq!(&b"C\x07data"[2..], b"data");
    }

    #[test]
    fn aid_parameters_are_tolerated() {
        let mut parser = OscParser::new();
        let markers = parser.feed(b"\x1b]133;A;aid=42\x07\x1b]133;D;aid=x;7\x07");

        assert_eq!(markers, vec![Marker::PromptStart, Marker::ExecEnd(Some(7))]);
    }

    #[test]
    fn base64_commands_are_decoded_and_flattened() {
        let mut parser = OscParser::new();
        let encoded = base64::engine::general_purpose::STANDARD.encode("echo hi\nthere");
        let markers = parser.feed(format!("\x1b]1337;CommandB64={encoded}\x07").as_bytes());

        assert_eq!(
            markers,
            vec![Marker::Command(Some("echo hi there".to_owned()))]
        );
    }

    #[test]
    fn current_dir_is_reported() {
        let mut parser = OscParser::new();
        let markers = parser.feed(b"\x1b]1337;CurrentDir=/tmp/work\x07");

        assert_eq!(markers, vec![Marker::CurrentDir("/tmp/work".to_owned())]);
    }

    #[test]
    fn strip_ansi_removes_sequences_but_keeps_text() {
        let text = strip_ansi(b"\x1b[38;2;10;20;30mecho \x1b[0m\x1b]133;C\x07done\x08\r\n");

        assert_eq!(text, "echo don");
    }

    #[test]
    fn strip_ansi_keeps_multibyte_text_intact() {
        let bytes = "echo h\u{e9}llo \u{4f60}\u{597d} \u{1f680}".as_bytes();
        assert_eq!(
            strip_ansi(bytes),
            "echo h\u{e9}llo \u{4f60}\u{597d} \u{1f680}"
        );
        let colored = b"\x1b[31mcaf\xc3\xa9\x1b[0m";
        assert_eq!(strip_ansi(colored), "caf\u{e9}");
    }

    #[test]
    fn strip_ansi_survives_a_character_cut_in_half() {
        // A capture limit can end in the middle of a multibyte character.
        let cut = &"\u{e9}".as_bytes()[..1];
        assert_eq!(strip_ansi(cut), "\u{fffd}");
    }
}
