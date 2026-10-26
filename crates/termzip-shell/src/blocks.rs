//! Command-block tracking on top of the OSC markers.

use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::marker::strip_ansi;

const MAX_STORED_BLOCKS: usize = 200;
const MAX_CAPTURED_BYTES: usize = 128 * 1024;
const MAX_STORED_OUTPUT_CHARS: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Block {
    pub index: usize,
    pub command: Option<String>,
    pub cwd: Option<String>,
    pub exit_code: Option<i32>,
    pub started_at_ms: Option<u64>,
    pub duration_ms: Option<u64>,
    pub output_chars: usize,
    pub truncated: bool,
    pub output: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Region {
    Idle,
    Prompt,
    Input,
    Output,
}

#[derive(Debug)]
struct Running {
    command: Option<String>,
    input: Vec<u8>,
    output_bytes: usize,
    output_head: Vec<u8>,
    cwd: Option<String>,
    started_at_ms: Option<u64>,
}

impl Running {
    fn new() -> Self {
        Self {
            command: None,
            input: Vec::new(),
            output_bytes: 0,
            output_head: Vec::new(),
            cwd: None,
            started_at_ms: None,
        }
    }
}

/// Turns the marker stream into finished [`Block`] entries. Region bytes are fed
/// in separately so echoed input and command output can be captured as text.
#[derive(Debug)]
pub struct BlockTracker {
    region: Region,
    running: Option<Running>,
    cwd: Option<String>,
    blocks: VecDeque<Block>,
    next_index: usize,
}

impl Default for BlockTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockTracker {
    pub fn new() -> Self {
        Self {
            region: Region::Idle,
            running: None,
            cwd: None,
            blocks: VecDeque::new(),
            next_index: 0,
        }
    }

    pub fn blocks(&self) -> &VecDeque<Block> {
        &self.blocks
    }

    pub fn cwd(&self) -> Option<&str> {
        self.cwd.as_deref()
    }

    pub fn on_marker(&mut self, marker: &crate::marker::Marker) {
        use crate::marker::Marker;
        match marker {
            Marker::PromptStart => {
                self.region = Region::Prompt;
            }
            Marker::InputStart => {
                self.region = Region::Input;
                self.running.get_or_insert_with(Running::new);
            }
            Marker::ExecStart => {
                self.region = Region::Output;
                let running = self.running.get_or_insert_with(Running::new);
                running.started_at_ms = Some(now_ms());
                running.cwd = self.cwd.clone();
            }
            Marker::ExecEnd(code) => {
                self.region = Region::Idle;
                let running = self.running.take().unwrap_or_else(|| {
                    let mut fresh = Running::new();
                    fresh.cwd = self.cwd.clone();
                    fresh
                });
                let command = running.command.clone().or_else(|| {
                    let text = strip_ansi(&running.input);
                    (!text.is_empty()).then_some(text)
                });
                let started_at_ms = running.started_at_ms;
                let output = if running.output_head.is_empty() {
                    None
                } else {
                    let raw = String::from_utf8_lossy(&running.output_head).into_owned();
                    let cleaned = trim_prompt_eol_marker(raw);
                    (!cleaned.is_empty()).then_some(cleaned)
                };
                self.blocks.push_back(Block {
                    index: self.next_index,
                    command,
                    cwd: running.cwd,
                    exit_code: *code,
                    started_at_ms,
                    duration_ms: started_at_ms.map(|started| now_ms().saturating_sub(started)),
                    output_chars: running.output_bytes,
                    truncated: running.output_bytes > MAX_CAPTURED_BYTES
                        || running.output_head.len() >= MAX_STORED_OUTPUT_CHARS,
                    output,
                });
                self.next_index += 1;
                while self.blocks.len() > MAX_STORED_BLOCKS {
                    self.blocks.pop_front();
                }
            }
            Marker::CurrentDir(path) => {
                self.cwd = Some(path.clone());
            }
            Marker::Command(text) => {
                if let Some(running) = self.running.as_mut()
                    && running.command.is_none()
                {
                    running.command = text.clone();
                }
            }
        }
    }

    pub fn on_region_bytes(&mut self, bytes: &[u8]) {
        match self.region {
            Region::Input => {
                if let Some(running) = self.running.as_mut()
                    && running.input.len() < MAX_CAPTURED_BYTES
                {
                    running
                        .input
                        .extend(bytes.iter().take(MAX_CAPTURED_BYTES - running.input.len()));
                }
            }
            Region::Output => {
                if let Some(running) = self.running.as_mut() {
                    running.output_bytes += bytes.len();
                    let room = MAX_STORED_OUTPUT_CHARS.saturating_sub(running.output_head.len());
                    running.output_head.extend(bytes.iter().take(room));
                }
            }
            _ => {}
        }
    }

    /// Plain-text snapshot of a running block's captured output head.
    pub fn running_output_head(&self) -> Option<String> {
        let running = self.running.as_ref()?;
        let text = strip_ansi(&running.output_head);
        (!text.is_empty()).then_some(text)
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or_default()
}

/// zsh's `PROMPT_SP` appends a `%` end-of-line marker before `precmd`; it is a
/// prompt artifact, not command output. Remove it only when it is the final
/// line by itself, so real `%` output is preserved.
fn trim_prompt_eol_marker(output: String) -> String {
    let trimmed = output.trim_end_matches([' ', '\r', '\n', '\t']);
    let last_line_start = trimmed.rfind('\n').map(|index| index + 1).unwrap_or(0);
    let last_line = &trimmed[last_line_start..];
    if strip_ansi(last_line.as_bytes()).trim() == "%" {
        output[..last_line_start].to_owned()
    } else {
        output
    }
}

#[cfg(test)]
mod tests {
    use super::BlockTracker;
    use crate::marker::{Marker, OscParser};

    fn feed(parser: &mut OscParser, tracker: &mut BlockTracker, chunk: &[u8]) {
        let hits = parser.feed_ranges(chunk);
        let mut cursor = 0;
        for hit in hits {
            if hit.start > cursor {
                tracker.on_region_bytes(&chunk[cursor..hit.start]);
            }
            tracker.on_marker(&hit.marker);
            cursor = hit.end;
        }
        if cursor < chunk.len() {
            tracker.on_region_bytes(&chunk[cursor..]);
        }
    }

    #[test]
    fn complete_command_produces_a_block() {
        let mut parser = OscParser::new();
        let mut tracker = BlockTracker::new();

        let bytes = b"\x1b]133;A\x07\x1b]1337;CurrentDir=/tmp\x07$ \x1b]133;B\x07\x1b]1337;CommandB64=Y2QgL2V0Yw==\x07\x1b]133;C\x07log out\x1b]133;D;0\x07";
        feed(&mut parser, &mut tracker, bytes);

        let blocks: Vec<_> = tracker.blocks().iter().cloned().collect();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].command.as_deref(), Some("cd /etc"));
        assert_eq!(blocks[0].cwd.as_deref(), Some("/tmp"));
        assert_eq!(blocks[0].exit_code, Some(0));
        assert!(blocks[0].duration_ms.is_some());
        assert_eq!(blocks[0].output.as_deref(), Some("log out"));
    }

    #[test]
    fn echoed_input_is_the_command_fallback() {
        let mut parser = OscParser::new();
        let mut tracker = BlockTracker::new();

        let bytes = b"\x1b]133;A\x07\x1b]133;B\x07ls -la\x1b]133;C\x07\x1b]133;D;2\x07";
        feed(&mut parser, &mut tracker, bytes);

        let blocks: Vec<_> = tracker.blocks().iter().cloned().collect();
        assert_eq!(blocks[0].command.as_deref(), Some("ls -la"));
        assert_eq!(blocks[0].exit_code, Some(2));
        assert!(blocks[0].output.is_none());
    }

    #[test]
    fn output_capture_stops_at_the_next_prompt() {
        let mut parser = OscParser::new();
        let mut tracker = BlockTracker::new();

        feed(
            &mut parser,
            &mut tracker,
            b"\x1b]133;A\x07\x1b]133;B\x07echo hi\x1b]133;C\x07hi\r\n\x1b]133;D;0\x07\x1b]133;A\x07prompt",
        );

        let blocks: Vec<_> = tracker.blocks().iter().cloned().collect();
        assert_eq!(blocks[0].command.as_deref(), Some("echo hi"));
        assert_eq!(blocks[0].output.as_deref(), Some("hi\r\n"));
    }

    #[test]
    fn zsh_prompt_eol_marker_is_trimmed() {
        let mut parser = OscParser::new();
        let mut tracker = BlockTracker::new();

        feed(
            &mut parser,
            &mut tracker,
            b"\x1b]133;A\x07\x1b]133;B\x07echo hi\x1b]133;C\x07hi\r\n\x1b[1m\x1b[7m%\x1b[27m\r\n\x1b]133;D;0\x07",
        );

        let blocks: Vec<_> = tracker.blocks().iter().cloned().collect();
        assert_eq!(blocks[0].output.as_deref(), Some("hi\r\n"));
    }

    #[test]
    fn exec_end_without_start_still_records() {
        let mut parser = OscParser::new();
        let mut tracker = BlockTracker::new();

        for marker in parser.feed(b"\x1b]133;D;127\x07") {
            tracker.on_marker(&marker);
        }

        assert_eq!(tracker.blocks().len(), 1);
        assert_eq!(tracker.blocks()[0].exit_code, Some(127));
        assert!(tracker.blocks()[0].command.is_none());
    }

    #[test]
    fn old_blocks_are_evicted() {
        let mut parser = OscParser::new();
        let mut tracker = BlockTracker::new();
        let one = b"\x1b]133;A\x07\x1b]133;B\x07\x1b]133;C\x07\x1b]133;D;0\x07";

        for _ in 0..250 {
            for marker in parser.feed(one) {
                tracker.on_marker(&marker);
            }
        }

        assert_eq!(tracker.blocks().len(), 200);
        assert_eq!(tracker.blocks().front().unwrap().index, 50);
        assert_eq!(tracker.blocks().back().unwrap().index, 249);
    }

    #[test]
    fn marker_enum_is_constructible_for_docs() {
        // Keeps the import meaningful; the tracker consumes references.
        let _ = Marker::PromptStart;
    }
}
