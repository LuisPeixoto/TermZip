//! Session lifecycle: PTY spawning, the reader loop, marker parsing, and block
//! assembly. One session owns exactly one shell process.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use base64::Engine;
use serde::Serialize;
use tauri::ipc::Channel;

use termzip_pty::{PtyOptions, PtySession};
use termzip_shell::{Block, BlockTracker, OscParser};

const EXIT_POLL: Duration = Duration::from_millis(400);
/// After the PTY closes, the child may need a moment before its status is
/// available: wait at most `EXIT_GRACE_POLLS * EXIT_GRACE_STEP` for it.
const EXIT_GRACE_POLLS: u32 = 20;
const EXIT_GRACE_STEP: Duration = Duration::from_millis(50);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub id: u32,
    pub shell: String,
    pub integrated: bool,
}

/// Everything a session tells the webview, in order, on one channel. The
/// channel belongs to a single session, so events carry no session id and
/// cannot arrive before the terminal is ready for them.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SessionEvent {
    /// Raw terminal output, base64 encoded.
    Data { data: String },
    /// The shell ended; `code` is `None` when the status could not be read.
    Exit { code: Option<u32> },
    /// One command finished. Sent once per block, so a busy shell never re-sends the whole list.
    Block { block: Block },
    /// The shell reported a new working directory.
    Cwd { path: String },
}

pub struct SessionHandle {
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    master: Mutex<Box<dyn termzip_pty::MasterPty + Send>>,
    child: Mutex<Box<dyn termzip_pty::Child + Send + Sync>>,
    parser: Mutex<OscParser>,
    tracker: Mutex<BlockTracker>,
    destroyed: AtomicBool,
}

impl SessionHandle {
    pub fn write(&self, bytes: &[u8]) -> anyhow::Result<()> {
        let mut writer = self
            .writer
            .lock()
            .map_err(|_| anyhow::anyhow!("PTY writer lock poisoned"))?;
        writer.write_all(bytes)?;
        writer.flush()?;
        Ok(())
    }

    pub fn resize(&self, rows: u16, columns: u16) -> anyhow::Result<()> {
        let master = self
            .master
            .lock()
            .map_err(|_| anyhow::anyhow!("PTY master lock poisoned"))?;
        master.resize(termzip_pty::PtySize {
            rows: rows.max(1),
            cols: columns.max(1),
            pixel_width: 0,
            pixel_height: 0,
        })?;
        Ok(())
    }

    fn try_exit(&self) -> anyhow::Result<Option<termzip_pty::ExitStatus>> {
        self.child
            .lock()
            .map_err(|_| anyhow::anyhow!("child lock poisoned"))?
            .try_wait()
            .map_err(Into::into)
    }

    fn kill(&self) {
        self.destroyed.store(true, Ordering::Relaxed);
        if let Ok(mut child) = self.child.lock() {
            let _ = child.kill();
        }
    }
}

#[derive(Default)]
pub struct Sessions {
    inner: Mutex<HashMap<u32, Arc<SessionHandle>>>,
    next_id: AtomicU32,
}

impl Sessions {
    fn allocate_id(&self) -> u32 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    pub fn create(
        &self,
        events: Channel<SessionEvent>,
        rows: u16,
        columns: u16,
        working_directory: Option<String>,
        shell_override: Option<String>,
        integration_dir: &Path,
    ) -> anyhow::Result<SessionInfo> {
        let launch = termzip_shell::launch_config(shell_override.as_deref(), integration_dir);
        let environment: BTreeMap<String, String> = launch.env.iter().cloned().collect();
        let options = PtyOptions {
            shell: Some(launch.program.clone()),
            args: launch.args.clone(),
            working_directory: working_directory.map(std::path::PathBuf::from),
            rows,
            columns,
            environment,
        };

        let (session, receiver) = PtySession::spawn(options, || {})?;
        let id = self.allocate_id();
        let info = SessionInfo {
            id,
            shell: launch
                .program
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| "shell".to_owned()),
            integrated: launch.integrated,
        };

        let parts = session.into_parts();
        let handle = Arc::new(SessionHandle {
            writer: parts.writer,
            master: Mutex::new(parts.master),
            child: Mutex::new(parts.child),
            parser: Mutex::new(OscParser::new()),
            tracker: Mutex::new(BlockTracker::new()),
            destroyed: AtomicBool::new(false),
        });

        match self.inner.lock() {
            Ok(mut sessions) => {
                sessions.insert(id, handle.clone());
            }
            Err(_) => {
                // Do not leave a shell running that nobody can reach.
                handle.kill();
                anyhow::bail!("sessions lock poisoned");
            }
        }

        spawn_reader(events, handle, receiver);
        Ok(info)
    }

    pub fn get(&self, id: u32) -> anyhow::Result<Arc<SessionHandle>> {
        self.inner
            .lock()
            .map_err(|_| anyhow::anyhow!("sessions lock poisoned"))?
            .get(&id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("session {id} is no longer available"))
    }

    pub fn destroy(&self, id: u32) {
        if let Ok(mut map) = self.inner.lock()
            && let Some(handle) = map.remove(&id)
        {
            handle.kill();
        }
    }

    pub fn destroy_all(&self) {
        if let Ok(mut map) = self.inner.lock() {
            for handle in map.values() {
                handle.kill();
            }
            map.clear();
        }
    }

    pub fn blocks_snapshot(&self, id: u32) -> anyhow::Result<Vec<Block>> {
        let handle = self.get(id)?;
        let tracker = handle
            .tracker
            .lock()
            .map_err(|_| anyhow::anyhow!("tracker lock poisoned"))?;
        Ok(tracker.blocks().iter().cloned().collect())
    }
}

fn spawn_reader(
    events: Channel<SessionEvent>,
    handle: Arc<SessionHandle>,
    receiver: Receiver<Vec<u8>>,
) {
    let _ = thread::Builder::new()
        .name("termzip-session-reader".to_owned())
        .spawn(move || {
            let code = loop {
                if handle.destroyed.load(Ordering::Relaxed) {
                    // The webview asked for this; it is not waiting for an exit.
                    return;
                }
                match receiver.recv_timeout(EXIT_POLL) {
                    Ok(chunk) => handle_chunk(&events, &handle, &chunk),
                    Err(RecvTimeoutError::Timeout) => match handle.try_exit() {
                        Ok(None) => {}
                        Ok(Some(status)) => {
                            // Output written just before the exit may still be queued.
                            while let Ok(chunk) = receiver.try_recv() {
                                handle_chunk(&events, &handle, &chunk);
                            }
                            break Some(status.exit_code());
                        }
                        // A poisoned lock never recovers: report and stop instead of spinning.
                        Err(_) => break None,
                    },
                    Err(RecvTimeoutError::Disconnected) => break wait_for_exit(&handle),
                }
            };
            if !handle.destroyed.load(Ordering::Relaxed) {
                let _ = events.send(SessionEvent::Exit { code });
            }
        });
}

/// The PTY closed, so the shell is ending; its status can lag by a moment.
fn wait_for_exit(handle: &SessionHandle) -> Option<u32> {
    for _ in 0..EXIT_GRACE_POLLS {
        match handle.try_exit() {
            Ok(Some(status)) => return Some(status.exit_code()),
            Ok(None) => thread::sleep(EXIT_GRACE_STEP),
            Err(_) => return None,
        }
    }
    None
}

fn handle_chunk(events: &Channel<SessionEvent>, handle: &Arc<SessionHandle>, chunk: &[u8]) {
    let _ = events.send(SessionEvent::Data {
        data: base64::engine::general_purpose::STANDARD.encode(chunk),
    });

    let Ok(mut parser) = handle.parser.lock() else {
        return;
    };
    let Ok(mut tracker) = handle.tracker.lock() else {
        return;
    };
    for event in apply_chunk(&mut parser, &mut tracker, chunk) {
        let _ = events.send(event);
    }
}

/// Feeds one chunk of output through the marker parser and the block tracker, and returns what
/// changed: each newly finished block, and the working directory when it moved.
fn apply_chunk(
    parser: &mut OscParser,
    tracker: &mut BlockTracker,
    chunk: &[u8],
) -> Vec<SessionEvent> {
    // Blocks are numbered for the life of the session, so "new" means a higher index. Comparing
    // lengths would miss them once the oldest blocks start being dropped.
    let last_index = tracker.blocks().back().map(|block| block.index);
    let previous_cwd = tracker.cwd().map(str::to_owned);

    let mut cursor = 0;
    for hit in parser.feed_ranges(chunk) {
        if hit.start > cursor {
            tracker.on_region_bytes(&chunk[cursor..hit.start]);
        }
        tracker.on_marker(&hit.marker);
        cursor = hit.end;
    }
    if cursor < chunk.len() {
        tracker.on_region_bytes(&chunk[cursor..]);
    }

    let mut events: Vec<SessionEvent> = tracker
        .blocks()
        .iter()
        .filter(|block| last_index.is_none_or(|last| block.index > last))
        .map(|block| SessionEvent::Block {
            block: block.clone(),
        })
        .collect();
    if let Some(path) = tracker.cwd()
        && previous_cwd.as_deref() != Some(path)
    {
        events.push(SessionEvent::Cwd {
            path: path.to_owned(),
        });
    }
    events
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use tauri::ipc::InvokeResponseBody;

    type Collected = Arc<Mutex<Vec<serde_json::Value>>>;

    fn collecting_channel() -> (Channel<SessionEvent>, Collected) {
        let collected: Collected = Arc::default();
        let sink = collected.clone();
        let channel = Channel::new(move |body| {
            if let InvokeResponseBody::Json(json) = body {
                sink.lock()
                    .unwrap()
                    .push(serde_json::from_str(&json).unwrap());
            }
            Ok(())
        });
        (channel, collected)
    }

    fn wait_for(collected: &Collected, kind: &str) -> bool {
        for _ in 0..100 {
            if collected
                .lock()
                .unwrap()
                .iter()
                .any(|event| event["type"] == kind)
            {
                return true;
            }
            thread::sleep(Duration::from_millis(50));
        }
        false
    }

    fn open(sessions: &Sessions, channel: Channel<SessionEvent>, script: &str) -> SessionInfo {
        let dir = std::env::temp_dir().join("termzip-sessions-test");
        // `sh` is not an integrated shell, so the script runs without injected startup files.
        let info = sessions
            .create(channel, 24, 80, None, Some("/bin/sh".to_owned()), &dir)
            .unwrap();
        sessions
            .get(info.id)
            .unwrap()
            .write(script.as_bytes())
            .unwrap();
        info
    }

    #[test]
    fn output_arrives_in_order_and_the_exit_code_is_reported_once() {
        let sessions = Sessions::default();
        let (channel, collected) = collecting_channel();
        open(&sessions, channel, "printf 'one-two'; exit 3\n");

        assert!(wait_for(&collected, "exit"), "no exit event arrived");
        thread::sleep(Duration::from_millis(600));

        let events = collected.lock().unwrap();
        let exits: Vec<_> = events.iter().filter(|e| e["type"] == "exit").collect();
        assert_eq!(exits.len(), 1, "exit must be sent exactly once");
        assert_eq!(exits[0]["code"], 3);
        assert_eq!(events.last().unwrap()["type"], "exit", "exit comes last");

        let output: Vec<u8> = events
            .iter()
            .filter(|e| e["type"] == "data")
            .flat_map(|e| {
                base64::engine::general_purpose::STANDARD
                    .decode(e["data"].as_str().unwrap())
                    .unwrap()
            })
            .collect();
        assert!(String::from_utf8_lossy(&output).contains("one-two"));
    }

    #[test]
    fn destroying_a_session_does_not_report_an_exit() {
        let sessions = Sessions::default();
        let (channel, collected) = collecting_channel();
        let info = open(&sessions, channel, "sleep 30\n");

        sessions.destroy(info.id);
        thread::sleep(Duration::from_millis(1200));

        assert!(
            !collected
                .lock()
                .unwrap()
                .iter()
                .any(|e| e["type"] == "exit"),
            "a destroyed session must stay silent"
        );
        assert!(sessions.get(info.id).is_err());
    }

    fn command(name: &str, output: &str, code: i32) -> Vec<u8> {
        use base64::Engine as _;
        let encoded = base64::engine::general_purpose::STANDARD.encode(name);
        format!(
            "\x1b]133;A\x07$ \x1b]133;B\x07\x1b]1337;CommandB64={encoded}\x07\x1b]133;C\x07{output}\x1b]133;D;{code}\x07"
        )
        .into_bytes()
    }

    #[test]
    fn a_finished_command_is_reported_as_one_block_event() {
        let (mut parser, mut tracker) = (OscParser::new(), BlockTracker::new());
        let events = apply_chunk(&mut parser, &mut tracker, &command("echo hi", "hi\n", 0));

        assert_eq!(events.len(), 1);
        match &events[0] {
            SessionEvent::Block { block } => {
                assert_eq!(block.command.as_deref(), Some("echo hi"));
                assert_eq!(block.exit_code, Some(0));
            }
            other => panic!("expected a block, got {other:?}"),
        }
    }

    #[test]
    fn blocks_keep_being_reported_after_the_store_is_full() {
        let (mut parser, mut tracker) = (OscParser::new(), BlockTracker::new());
        // The tracker keeps 200 blocks; comparing lengths would stop reporting at that point.
        for number in 0..230 {
            let events = apply_chunk(
                &mut parser,
                &mut tracker,
                &command(&format!("echo {number}"), "x", 0),
            );
            let blocks = events
                .iter()
                .filter(|event| matches!(event, SessionEvent::Block { .. }))
                .count();
            assert_eq!(blocks, 1, "command {number} was not reported exactly once");
        }
    }

    #[test]
    fn a_chunk_without_a_finished_command_reports_no_block() {
        let (mut parser, mut tracker) = (OscParser::new(), BlockTracker::new());
        let events = apply_chunk(
            &mut parser,
            &mut tracker,
            b"\x1b]133;A\x07$ \x1b]133;B\x07ec",
        );
        assert!(events.is_empty());
    }

    #[test]
    fn the_working_directory_is_reported_only_when_it_changes() {
        let (mut parser, mut tracker) = (OscParser::new(), BlockTracker::new());
        let first = apply_chunk(&mut parser, &mut tracker, b"\x1b]1337;CurrentDir=/tmp\x07");
        assert!(matches!(&first[..], [SessionEvent::Cwd { path }] if path == "/tmp"));

        let same = apply_chunk(&mut parser, &mut tracker, b"\x1b]1337;CurrentDir=/tmp\x07");
        assert!(same.is_empty());

        let moved = apply_chunk(&mut parser, &mut tracker, b"\x1b]1337;CurrentDir=/etc\x07");
        assert!(matches!(&moved[..], [SessionEvent::Cwd { path }] if path == "/etc"));
    }
}
