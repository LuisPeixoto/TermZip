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

use crate::flow::FlowControl;
use termzip_core::Remote;
use termzip_pty::{PtyOptions, PtySession};
use termzip_shell::{Block, BlockTracker, Launch, OscParser};

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
    /// The profile chosen only because the shell started in a folder tied to it, so the interface can show the
    /// look of that profile. `None` when a profile was asked for or none matched.
    pub folder_profile: Option<String>,
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

/// Everything needed to start a shell, after the profile was resolved.
pub struct SessionRequest {
    pub rows: u16,
    pub columns: u16,
    pub working_directory: Option<String>,
    pub shell: Option<String>,
    /// The profile's variables; the integration's own win over them.
    pub env: BTreeMap<String, String>,
    /// An SSH connection or a WSL distribution to start instead of a local shell.
    pub remote: Option<Remote>,
}

/// Told about every finished block of every session, with the id of the session.
pub type BlockObserver = Arc<dyn Fn(u32, &Block) + Send + Sync>;

pub struct SessionHandle {
    id: u32,
    observer: Option<BlockObserver>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    master: Mutex<Box<dyn termzip_pty::MasterPty + Send>>,
    child: Mutex<Box<dyn termzip_pty::Child + Send + Sync>>,
    parser: Mutex<OscParser>,
    tracker: Mutex<BlockTracker>,
    destroyed: AtomicBool,
    /// Back-pressure: how much output the webview has not finished drawing.
    flow: FlowControl,
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

    /// The webview finished drawing `bytes` of output; a paused reader may resume.
    pub fn acknowledge(&self, bytes: usize) {
        self.flow.acknowledge(bytes);
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
        self.flow.wake();
        if let Ok(mut child) = self.child.lock() {
            let _ = child.kill();
        }
    }
}

#[derive(Default)]
pub struct Sessions {
    inner: Mutex<HashMap<u32, Arc<SessionHandle>>>,
    next_id: AtomicU32,
    observer: Mutex<Option<BlockObserver>>,
}

/// What to spawn for a request, and the name the tab falls back to when the program sets no title: for a remote
/// profile that is where it connects (`ssh build-box`), otherwise the shell's own name.
fn launch_for(
    remote: Option<&Remote>,
    shell: Option<&str>,
    working_directory: Option<&str>,
    integration_dir: &Path,
) -> anyhow::Result<(Launch, Option<String>)> {
    match remote {
        Some(Remote::Ssh { host, user, port }) => {
            let launch = termzip_shell::ssh_launch(host, user.as_deref(), *port)
                .map_err(anyhow::Error::msg)?;
            Ok((launch, Some(format!("ssh {host}"))))
        }
        Some(Remote::Wsl { distribution }) => {
            let launch = termzip_shell::wsl_launch(distribution.as_deref(), working_directory)
                .map_err(anyhow::Error::msg)?;
            let label = match distribution {
                Some(name) => format!("wsl {name}"),
                None => "wsl".to_owned(),
            };
            Ok((launch, Some(label)))
        }
        None => Ok((termzip_shell::launch_config(shell, integration_dir), None)),
    }
}

impl Sessions {
    /// Who is told about finished blocks, for sessions created from now on.
    pub fn set_block_observer(&self, observer: BlockObserver) {
        if let Ok(mut slot) = self.observer.lock() {
            *slot = Some(observer);
        }
    }

    fn allocate_id(&self) -> u32 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    pub fn create(
        &self,
        events: Channel<SessionEvent>,
        request: SessionRequest,
        integration_dir: &Path,
    ) -> anyhow::Result<SessionInfo> {
        let SessionRequest {
            rows,
            columns,
            working_directory,
            shell: shell_override,
            env: profile_env,
            remote,
        } = request;
        let (launch, label) = launch_for(
            remote.as_ref(),
            shell_override.as_deref(),
            working_directory.as_deref(),
            integration_dir,
        )?;
        // The profile's variables go first: the integration's own (ZDOTDIR and friends) win.
        let mut environment = profile_env;
        environment.extend(launch.env.iter().cloned());
        let options = PtyOptions {
            shell: Some(launch.program.clone()),
            args: launch.args.clone(),
            // WSL takes its folder as an argument, inside the distribution; an SSH connection starts here.
            working_directory: working_directory
                .filter(|_| remote.is_none())
                .map(std::path::PathBuf::from),
            rows,
            columns,
            environment,
        };

        let (session, receiver) = PtySession::spawn(options, || {})?;
        let id = self.allocate_id();
        let info = SessionInfo {
            id,
            shell: label.unwrap_or_else(|| {
                launch
                    .program
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "shell".to_owned())
            }),
            integrated: launch.integrated,
            folder_profile: None,
        };

        let parts = session.into_parts();
        let observer = self.observer.lock().ok().and_then(|slot| slot.clone());
        let handle = Arc::new(SessionHandle {
            id,
            observer,
            writer: parts.writer,
            master: Mutex::new(parts.master),
            child: Mutex::new(parts.child),
            parser: Mutex::new(OscParser::new()),
            tracker: Mutex::new(BlockTracker::new()),
            destroyed: AtomicBool::new(false),
            flow: FlowControl::default(),
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

/// The most output that is gathered into one event while more is waiting.
const COALESCE_BYTES: usize = 64 * 1024;

/// Adds to `chunk` the reads that are already waiting, up to [`COALESCE_BYTES`], in order. A flood leaves many reads
/// of a few KiB queued, and each one used to be an event of its own (JSON, base64, a script run in the page), which
/// is where the time went; sent together they cost a fraction. A single keystroke is alone in the queue and goes at
/// once: nothing waits for more to arrive.
fn gather(mut chunk: Vec<u8>, receiver: &Receiver<Vec<u8>>) -> Vec<u8> {
    while chunk.len() < COALESCE_BYTES {
        match receiver.try_recv() {
            Ok(next) => chunk.extend_from_slice(&next),
            Err(_) => break,
        }
    }
    chunk
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
                    Ok(chunk) => {
                        let chunk = gather(chunk, &receiver);
                        handle_chunk(&events, &handle, &chunk);
                        // Past the high-water mark this waits for the webview to catch up, which
                        // stops draining the PTY channel and so makes the shell block on write.
                        handle.flow.sent(chunk.len());
                        handle.flow.wait_if_saturated(&handle.destroyed);
                    }
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
        if let (SessionEvent::Block { block }, Some(observer)) = (&event, &handle.observer) {
            observer(handle.id, block);
        }
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
            .create(
                channel,
                SessionRequest {
                    remote: None,
                    rows: 24,
                    columns: 80,
                    working_directory: None,
                    shell: Some("/bin/sh".to_owned()),
                    env: BTreeMap::new(),
                },
                &dir,
            )
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

    /// Polls until `condition` holds, so a slow runner only makes a test slower, never flaky.
    fn wait_until(limit: Duration, condition: impl Fn() -> bool) -> bool {
        let started = std::time::Instant::now();
        while started.elapsed() < limit {
            if condition() {
                return true;
            }
            thread::sleep(Duration::from_millis(100));
        }
        condition()
    }

    fn received_bytes(collected: &Collected) -> usize {
        use base64::Engine as _;
        collected
            .lock()
            .unwrap()
            .iter()
            .filter(|event| event["type"] == "data")
            .map(|event| {
                base64::engine::general_purpose::STANDARD
                    .decode(event["data"].as_str().unwrap())
                    .unwrap()
                    .len()
            })
            .sum()
    }

    #[test]
    fn a_flood_stops_near_the_high_water_mark_until_it_is_acknowledged() {
        let sessions = Sessions::default();
        let (channel, collected) = collecting_channel();
        let info = open(
            &sessions,
            channel,
            "yes aaaaaaaaaaaaaaa | head -c 4000000; exit 0\n",
        );
        let handle = sessions.get(info.id).unwrap();

        // Nobody acknowledges: the output must stop instead of piling up in the webview.
        assert!(
            wait_until(Duration::from_secs(30), || received_bytes(&collected)
                >= crate::flow::HIGH_WATER),
            "the output never reached the high-water mark"
        );
        thread::sleep(Duration::from_millis(800)); // the reader notices and pauses
        let stalled = received_bytes(&collected);
        assert!(
            stalled <= crate::flow::HIGH_WATER + 16 * 1024,
            "kept sending past the high-water mark: {stalled}"
        );
        thread::sleep(Duration::from_millis(500));
        assert_eq!(
            received_bytes(&collected),
            stalled,
            "output continued while paused"
        );

        // Acknowledging what was received lets the rest through, up to the exit.
        let mut acknowledged = 0;
        for _ in 0..400 {
            let total = received_bytes(&collected);
            handle.acknowledge(total - acknowledged);
            acknowledged = total;
            if collected
                .lock()
                .unwrap()
                .iter()
                .any(|event| event["type"] == "exit")
            {
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }
        assert!(
            received_bytes(&collected) >= 4_000_000,
            "the flood did not finish"
        );
    }

    #[test]
    fn destroying_a_paused_session_releases_its_reader() {
        let sessions = Sessions::default();
        let (channel, collected) = collecting_channel();
        let info = open(&sessions, channel, "yes aaaaaaaaaaaaaaa\n");
        let handle = sessions.get(info.id).unwrap();

        assert!(
            wait_until(Duration::from_secs(30), || received_bytes(&collected)
                >= crate::flow::HIGH_WATER),
            "never reached the high-water mark, so it never paused"
        );
        thread::sleep(Duration::from_millis(500)); // the reader notices and pauses

        sessions.destroy(info.id);
        // The reader holds one reference; it must let go once it notices the destroy.
        for _ in 0..60 {
            if Arc::strong_count(&handle) == 1 {
                return;
            }
            thread::sleep(Duration::from_millis(50));
        }
        panic!("the reader of a destroyed, paused session is still running");
    }

    #[test]
    fn a_profile_variable_reaches_the_shell() {
        let sessions = Sessions::default();
        let (channel, collected) = collecting_channel();
        let dir = std::env::temp_dir().join("termzip-sessions-test");
        let mut env = BTreeMap::new();
        env.insert(
            "TERMZIP_TEST_PROFILE_VAR".to_owned(),
            "from-the-profile".to_owned(),
        );
        let info = sessions
            .create(
                channel,
                SessionRequest {
                    remote: None,
                    rows: 24,
                    columns: 80,
                    working_directory: None,
                    shell: Some("/bin/sh".to_owned()),
                    env,
                },
                &dir,
            )
            .unwrap();
        sessions
            .get(info.id)
            .unwrap()
            .write(b"printf '[%s]' \"$TERMZIP_TEST_PROFILE_VAR\"; exit 0\n")
            .unwrap();

        assert!(wait_for(&collected, "exit"));
        let output = received_text(&collected);
        assert!(output.contains("[from-the-profile]"), "{output}");
    }

    /// (session id, command) of each block an observer was told about.
    type Seen = Arc<Mutex<Vec<(u32, Option<String>)>>>;

    #[test]
    fn an_observer_is_told_about_each_finished_block_with_its_session() {
        let sessions = Sessions::default();
        let seen: Seen = Arc::default();
        sessions.set_block_observer({
            let seen = seen.clone();
            Arc::new(move |id, block| seen.lock().unwrap().push((id, block.command.clone())))
        });
        let (channel, collected) = collecting_channel();
        // The shell prints the markers of one finished command, as the integration scripts do.
        let printed = String::from_utf8(command("echo hi", "hi\n", 0))
            .unwrap()
            .replace('\x1b', "\\033")
            .replace('\x07', "\\007")
            .replace('\n', "\\n");
        let info = open(
            &sessions,
            channel,
            &format!("printf '%b' '{printed}'; exit 0\n"),
        );

        assert!(wait_for(&collected, "exit"));
        assert_eq!(
            *seen.lock().unwrap(),
            [(info.id, Some("echo hi".to_owned()))]
        );
    }

    fn received_text(collected: &Collected) -> String {
        use base64::Engine as _;
        let bytes: Vec<u8> = collected
            .lock()
            .unwrap()
            .iter()
            .filter(|event| event["type"] == "data")
            .flat_map(|event| {
                base64::engine::general_purpose::STANDARD
                    .decode(event["data"].as_str().unwrap())
                    .unwrap()
            })
            .collect();
        String::from_utf8_lossy(&bytes).into_owned()
    }

    #[test]
    fn an_ssh_profile_starts_ssh_and_names_the_tab_after_the_host() {
        let remote = Remote::Ssh {
            host: "build-box".to_owned(),
            user: Some("deploy".to_owned()),
            port: Some(2222),
        };

        let (launch, label) = launch_for(
            Some(&remote),
            Some("/bin/zsh"),
            Some("/local"),
            Path::new("/x"),
        )
        .unwrap();

        assert!(launch.program.to_string_lossy().starts_with("ssh"));
        assert_eq!(
            launch.args,
            ["-l", "deploy", "-p", "2222", "--", "build-box"]
        );
        assert!(!launch.integrated);
        assert_eq!(label.as_deref(), Some("ssh build-box"));
    }

    #[test]
    fn a_host_that_reads_as_an_option_never_reaches_ssh() {
        let remote = Remote::Ssh {
            host: "-oProxyCommand=touch /tmp/pwned".to_owned(),
            user: None,
            port: None,
        };

        assert!(launch_for(Some(&remote), None, None, Path::new("/x")).is_err());
    }

    #[test]
    fn a_local_profile_starts_its_shell_and_keeps_the_shell_s_name() {
        let (launch, label) = launch_for(None, Some("/bin/sh"), None, Path::new("/x")).unwrap();

        assert_eq!(launch.program, Path::new("/bin/sh"));
        assert_eq!(label, None);
    }

    #[cfg(not(windows))]
    #[test]
    fn a_wsl_profile_says_it_needs_windows_elsewhere() {
        let remote = Remote::Wsl {
            distribution: Some("Ubuntu".to_owned()),
        };

        let problem = launch_for(Some(&remote), None, None, Path::new("/x")).unwrap_err();

        assert_eq!(problem.to_string(), "WSL profiles work on Windows only.");
    }

    /// Signs in to a real `sshd` through an SSH profile, the way a person would: the host is an alias in an SSH
    /// config. `ssh` is found through the `PATH` of the profile, where a wrapper adds `-F <a throwaway config>` and
    /// passes on every other argument as TermZip built it, so nothing of the real `~/.ssh` is read or written.
    ///
    /// Needs a throwaway server: run `tools/live-tests.sh`, which starts one and sets the variable.
    #[test]
    #[ignore = "needs a throwaway sshd: run tools/live-tests.sh"]
    fn an_ssh_profile_signs_in_to_a_real_sshd_and_runs_a_command() {
        let bin =
            std::env::var("TERMZIP_LIVE_SSH_BIN").expect("run this through tools/live-tests.sh");
        let path = std::env::join_paths(std::iter::once(std::path::PathBuf::from(bin)).chain(
            std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()),
        ))
        .unwrap();
        let sessions = Sessions::default();
        let (channel, collected) = collecting_channel();
        let dir = std::env::temp_dir().join("termzip-sessions-live");

        let info = sessions
            .create(
                channel,
                SessionRequest {
                    remote: Some(Remote::Ssh {
                        host: "livebox".to_owned(),
                        user: None,
                        port: None,
                    }),
                    rows: 24,
                    columns: 80,
                    working_directory: None,
                    shell: None,
                    env: BTreeMap::from([("PATH".to_owned(), path.to_string_lossy().into_owned())]),
                },
                &dir,
            )
            .unwrap();
        assert_eq!(info.shell, "ssh livebox");
        assert!(!info.integrated, "the other side has no shell integration");
        // The sum is computed by the remote shell, so the answer is not in what was typed.
        sessions
            .get(info.id)
            .unwrap()
            .write(b"echo live-$((40+2))\nexit\n")
            .unwrap();

        for _ in 0..300 {
            if collected
                .lock()
                .unwrap()
                .iter()
                .any(|e| e["type"] == "exit")
            {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
        let events = collected.lock().unwrap();
        let output: Vec<u8> = events
            .iter()
            .filter(|e| e["type"] == "data")
            .flat_map(|e| {
                base64::engine::general_purpose::STANDARD
                    .decode(e["data"].as_str().unwrap())
                    .unwrap()
            })
            .collect();
        let text = String::from_utf8_lossy(&output);
        assert!(
            text.contains("live-42"),
            "no answer from the remote shell: {text:?}"
        );
        let exits: Vec<_> = events.iter().filter(|e| e["type"] == "exit").collect();
        assert_eq!(exits.len(), 1, "the connection did not end: {text:?}");
        assert_eq!(exits[0]["code"], 0);
    }

    #[test]
    fn a_command_with_a_huge_output_still_reports_its_block() {
        // `seq 1 200000`: 1.3 MB between the start and the end markers, in the chunks a PTY delivers. The end marker
        // used to be dropped once the output had filled the parser's buffer, so the block never finished.
        let (mut parser, mut tracker) = (OscParser::new(), BlockTracker::new());
        let mut stream = Vec::new();
        stream.extend_from_slice(
            b"\x1b]133;A\x07$ \x1b]133;B\x07\x1b]1337;CommandB64=c2VxIDEgMjAwMDAw\x07\x1b]133;C\x07",
        );
        for number in 1..=200_000u32 {
            stream.extend_from_slice(format!("{number}\r\n").as_bytes());
        }
        stream.extend_from_slice(b"\x1b]133;D;0\x07\x1b]133;A\x07$ \x1b]133;B\x07");

        let mut blocks = Vec::new();
        for chunk in stream.chunks(4096) {
            for event in apply_chunk(&mut parser, &mut tracker, chunk) {
                if let SessionEvent::Block { block } = event {
                    blocks.push(block);
                }
            }
        }

        assert_eq!(
            blocks.len(),
            1,
            "the block of the command was never reported"
        );
        assert_eq!(blocks[0].command.as_deref(), Some("seq 1 200000"));
        assert_eq!(blocks[0].exit_code, Some(0));
        assert!(
            blocks[0].truncated,
            "the output is longer than a block keeps"
        );
    }

    /// The output of every data event, in order.
    fn output_of(collected: &Collected) -> (Vec<u8>, Vec<usize>) {
        let events = collected.lock().unwrap();
        let mut output = Vec::new();
        let mut sizes = Vec::new();
        for event in events.iter().filter(|event| event["type"] == "data") {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(event["data"].as_str().unwrap())
                .unwrap();
            sizes.push(bytes.len());
            output.extend(bytes);
        }
        (output, sizes)
    }

    #[cfg(unix)]
    #[test]
    fn a_flood_arrives_whole_and_in_order() {
        let sessions = Sessions::default();
        let (channel, collected) = collecting_channel();
        open(&sessions, channel, "seq 1 60000; exit 0\n");
        assert!(wait_for_long(&collected, "exit"), "the flood did not end");
        thread::sleep(Duration::from_millis(300));

        let (output, sizes) = output_of(&collected);
        let text = String::from_utf8_lossy(&output);
        let expected: String = (1..=60_000u32).map(|n| format!("{n}\r\n")).collect();

        assert!(
            text.contains(&expected),
            "the numbers are missing or out of order"
        );
        assert!(sizes.iter().all(|size| *size <= COALESCE_BYTES + 8192));
    }

    #[cfg(unix)]
    #[test]
    fn a_keystroke_is_not_held_back_to_fill_a_batch() {
        let sessions = Sessions::default();
        let (channel, collected) = collecting_channel();
        let info = open(&sessions, channel, "printf ready; cat\n");
        let started = std::time::Instant::now();
        while !String::from_utf8_lossy(&output_of(&collected).0).contains("ready") {
            assert!(
                started.elapsed() < Duration::from_secs(5),
                "no prompt output"
            );
            thread::sleep(Duration::from_millis(10));
        }

        sessions.get(info.id).unwrap().write(b"q").unwrap();
        let typed = std::time::Instant::now();
        while !String::from_utf8_lossy(&output_of(&collected).0).contains('q') {
            assert!(
                typed.elapsed() < Duration::from_secs(2),
                "the echo of a keystroke took too long"
            );
            thread::sleep(Duration::from_millis(5));
        }
        assert!(typed.elapsed() < Duration::from_millis(500));
        sessions.destroy(info.id);
    }

    /// Like `wait_for`, for something that takes a while.
    fn wait_for_long(collected: &Collected, kind: &str) -> bool {
        for _ in 0..600 {
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

    fn queue(chunks: usize, size: usize) -> Receiver<Vec<u8>> {
        let (sender, receiver) = std::sync::mpsc::channel();
        for number in 0..chunks {
            sender.send(vec![(number % 251) as u8; size]).unwrap();
        }
        receiver
    }

    #[test]
    fn reads_that_are_waiting_are_gathered_into_one_chunk_in_order() {
        let receiver = queue(100, 1000);

        let gathered = gather(vec![255; 1000], &receiver);

        // The first read, then the waiting ones in the order they came, until the limit is passed.
        assert!(gathered.len() >= COALESCE_BYTES && gathered.len() < COALESCE_BYTES + 1000);
        assert_eq!(gathered[0], 255);
        assert_eq!(gathered[1000], 0);
        assert_eq!(gathered[2000], 1);
        // What was not taken is still there, next in line.
        let rest = receiver.try_recv().unwrap();
        assert_eq!(rest[0], ((gathered.len() / 1000 - 1) % 251) as u8);
    }

    #[test]
    fn a_read_that_is_alone_is_returned_at_once() {
        let receiver = queue(0, 0);

        assert_eq!(gather(b"q".to_vec(), &receiver), b"q");
    }

    #[test]
    fn a_big_first_read_takes_nothing_more() {
        let receiver = queue(3, 10);

        let gathered = gather(vec![1; COALESCE_BYTES], &receiver);

        assert_eq!(gathered.len(), COALESCE_BYTES);
        assert_eq!(receiver.try_iter().count(), 3);
    }
}
