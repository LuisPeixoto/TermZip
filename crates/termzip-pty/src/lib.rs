use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread;

use anyhow::{Context, Result};
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};

const OUTPUT_BUFFER_CHUNKS: usize = 128;
const READ_BUFFER_BYTES: usize = 8192;

#[derive(Debug, Clone)]
pub struct PtyOptions {
    pub shell: Option<PathBuf>,
    pub args: Vec<String>,
    pub working_directory: Option<PathBuf>,
    pub rows: u16,
    pub columns: u16,
    pub environment: BTreeMap<String, String>,
}

impl Default for PtyOptions {
    fn default() -> Self {
        Self {
            shell: None,
            args: Vec::new(),
            working_directory: None,
            rows: 24,
            columns: 80,
            environment: BTreeMap::new(),
        }
    }
}

pub struct PtySession {
    master: Box<dyn MasterPty + Send>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    child: Box<dyn Child + Send + Sync>,
}

impl PtySession {
    pub fn spawn(
        options: PtyOptions,
        on_output: impl Fn() + Send + Sync + 'static,
    ) -> Result<(Self, Receiver<Vec<u8>>)> {
        let pty_system = native_pty_system();
        let size = PtySize {
            rows: options.rows.max(1),
            cols: options.columns.max(1),
            pixel_width: 0,
            pixel_height: 0,
        };
        let pair = pty_system
            .openpty(size)
            .context("could not open a native pseudo-terminal")?;
        let shell = options.shell.unwrap_or_else(default_shell);
        let mut command = CommandBuilder::new(&shell);
        command.args(&options.args);
        command.env("TERM", "xterm-256color");
        command.env("COLORTERM", "truecolor");
        command.env("TERM_PROGRAM", "TermZip");
        command.env("TERM_PROGRAM_VERSION", env!("CARGO_PKG_VERSION"));
        if let Some(directory) = options.working_directory {
            command.cwd(directory);
        }
        for (key, value) in options.environment {
            command.env(key, value);
        }

        let child = pair
            .slave
            .spawn_command(command)
            .with_context(|| format!("could not start shell {}", shell.display()))?;
        drop(pair.slave);

        let reader = pair
            .master
            .try_clone_reader()
            .context("could not read from the PTY")?;
        let writer = pair
            .master
            .take_writer()
            .context("could not write to the PTY")?;
        let (sender, receiver) = mpsc::sync_channel(OUTPUT_BUFFER_CHUNKS);
        spawn_reader(reader, sender, Arc::new(on_output));

        Ok((
            Self {
                master: pair.master,
                writer: Arc::new(Mutex::new(writer)),
                child,
            },
            receiver,
        ))
    }

    pub fn write(&self, bytes: &[u8]) -> Result<()> {
        let mut writer = self
            .writer
            .lock()
            .map_err(|_| anyhow::anyhow!("PTY writer lock poisoned"))?;
        writer
            .write_all(bytes)
            .context("could not write to the shell")?;
        writer.flush().context("could not flush input to the shell")
    }

    pub fn resize(
        &self,
        rows: u16,
        columns: u16,
        pixel_width: u16,
        pixel_height: u16,
    ) -> Result<()> {
        self.master
            .resize(PtySize {
                rows: rows.max(1),
                cols: columns.max(1),
                pixel_width,
                pixel_height,
            })
            .context("could not resize the shell PTY")
    }

    pub fn try_wait(&mut self) -> Result<Option<portable_pty::ExitStatus>> {
        self.child
            .try_wait()
            .context("could not check the shell process")
    }
}

impl Drop for PtySession {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
        }
    }
}

fn spawn_reader(
    mut reader: Box<dyn Read + Send>,
    sender: SyncSender<Vec<u8>>,
    on_output: Arc<dyn Fn() + Send + Sync>,
) {
    let _ = thread::Builder::new()
        .name("termzip-pty-reader".to_owned())
        .spawn(move || {
            let mut buffer = [0_u8; READ_BUFFER_BYTES];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(count) => {
                        if sender.send(buffer[..count].to_vec()).is_err() {
                            break;
                        }
                        on_output();
                    }
                }
            }
        });
}

fn default_shell() -> PathBuf {
    #[cfg(windows)]
    {
        std::env::var_os("COMSPEC")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("cmd.exe"))
    }

    #[cfg(not(windows))]
    {
        std::env::var_os("SHELL")
            .map(PathBuf::from)
            .filter(|shell| shell.is_file())
            .unwrap_or_else(|| PathBuf::from("/bin/sh"))
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{PtyOptions, PtySession, default_shell};

    #[test]
    fn default_shell_is_not_empty() {
        assert!(!default_shell().as_os_str().is_empty());
    }

    #[test]
    fn shell_output_round_trips_through_the_native_pty() {
        #[cfg(unix)]
        let (mut session, output) = {
            let (session, output) = PtySession::spawn(PtyOptions::default(), || {}).unwrap();
            session.write(b"echo termzip_pty_smoke_marker\r").unwrap();
            (session, output)
        };

        #[cfg(windows)]
        let (mut session, output) = PtySession::spawn(
            PtyOptions {
                shell: Some("cmd.exe".into()),
                args: vec!["/C".to_owned(), "echo termzip_pty_smoke_marker".to_owned()],
                ..PtyOptions::default()
            },
            || {},
        )
        .unwrap();

        let deadline = Instant::now() + Duration::from_secs(10);
        let mut received = Vec::new();
        while Instant::now() < deadline {
            if let Ok(bytes) = output.recv_timeout(Duration::from_millis(250)) {
                received.extend(bytes);
                #[cfg(windows)]
                if received
                    .windows(b"\x1b[6n".len())
                    .any(|window| window == b"\x1b[6n")
                {
                    session.write(b"\x1b[1;1R").unwrap();
                }
                if received
                    .windows(b"termzip_pty_smoke_marker".len())
                    .any(|window| window == b"termzip_pty_smoke_marker")
                {
                    return;
                }
            }
        }

        let exit_code = session
            .try_wait()
            .ok()
            .flatten()
            .map(|status| status.exit_code());
        panic!(
            "shell output did not arrive over the PTY before the test timed out; exit code: {exit_code:?}; received: {:?}",
            String::from_utf8_lossy(&received)
        );
    }
}
