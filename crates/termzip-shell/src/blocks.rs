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

#[cfg(test)]
mod tests {
    use super::BlockTracker;
    use crate::marker::{Marker, OscParser};

    #[test]
    fn complete_command_produces_a_block() {
        let mut parser = OscParser::new();
        let mut tracker = BlockTracker::new();

        let bytes = b"\x1b]133;A\x07\x1b]1337;CurrentDir=/tmp\x07$ \x1b]133;B\x07\x1b]1337;CommandB64=Y2QgL2V0Yw==\x07\x1b]133;C\x07log out\x1b]133;D;0\x07";
        let markers = parser.feed(bytes);
        for marker in &markers {
            tracker.on_region_bytes(bytes);
            tracker.on_marker(marker);
        }

        let blocks: Vec<_> = tracker.blocks().iter().cloned().collect();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].command.as_deref(), Some("cd /etc"));
        assert_eq!(blocks[0].cwd.as_deref(), Some("/tmp"));
        assert_eq!(blocks[0].exit_code, Some(0));
        assert!(blocks[0].duration_ms.is_some());
    }

    #[test]
    fn echoed_input_is_the_command_fallback() {
        let mut parser = OscParser::new();
        let mut tracker = BlockTracker::new();

        let bytes = b"\x1b]133;A\x07\x1b]133;B\x07ls -la\x1b]133;C\x07\x1b]133;D;2\x07";
        let markers = parser.feed(bytes);
        for marker in &markers {
            tracker.on_region_bytes(bytes);
            tracker.on_marker(marker);
        }

        let blocks: Vec<_> = tracker.blocks().iter().cloned().collect();
        assert_eq!(blocks[0].command.as_deref(), Some("ls -la"));
        assert_eq!(blocks[0].exit_code, Some(2));
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
