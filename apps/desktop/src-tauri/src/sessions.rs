//! Session lifecycle: PTY spawning, the reader loop, marker parsing, and block
//! assembly. One session owns exactly one shell process.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use base64::Engine;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use termzip_pty::{PtyOptions, PtySession};
use termzip_shell::{Block, BlockTracker, OscParser};

const BACKLOG_LIMIT: usize = 512 * 1024;
const EXIT_POLL: Duration = Duration::from_millis(400);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub id: u32,
    pub shell: String,
    pub integrated: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DataEvent<'a> {
    session_id: u32,
    data: &'a str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExitEvent {
    session_id: u32,
    code: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct BlocksEvent<'a> {
    session_id: u32,
    blocks: &'a [Block],
}

pub struct SessionHandle {
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    master: Mutex<Box<dyn termzip_pty::MasterPty + Send>>,
    child: Mutex<Box<dyn termzip_pty::Child + Send + Sync>>,
    parser: Mutex<OscParser>,
    tracker: Mutex<BlockTracker>,
    backlog: Mutex<Vec<u8>>,
    attached: AtomicBool,
    info: SessionInfo,
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

    pub fn take_backlog(&self) -> String {
        self.attached.store(true, Ordering::Relaxed);
        let Ok(mut backlog) = self.backlog.lock() else {
            return String::new();
        };
        let encoded = base64::engine::general_purpose::STANDARD.encode(backlog.as_slice());
        backlog.clear();
        encoded
    }

    fn try_exit(&self) -> anyhow::Result<Option<termzip_pty::ExitStatus>> {
        self.child
            .lock()
            .map_err(|_| anyhow::anyhow!("child lock poisoned"))?
            .try_wait()
            .map_err(Into::into)
    }

    fn kill(&self) {
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
        app: &AppHandle,
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
            backlog: Mutex::new(Vec::new()),
            attached: AtomicBool::new(false),
            info: info.clone(),
        });

        self.inner
            .lock()
            .map_err(|_| anyhow::anyhow!("sessions lock poisoned"))?
            .insert(id, handle.clone());

        spawn_reader(app.clone(), handle, receiver);
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
    app: AppHandle,
    handle: Arc<SessionHandle>,
    receiver: std::sync::mpsc::Receiver<Vec<u8>>,
) {
    let _ = thread::Builder::new()
        .name("termzip-session-reader".to_owned())
        .spawn(move || {
            loop {
                match receiver.recv_timeout(EXIT_POLL) {
                    Ok(chunk) => handle_chunk(&app, &handle, &chunk),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        if let Ok(Some(status)) = handle.try_exit() {
                            let _ = app.emit(
                                "pty-exit",
                                ExitEvent {
                                    session_id: handle.info.id,
                                    code: Some(status.exit_code() as i32),
                                },
                            );
                            break;
                        }
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                        if let Ok(Some(status)) = handle.try_exit() {
                            let _ = app.emit(
                                "pty-exit",
                                ExitEvent {
                                    session_id: handle.info.id,
                                    code: Some(status.exit_code() as i32),
                                },
                            );
                        }
                        break;
                    }
                }
            }
        });
}

fn handle_chunk(app: &AppHandle, handle: &Arc<SessionHandle>, chunk: &[u8]) {
    if !handle.attached.load(Ordering::Relaxed)
        && let Ok(mut backlog) = handle.backlog.lock()
    {
        backlog.extend_from_slice(chunk);
        if backlog.len() > BACKLOG_LIMIT {
            let overflow = backlog.len() - BACKLOG_LIMIT;
            backlog.drain(..overflow);
        }
    }

    let encoded = base64::engine::general_purpose::STANDARD.encode(chunk);
    let _ = app.emit(
        "pty-data",
        DataEvent {
            session_id: handle.info.id,
            data: &encoded,
        },
    );

    let markers = handle
        .parser
        .lock()
        .map(|mut parser| parser.feed(chunk))
        .unwrap_or_default();

    let Ok(mut tracker) = handle.tracker.lock() else {
        return;
    };
    tracker.on_region_bytes(chunk);
    let before = tracker.blocks().len();
    for marker in &markers {
        tracker.on_marker(marker);
    }
    if tracker.blocks().len() != before {
        let blocks: Vec<Block> = tracker.blocks().iter().cloned().collect();
        let _ = app.emit(
            "blocks-update",
            BlocksEvent {
                session_id: handle.info.id,
                blocks: &blocks,
            },
        );
    }
}
