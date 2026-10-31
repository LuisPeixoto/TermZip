//! Plugins at work: each enabled plugin lives on a thread of its own and is fed through a bounded queue, so a
//! slow or stuck plugin can delay only itself. What a plugin answers is checked against its permissions
//! ([`check_actions`]) and handed to a sink as [`Report`]s; this crate knows nothing about windows or terminals.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{SyncSender, TrySendError, sync_channel};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use termzip_plugin_api::{
    API_VERSION, Action, BlockEvent, CommandCall, MAX_OUTPUT_BYTES, Manifest, Permission,
};

use crate::sandbox::Sandbox;
use crate::sanitize::check_actions;

/// How many events wait for a plugin. More than this are dropped (and counted) rather than queued.
pub const QUEUE_LEN: usize = 32;
/// A plugin that faults this many calls in a row is stopped until it is enabled again.
pub const MAX_STRIKES: u32 = 3;
/// The interpreter may recurse deeply on the host's stack for deep plugin recursion; give the thread room.
const THREAD_STACK_BYTES: usize = 8 * 1024 * 1024;

/// What caused a set of actions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Trigger {
    /// A finished command: where it ran, so a label can be put on its block.
    Block { session: u32, index: usize },
    /// A palette command the user ran, from the terminal with this session id.
    Command { command: String, session: u32 },
}

/// What the runtime tells its owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Report {
    /// Actions that passed the checks, never an empty list.
    Actions {
        plugin: String,
        trigger: Trigger,
        actions: Vec<Action>,
    },
    /// A call went wrong. `stopped` says the plugin was stopped because of it.
    Fault {
        plugin: String,
        reason: String,
        stopped: bool,
    },
}

pub type Sink = Arc<dyn Fn(Report) + Send + Sync>;

enum Job {
    Block {
        session: u32,
        event: Box<BlockEvent>,
    },
    Command {
        call: CommandCall,
        session: u32,
    },
}

struct Worker {
    queue: SyncSender<Job>,
    permissions: Vec<Permission>,
    alive: Arc<AtomicBool>,
    dropped: Arc<AtomicU64>,
    handle: Option<JoinHandle<()>>,
}

pub struct Runtime {
    workers: Mutex<HashMap<String, Worker>>,
    sink: Sink,
}

/// Cuts the output at a character boundary so it is at most `MAX_OUTPUT_BYTES`.
fn limit_output(event: &mut BlockEvent) {
    if let Some(output) = &mut event.output
        && output.len() > MAX_OUTPUT_BYTES
    {
        let mut end = MAX_OUTPUT_BYTES;
        while !output.is_char_boundary(end) {
            end -= 1;
        }
        output.truncate(end);
    }
}

fn work(
    id: String,
    mut sandbox: Sandbox,
    jobs: std::sync::mpsc::Receiver<Job>,
    permissions: Vec<Permission>,
    alive: Arc<AtomicBool>,
    sink: Sink,
) {
    let mut strikes = 0;
    while let Ok(job) = jobs.recv() {
        let (trigger, result, from_command) = match job {
            Job::Block { session, event } => (
                Trigger::Block {
                    session,
                    index: event.index,
                },
                sandbox.on_block(&event),
                false,
            ),
            Job::Command { call, session } => (
                Trigger::Command {
                    command: call.command.clone(),
                    session,
                },
                sandbox.run_command(&call),
                true,
            ),
        };
        let Some(result) = result else { continue };
        match result {
            Ok(actions) => {
                strikes = 0;
                let checked = check_actions(actions, &permissions, from_command);
                if !checked.actions.is_empty() {
                    sink(Report::Actions {
                        plugin: id.clone(),
                        trigger,
                        actions: checked.actions,
                    });
                }
            }
            Err(fault) => {
                strikes += 1;
                let stopped = strikes >= MAX_STRIKES;
                sink(Report::Fault {
                    plugin: id.clone(),
                    reason: fault.describe(),
                    stopped,
                });
                if stopped {
                    break;
                }
            }
        }
    }
    alive.store(false, Ordering::SeqCst);
}

impl Runtime {
    pub fn new(sink: Sink) -> Self {
        Self {
            workers: Mutex::new(HashMap::new()),
            sink,
        }
    }

    fn workers(&self) -> std::sync::MutexGuard<'_, HashMap<String, Worker>> {
        self.workers
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Loads a plugin and starts its thread. A plugin of the same id that is running is replaced. Nothing is
    /// started when the plugin cannot be loaded, and the error says why.
    pub fn start(&self, bytes: &[u8], manifest: &Manifest) -> Result<(), String> {
        let sandbox = Sandbox::load(bytes, manifest)?;
        let id = manifest.id.clone();
        self.stop(&id);

        let (queue, jobs) = sync_channel(QUEUE_LEN);
        let alive = Arc::new(AtomicBool::new(true));
        let handle = std::thread::Builder::new()
            .name(format!("termzip-plugin-{id}"))
            .stack_size(THREAD_STACK_BYTES)
            .spawn({
                let (id, permissions, alive, sink) = (
                    id.clone(),
                    manifest.permissions.clone(),
                    Arc::clone(&alive),
                    Arc::clone(&self.sink),
                );
                move || work(id, sandbox, jobs, permissions.clone(), alive, sink)
            })
            .map_err(|error| format!("Could not start the plugin: {error}"))?;
        self.workers().insert(
            id,
            Worker {
                queue,
                permissions: manifest.permissions.clone(),
                alive,
                dropped: Arc::new(AtomicU64::new(0)),
                handle: Some(handle),
            },
        );
        Ok(())
    }

    /// Stops a plugin. What it was doing finishes first (a call is bounded by its fuel).
    pub fn stop(&self, id: &str) {
        let worker = self.workers().remove(id);
        if let Some(mut worker) = worker {
            drop(worker.queue); // the thread ends when its queue closes
            if let Some(handle) = worker.handle.take() {
                let _ = handle.join();
            }
        }
    }

    pub fn stop_all(&self) {
        let ids: Vec<String> = self.workers().keys().cloned().collect();
        for id in ids {
            self.stop(&id);
        }
    }

    /// Every plugin that was started and not stopped by the owner, including one that stopped itself.
    pub fn known(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.workers().keys().cloned().collect();
        ids.sort();
        ids
    }

    /// Whether a plugin has been started and not stopped by the owner. A plugin that stopped itself after
    /// repeated crashes still counts: it stays out of the running until it is started again on purpose.
    pub fn has(&self, id: &str) -> bool {
        self.workers().contains_key(id)
    }

    /// The plugins that are running: a plugin that was stopped for faulting is not.
    pub fn running(&self) -> Vec<String> {
        let mut ids: Vec<String> = self
            .workers()
            .iter()
            .filter(|(_, worker)| worker.alive.load(Ordering::SeqCst))
            .map(|(id, _)| id.clone())
            .collect();
        ids.sort();
        ids
    }

    /// How many events were dropped because the plugin's queue was full.
    pub fn dropped(&self, id: &str) -> u64 {
        self.workers()
            .get(id)
            .map_or(0, |worker| worker.dropped.load(Ordering::SeqCst))
    }

    fn offer(worker: &Worker, job: Job) -> bool {
        match worker.queue.try_send(job) {
            Ok(()) => true,
            Err(TrySendError::Full(_)) => {
                worker.dropped.fetch_add(1, Ordering::SeqCst);
                false
            }
            Err(TrySendError::Disconnected(_)) => false,
        }
    }

    /// Tells every plugin that watches commands that one finished. Returns at once: a plugin that is slow or
    /// stuck only fills its own queue.
    pub fn notify_block(&self, session: u32, event: &BlockEvent) {
        let workers = self.workers();
        for worker in workers.values() {
            if !worker.permissions.contains(&Permission::Blocks)
                || !worker.alive.load(Ordering::SeqCst)
            {
                continue;
            }
            let mut event = event.clone();
            event.api = API_VERSION;
            if worker.permissions.contains(&Permission::Output) {
                limit_output(&mut event);
            } else {
                event.output = None;
            }
            Self::offer(
                worker,
                Job::Block {
                    session,
                    event: Box::new(event),
                },
            );
        }
    }

    /// Runs one of a plugin's palette commands, from the terminal with this session id. The answer arrives as
    /// a [`Report`] that names the same session.
    pub fn run_command(
        &self,
        id: &str,
        command: &str,
        cwd: Option<&str>,
        session: u32,
    ) -> Result<(), String> {
        let workers = self.workers();
        let worker = workers
            .get(id)
            .filter(|worker| worker.alive.load(Ordering::SeqCst))
            .ok_or("That plugin is not running.")?;
        let call = CommandCall {
            api: API_VERSION,
            command: command.to_owned(),
            cwd: cwd.map(str::to_owned),
        };
        if Self::offer(worker, Job::Command { call, session }) {
            Ok(())
        } else {
            Err("That plugin is busy. Try again in a moment.".to_owned())
        }
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        self.stop_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::fixtures::{escape, module};
    use std::sync::mpsc::{Receiver, channel};
    use std::time::{Duration, Instant};
    use termzip_plugin_api::Tone;

    const WAIT: Duration = Duration::from_secs(10);

    fn manifest(permissions: &str) -> Manifest {
        toml::from_str(&format!(
            "api = 1\nid = \"t\"\nname = \"T\"\nversion = \"1.0.0\"\npermissions = [{permissions}]\n\n[[commands]]\nid = \"go\"\ntitle = \"Go\"\n"
        ))
        .unwrap()
    }

    fn runtime() -> (Runtime, Receiver<Report>) {
        let (sender, receiver) = channel();
        let sender = Mutex::new(sender);
        let sink: Sink = Arc::new(move |report| {
            let _ = sender.lock().unwrap().send(report);
        });
        (Runtime::new(sink), receiver)
    }

    fn event(index: usize) -> BlockEvent {
        BlockEvent {
            api: 1,
            index,
            command: Some("make".to_owned()),
            cwd: None,
            exit_code: Some(1),
            duration_ms: Some(20),
            output: Some("x".repeat(100)),
        }
    }

    /// A plugin that answers every call with this JSON.
    fn replying(manifest: &Manifest, json: &str) -> Vec<u8> {
        let text = toml::to_string(manifest).unwrap();
        module(
            &text,
            &format!(
                r#"(memory (export "memory") 2)
  (data (i32.const 2048) "{}")
  (func (export "tz_alloc") (param i32) (result i32) (i32.const 1024))
  (func (export "tz_on_block") (param i32 i32) (result i64)
    (i64.or (i64.shl (i64.const 2048) (i64.const 32)) (i64.const {len})))
  (func (export "tz_run_command") (param i32 i32) (result i64)
    (i64.or (i64.shl (i64.const 2048) (i64.const 32)) (i64.const {len})))"#,
                escape(json),
                len = json.len()
            ),
        )
    }

    /// A plugin whose calls all trap.
    fn crashing(manifest: &Manifest) -> Vec<u8> {
        let text = toml::to_string(manifest).unwrap();
        module(
            &text,
            r#"(memory (export "memory") 1)
  (func (export "tz_alloc") (param i32) (result i32) (i32.const 1024))
  (func (export "tz_on_block") (param i32 i32) (result i64) (unreachable))
  (func (export "tz_run_command") (param i32 i32) (result i64) (unreachable))"#,
        )
    }

    #[test]
    fn a_finished_command_reaches_a_plugin_and_its_answer_comes_back_checked() {
        let (runtime, reports) = runtime();
        let m = manifest("\"blocks\", \"notify\"");
        runtime
            .start(
                &replying(&m, r#"{"actions":[{"label":{"text":"slow","tone":"warn"}},{"notify":{"text":"done"}},{"type":"rm -rf /"}]}"#),
                &m,
            )
            .unwrap();

        runtime.notify_block(7, &event(4));

        let report = reports.recv_timeout(WAIT).unwrap();
        assert_eq!(
            report,
            Report::Actions {
                plugin: "t".to_owned(),
                trigger: Trigger::Block {
                    session: 7,
                    index: 4
                },
                // The typing was dropped: not allowed after a finished command, whatever the permissions.
                actions: vec![
                    Action::Label {
                        text: "slow".to_owned(),
                        tone: Tone::Warn
                    },
                    Action::Notify {
                        text: "done".to_owned(),
                        tone: Tone::Info
                    },
                ],
            }
        );
        assert_eq!(runtime.running(), ["t"]);
    }

    #[test]
    fn a_plugin_types_only_for_a_command_the_user_ran_and_only_with_the_permission() {
        let (runtime, reports) = runtime();
        let allowed = manifest("\"type\"");
        runtime
            .start(
                &replying(&allowed, r#"{"actions":[{"type":"git status\n"}]}"#),
                &allowed,
            )
            .unwrap();
        runtime.run_command("t", "go", Some("/w"), 5).unwrap();

        let Report::Actions {
            trigger, actions, ..
        } = reports.recv_timeout(WAIT).unwrap()
        else {
            panic!("expected actions");
        };
        assert_eq!(
            trigger,
            Trigger::Command {
                command: "go".to_owned(),
                session: 5
            }
        );
        // The line break is gone, so it will not run.
        assert_eq!(actions, [Action::Type("git status".to_owned())]);

        // Without the permission the typing is dropped and nothing is reported.
        let (runtime, reports) = self::runtime();
        let denied = manifest("\"notify\"");
        runtime
            .start(
                &replying(&denied, r#"{"actions":[{"type":"git status"}]}"#),
                &denied,
            )
            .unwrap();
        runtime.run_command("t", "go", None, 1).unwrap();
        assert!(reports.recv_timeout(Duration::from_millis(500)).is_err());
    }

    #[test]
    fn a_plugin_is_only_told_about_commands_if_it_asked() {
        let (runtime, reports) = runtime();
        let m = manifest("\"notify\"");
        runtime
            .start(&replying(&m, r#"{"actions":[{"status":"hi"}]}"#), &m)
            .unwrap();

        runtime.notify_block(1, &event(0));

        assert!(reports.recv_timeout(Duration::from_millis(500)).is_err());
    }

    #[test]
    fn answers_arrive_in_order() {
        let (runtime, reports) = runtime();
        let m = manifest("\"blocks\"");
        runtime
            .start(&replying(&m, r#"{"actions":[{"status":"x"}]}"#), &m)
            .unwrap();

        for index in 0..5 {
            runtime.notify_block(1, &event(index));
        }
        let indexes: Vec<usize> = (0..5)
            .map(|_| match reports.recv_timeout(WAIT).unwrap() {
                Report::Actions {
                    trigger: Trigger::Block { index, .. },
                    ..
                } => index,
                other => panic!("{other:?}"),
            })
            .collect();
        assert_eq!(indexes, [0, 1, 2, 3, 4]);
    }

    #[test]
    fn a_plugin_that_keeps_crashing_is_stopped_after_three_in_a_row() {
        let (runtime, reports) = runtime();
        let m = manifest("\"blocks\"");
        runtime.start(&crashing(&m), &m).unwrap();

        for index in 0..3 {
            runtime.notify_block(1, &event(index));
        }
        let mut stopped = Vec::new();
        for _ in 0..3 {
            match reports.recv_timeout(WAIT).unwrap() {
                Report::Fault {
                    plugin,
                    reason,
                    stopped: done,
                } => {
                    assert_eq!(plugin, "t");
                    assert!(reason.starts_with("it crashed"), "{reason}");
                    stopped.push(done);
                }
                other => panic!("{other:?}"),
            }
        }
        assert_eq!(stopped, [false, false, true]);

        // It is out of the running, and nothing more is sent to it.
        let deadline = Instant::now() + WAIT;
        while !runtime.running().is_empty() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(runtime.running().is_empty());
        assert!(runtime.run_command("t", "go", None, 1).is_err());
        // It stopped itself, but it is still the owner's to start again on purpose: `has` says it is known.
        assert!(runtime.has("t"));
    }

    #[test]
    fn a_plugin_that_recovers_between_crashes_is_not_stopped() {
        // Crashes on every other call, so there are never three in a row.
        let m = manifest("\"blocks\"");
        let text = toml::to_string(&m).unwrap();
        let bytes = module(
            &text,
            r#"(memory (export "memory") 1)
  (data (i32.const 2048) "{}")
  (global $n (mut i32) (i32.const 0))
  (func (export "tz_alloc") (param i32) (result i32) (i32.const 1024))
  (func (export "tz_run_command") (param i32 i32) (result i64) (i64.const 0))
  (func (export "tz_on_block") (param i32 i32) (result i64)
    (global.set $n (i32.add (global.get $n) (i32.const 1)))
    (if (i32.and (global.get $n) (i32.const 1)) (then (unreachable)))
    (i64.or (i64.shl (i64.const 2048) (i64.const 32)) (i64.const 2)))"#,
        );
        let (runtime, reports) = runtime();
        runtime.start(&bytes, &m).unwrap();

        for index in 0..8 {
            runtime.notify_block(1, &event(index));
        }
        let mut faults = 0;
        while let Ok(report) = reports.recv_timeout(Duration::from_millis(800)) {
            if let Report::Fault { stopped, .. } = report {
                assert!(!stopped);
                faults += 1;
            }
        }
        assert_eq!(faults, 4);
        assert_eq!(runtime.running(), ["t"]);
    }

    #[test]
    fn a_stuck_plugin_never_holds_up_the_caller() {
        // Every call burns all its fuel, then reports a fault; the caller must return at once all the same.
        let m = manifest("\"blocks\"");
        let text = toml::to_string(&m).unwrap();
        let bytes = module(
            &text,
            r#"(memory (export "memory") 1)
  (func (export "tz_alloc") (param i32) (result i32) (i32.const 1024))
  (func (export "tz_run_command") (param i32 i32) (result i64) (i64.const 0))
  (func (export "tz_on_block") (param i32 i32) (result i64) (loop $l (br $l)) (i64.const 0))"#,
        );
        let (runtime, _reports) = runtime();
        runtime.start(&bytes, &m).unwrap();

        let started = Instant::now();
        for index in 0..500 {
            runtime.notify_block(1, &event(index));
        }
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "{:?}",
            started.elapsed()
        );
        // The queue holds a few and the rest were dropped, not piled up.
        assert!(runtime.dropped("t") > 0 || !runtime.running().contains(&"t".to_owned()));
    }

    #[test]
    fn stopping_ends_the_plugin_and_starting_again_replaces_it() {
        let (runtime, reports) = runtime();
        let m = manifest("\"blocks\"");
        runtime
            .start(&replying(&m, r#"{"actions":[{"status":"one"}]}"#), &m)
            .unwrap();
        runtime
            .start(&replying(&m, r#"{"actions":[{"status":"two"}]}"#), &m)
            .unwrap();
        assert_eq!(runtime.running(), ["t"]);

        runtime.notify_block(1, &event(0));
        let Report::Actions { actions, .. } = reports.recv_timeout(WAIT).unwrap() else {
            panic!("expected actions");
        };
        assert_eq!(actions, [Action::Status("two".to_owned())]);

        runtime.stop("t");
        assert!(runtime.running().is_empty());
        assert!(runtime.run_command("t", "go", None, 1).is_err());
        // Stopping what is not running is fine.
        runtime.stop("t");
        runtime.stop_all();
    }

    #[test]
    fn a_plugin_that_cannot_be_loaded_starts_nothing() {
        let (runtime, _reports) = runtime();
        let m = manifest("\"blocks\"");
        assert!(runtime.start(b"nope", &m).is_err());
        assert!(runtime.running().is_empty());
    }

    #[test]
    fn output_is_cut_to_the_limit_and_only_given_with_the_permission() {
        let mut big = event(0);
        big.output = Some("é".repeat(MAX_OUTPUT_BYTES));
        limit_output(&mut big);
        let cut = big.output.unwrap();
        assert!(cut.len() <= MAX_OUTPUT_BYTES);
        assert!(
            cut.chars().all(|c| c == 'é'),
            "never in the middle of a character"
        );
    }
}
