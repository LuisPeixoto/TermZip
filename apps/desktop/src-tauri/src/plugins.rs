//! Plugins in the desktop shell: what is installed, what runs, and how what a plugin answers reaches the
//! webview. The sandbox, the checks and the files are `termzip-plugins`; this is the glue to windows and
//! terminals.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;
use termzip_plugin_api::{Action, BlockEvent, Permission, Tone};
use termzip_plugins::{
    LinkMatch, LinkMatcher, PluginInfo, PluginStore, Report, Runtime, Sink, Source, Trigger,
};
use termzip_shell::Block;

use crate::links::LinkTarget;

/// What the webview is told. Plain data: text that already passed the checks of the runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PluginEvent {
    /// A badge for a finished command's block.
    #[serde(rename_all = "camelCase")]
    Label {
        plugin: String,
        session: u32,
        block_index: usize,
        text: String,
        tone: Tone,
    },
    /// A short message.
    Notify {
        plugin: String,
        text: String,
        tone: Tone,
    },
    /// The text of the plugin's status-bar item; empty removes it.
    Status { plugin: String, text: String },
    /// Text to paste into the terminal the user ran the command from. It never contains a line break.
    Type {
        plugin: String,
        session: u32,
        text: String,
    },
    /// A call went wrong; `stopped` says the plugin was stopped because of it.
    Fault {
        plugin: String,
        reason: String,
        stopped: bool,
    },
}

/// What a report means for the webview. Addresses to open are not in the list: they are opened here.
fn events_of(report: Report) -> (Vec<PluginEvent>, Vec<String>) {
    match report {
        Report::Fault {
            plugin,
            reason,
            stopped,
        } => (
            vec![PluginEvent::Fault {
                plugin,
                reason,
                stopped,
            }],
            Vec::new(),
        ),
        Report::Actions {
            plugin,
            trigger,
            actions,
        } => {
            let mut events = Vec::new();
            let mut addresses = Vec::new();
            for action in actions {
                match (action, &trigger) {
                    (Action::Label { text, tone }, Trigger::Block { session, index }) => {
                        events.push(PluginEvent::Label {
                            plugin: plugin.clone(),
                            session: *session,
                            block_index: *index,
                            text,
                            tone,
                        });
                    }
                    (Action::Notify { text, tone }, _) => events.push(PluginEvent::Notify {
                        plugin: plugin.clone(),
                        text,
                        tone,
                    }),
                    (Action::Status(text), _) => events.push(PluginEvent::Status {
                        plugin: plugin.clone(),
                        text,
                    }),
                    (Action::Type(text), Trigger::Command { session, .. }) => {
                        events.push(PluginEvent::Type {
                            plugin: plugin.clone(),
                            session: *session,
                            text,
                        });
                    }
                    (Action::Open(url), Trigger::Command { .. }) => addresses.push(url),
                    // The runtime already refused the rest; nothing here can be reached.
                    _ => {}
                }
            }
            (events, addresses)
        }
    }
}

/// Opens an address a plugin asked for after the user ran its command: through the same check as a click.
fn open_address(url: &str) {
    if let Ok(LinkTarget::Url(url)) = crate::links::resolve(url, None, None) {
        let _ = open::that_detached(url);
    }
}

type Subscriber = Arc<Mutex<Option<Channel<PluginEvent>>>>;

/// The plugins of this run of the app.
pub struct PluginHost {
    /// Where plugins are kept; `None` is the data folder.
    dir: Option<PathBuf>,
    runtime: Runtime,
    subscriber: Subscriber,
    links: RwLock<LinkMatcher>,
}

impl PluginHost {
    pub fn new() -> Self {
        Self::in_dir(None)
    }

    pub fn in_dir(dir: Option<PathBuf>) -> Self {
        let subscriber: Subscriber = Arc::default();
        let sink: Sink = {
            let subscriber = Arc::clone(&subscriber);
            Arc::new(move |report| {
                let (events, addresses) = events_of(report);
                if let Ok(slot) = subscriber.lock()
                    && let Some(channel) = slot.as_ref()
                {
                    for event in events {
                        let _ = channel.send(event);
                    }
                }
                for url in addresses {
                    open_address(&url);
                }
            })
        };
        Self {
            dir,
            runtime: Runtime::new(sink),
            subscriber,
            links: RwLock::new(LinkMatcher::default()),
        }
    }

    pub fn store(&self) -> Result<PluginStore, String> {
        match &self.dir {
            Some(dir) => Ok(PluginStore::new(dir)),
            None => PluginStore::in_data_dir(),
        }
    }

    /// Makes what runs match what the user has approved: starts approved plugins that are not running, stops
    /// those that are no longer approved, and rebuilds the link patterns. Returns a sentence for each plugin
    /// that could not be started.
    pub fn sync(&self) -> Vec<String> {
        let Ok(store) = self.store() else {
            return Vec::new();
        };
        let listing = store.list();
        let mut problems = Vec::new();
        let enabled: Vec<&PluginInfo> =
            listing.plugins.iter().filter(|info| info.enabled).collect();

        for id in self.runtime.known() {
            if !enabled.iter().any(|info| info.manifest.id == id) {
                self.runtime.stop(&id);
            }
        }
        for info in &enabled {
            if self.runtime.has(&info.manifest.id) {
                continue;
            }
            let started = store
                .approved_bytes(&info.manifest.id)
                .and_then(|bytes| self.runtime.start(&bytes, &info.manifest));
            if let Err(why) = started {
                problems.push(format!("“{}” could not start: {why}", info.manifest.name));
            }
        }
        if let Ok(mut links) = self.links.write() {
            *links = LinkMatcher::new(
                enabled
                    .iter()
                    .filter(|info| self.runtime.running().contains(&info.manifest.id))
                    .map(|info| &info.manifest),
            );
        }
        problems
    }

    /// A command finished in a session: tell the plugins that watch commands.
    pub fn on_block(&self, session: u32, block: &Block) {
        self.runtime.notify_block(
            session,
            &BlockEvent {
                api: termzip_plugin_api::API_VERSION,
                index: block.index,
                command: block.command.clone(),
                cwd: block.cwd.clone(),
                exit_code: block.exit_code,
                duration_ms: block.duration_ms,
                output: block.output.clone(),
            },
        );
    }

    pub fn stop_all(&self) {
        self.runtime.stop_all();
    }
}

impl Default for PluginHost {
    fn default() -> Self {
        Self::new()
    }
}

/// An installed plugin as the interface shows it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginView {
    #[serde(flatten)]
    pub info: PluginInfo,
    /// Running right now: enabled, loaded, and not stopped for crashing.
    pub running: bool,
    /// What each permission lets the plugin do, as sentences for the confirmation.
    pub permission_notes: Vec<PermissionNote>,
}

#[derive(Debug, Serialize)]
pub struct PermissionNote {
    pub permission: Permission,
    pub text: &'static str,
}

#[derive(Debug, Serialize)]
pub struct PluginListing {
    pub plugins: Vec<PluginView>,
    pub problems: Vec<String>,
}

fn view(host: &PluginHost, info: PluginInfo) -> PluginView {
    let running = host.runtime.running().contains(&info.manifest.id);
    let permission_notes = info
        .manifest
        .permissions
        .iter()
        .map(|permission| PermissionNote {
            permission: *permission,
            text: permission.describe(),
        })
        .collect();
    PluginView {
        info,
        running,
        permission_notes,
    }
}

#[tauri::command]
pub fn list_plugins(host: State<'_, Arc<PluginHost>>) -> Result<PluginListing, String> {
    let listing = host.store()?.list();
    let mut problems = listing.problems;
    problems.extend(host.sync());
    Ok(PluginListing {
        plugins: listing
            .plugins
            .into_iter()
            .map(|info| view(&host, info))
            .collect(),
        problems,
    })
}

/// Asks for a `.wasm` file and installs it, disabled and marked as not verified (it came from the disk).
#[tauri::command]
pub async fn install_plugin_file(
    app: AppHandle,
    host: State<'_, Arc<PluginHost>>,
) -> Result<Option<PluginView>, String> {
    let Some(picked) = app
        .dialog()
        .file()
        .set_title("Install a plugin")
        .add_filter("TermZip plugin", &["wasm"])
        .blocking_pick_file()
    else {
        return Ok(None);
    };
    let path = picked.into_path().map_err(|error| error.to_string())?;
    let bytes = termzip_plugins::read_plugin_file(&path)?;
    let info = host.store()?.install(&bytes, Source::Local)?;
    host.sync();
    Ok(Some(view(&host, info)))
}

/// Approves the plugin as it is and starts it. If it cannot start, the approval is taken back.
#[tauri::command]
pub fn enable_plugin(id: String, host: State<'_, Arc<PluginHost>>) -> Result<PluginView, String> {
    let store = host.store()?;
    let info = store.enable(&id)?;
    let bytes = store.approved_bytes(&id)?;
    if let Err(why) = host.runtime.start(&bytes, &info.manifest) {
        let _ = store.disable(&id);
        host.sync();
        return Err(why);
    }
    host.sync();
    Ok(view(&host, info))
}

#[tauri::command]
pub fn disable_plugin(id: String, host: State<'_, Arc<PluginHost>>) -> Result<(), String> {
    host.store()?.disable(&id)?;
    host.runtime.stop(&id);
    host.sync();
    Ok(())
}

#[tauri::command]
pub fn uninstall_plugin(id: String, host: State<'_, Arc<PluginHost>>) -> Result<(), String> {
    host.runtime.stop(&id);
    host.store()?.uninstall(&id)?;
    host.sync();
    Ok(())
}

/// Runs a plugin's palette command from the terminal with `session`. The answer comes on the channel.
#[tauri::command]
pub fn run_plugin_command(
    plugin: String,
    command: String,
    cwd: Option<String>,
    session: u32,
    host: State<'_, Arc<PluginHost>>,
) -> Result<(), String> {
    host.runtime
        .run_command(&plugin, &command, cwd.as_deref(), session)
}

/// The links the enabled plugins find in a line of output.
#[tauri::command]
pub fn match_plugin_links(line: String, host: State<'_, Arc<PluginHost>>) -> Vec<LinkMatch> {
    host.links
        .read()
        .map(|links| links.find(&line))
        .unwrap_or_default()
}

/// Where the answers of plugins are sent: one channel for the whole window.
#[tauri::command]
pub fn subscribe_plugins(on_event: Channel<PluginEvent>, host: State<'_, Arc<PluginHost>>) {
    if let Ok(mut slot) = host.subscriber.lock() {
        *slot = Some(on_event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tauri::ipc::InvokeResponseBody;
    use termzip_plugin_api::Tone;

    // --- what a report means for the webview ---------------------------------------------------------------

    fn actions(trigger: Trigger, actions: Vec<Action>) -> Report {
        Report::Actions {
            plugin: "p".to_owned(),
            trigger,
            actions,
        }
    }

    fn label(text: &str) -> Action {
        Action::Label {
            text: text.to_owned(),
            tone: Tone::Warn,
        }
    }

    #[test]
    fn the_answer_to_a_finished_command_labels_its_block() {
        let (events, addresses) = events_of(actions(
            Trigger::Block {
                session: 3,
                index: 9,
            },
            vec![label("slow"), Action::Status("1 failed".to_owned())],
        ));

        assert!(addresses.is_empty());
        assert_eq!(
            events,
            [
                PluginEvent::Label {
                    plugin: "p".to_owned(),
                    session: 3,
                    block_index: 9,
                    text: "slow".to_owned(),
                    tone: Tone::Warn
                },
                PluginEvent::Status {
                    plugin: "p".to_owned(),
                    text: "1 failed".to_owned()
                },
            ]
        );
    }

    #[test]
    fn the_answer_to_a_command_types_into_the_terminal_it_came_from_and_opens_addresses() {
        let trigger = Trigger::Command {
            command: "go".to_owned(),
            session: 7,
        };
        let (events, addresses) = events_of(actions(
            trigger,
            vec![
                Action::Type("git status".to_owned()),
                Action::Open("https://example.com".to_owned()),
                Action::Notify {
                    text: "hi".to_owned(),
                    tone: Tone::Ok,
                },
            ],
        ));

        assert_eq!(addresses, ["https://example.com"]);
        assert_eq!(
            events[0],
            PluginEvent::Type {
                plugin: "p".to_owned(),
                session: 7,
                text: "git status".to_owned()
            }
        );
        assert!(matches!(events[1], PluginEvent::Notify { .. }));
    }

    #[test]
    fn what_the_wrong_trigger_cannot_do_is_never_turned_into_an_event() {
        // Defense in depth: the runtime already drops these, and here they could not be sent either.
        let after_block = events_of(actions(
            Trigger::Block {
                session: 1,
                index: 0,
            },
            vec![
                Action::Type("rm -rf /".to_owned()),
                Action::Open("https://example.com".to_owned()),
            ],
        ));
        assert!(after_block.0.is_empty() && after_block.1.is_empty());

        let after_command = events_of(actions(
            Trigger::Command {
                command: "go".to_owned(),
                session: 1,
            },
            vec![label("slow")],
        ));
        assert!(after_command.0.is_empty());
    }

    #[test]
    fn the_events_have_the_shape_the_webview_expects() {
        let label = serde_json::to_value(PluginEvent::Label {
            plugin: "p".to_owned(),
            session: 3,
            block_index: 9,
            text: "slow".to_owned(),
            tone: Tone::Warn,
        })
        .unwrap();
        assert_eq!(
            label,
            serde_json::json!({"type":"label","plugin":"p","session":3,"blockIndex":9,"text":"slow","tone":"warn"})
        );
        let fault = serde_json::to_value(PluginEvent::Fault {
            plugin: "p".to_owned(),
            reason: "it crashed".to_owned(),
            stopped: true,
        })
        .unwrap();
        assert_eq!(
            fault,
            serde_json::json!({"type":"fault","plugin":"p","reason":"it crashed","stopped":true})
        );
        let typed = serde_json::to_value(PluginEvent::Type {
            plugin: "p".to_owned(),
            session: 2,
            text: "ls".to_owned(),
        })
        .unwrap();
        assert_eq!(typed["type"], "type");
    }

    // --- a host with a real plugin --------------------------------------------------------------------------

    const MANIFEST: &str = "api = 1\nid = \"demo\"\nname = \"Demo\"\nversion = \"1.0.0\"\npermissions = [\"blocks\", \"notify\", \"type\"]\n\n[[commands]]\nid = \"go\"\ntitle = \"Go\"\n\n[[links]]\npattern = \"#([0-9]+)\"\nurl = \"https://example.com/issues/{1}\"\n";

    fn escape(text: &str) -> String {
        text.bytes().map(|byte| format!("\\{byte:02x}")).collect()
    }

    /// A plugin that answers a finished command with a label, a status and a message, and a command with typing.
    fn demo_plugin() -> Vec<u8> {
        let on_block = r#"{"actions":[{"label":{"text":"slow","tone":"warn"}},{"status":"1 seen"},{"notify":{"text":"done"}}]}"#;
        let on_command = r#"{"actions":[{"type":"git status\n"}]}"#;
        let text = format!(
            r#"(module
  (@custom "termzip-plugin" "{}")
  (memory (export "memory") 2)
  (data (i32.const 2048) "{}")
  (data (i32.const 4096) "{}")
  (func (export "tz_alloc") (param i32) (result i32) (i32.const 1024))
  (func (export "tz_on_block") (param i32 i32) (result i64)
    (i64.or (i64.shl (i64.const 2048) (i64.const 32)) (i64.const {})))
  (func (export "tz_run_command") (param i32 i32) (result i64)
    (i64.or (i64.shl (i64.const 4096) (i64.const 32)) (i64.const {}))))"#,
            escape(MANIFEST),
            escape(on_block),
            escape(on_command),
            on_block.len(),
            on_command.len()
        );
        wat::parse_str(text).unwrap()
    }

    type Collected = Arc<Mutex<Vec<serde_json::Value>>>;

    fn subscribe(host: &PluginHost) -> Collected {
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
        *host.subscriber.lock().unwrap() = Some(channel);
        collected
    }

    fn wait_for(collected: &Collected, count: usize) -> Vec<serde_json::Value> {
        for _ in 0..200 {
            if collected.lock().unwrap().len() >= count {
                break;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        collected.lock().unwrap().clone()
    }

    fn temp_host(name: &str) -> (PluginHost, PathBuf) {
        let dir =
            std::env::temp_dir().join(format!("termzip-plugin-host-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        (PluginHost::in_dir(Some(dir.clone())), dir)
    }

    fn block() -> Block {
        Block {
            index: 4,
            command: Some("sleep 20".to_owned()),
            cwd: Some("/work".to_owned()),
            exit_code: Some(0),
            started_at_ms: Some(1),
            duration_ms: Some(20_000),
            output_chars: 0,
            truncated: false,
            output: None,
        }
    }

    #[test]
    fn an_approved_plugin_runs_and_a_disabled_one_does_not() {
        let (host, dir) = temp_host("flow");
        let events = subscribe(&host);
        let store = host.store().unwrap();
        store.install(&demo_plugin(), Source::Local).unwrap();

        // Installed but not approved: nothing runs.
        assert!(host.sync().is_empty());
        host.on_block(1, &block());
        std::thread::sleep(Duration::from_millis(300));
        assert!(events.lock().unwrap().is_empty());

        // Approved: it is started by `sync`, and told about the next command.
        store.enable("demo").unwrap();
        assert!(host.sync().is_empty(), "the plugin must start");
        host.on_block(1, &block());
        let got = wait_for(&events, 3);
        let kinds: Vec<&str> = got
            .iter()
            .map(|event| event["type"].as_str().unwrap())
            .collect();
        assert_eq!(kinds, ["label", "status", "notify"]);
        assert_eq!(got[0]["blockIndex"], 4);
        assert_eq!(got[0]["session"], 1);

        // Disabled again: it is stopped and hears nothing more.
        store.disable("demo").unwrap();
        host.sync();
        events.lock().unwrap().clear();
        host.on_block(1, &block());
        std::thread::sleep(Duration::from_millis(300));
        assert!(events.lock().unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_command_of_a_plugin_types_into_the_session_it_was_run_from() {
        let (host, dir) = temp_host("command");
        let events = subscribe(&host);
        let store = host.store().unwrap();
        store.install(&demo_plugin(), Source::Local).unwrap();
        store.enable("demo").unwrap();
        host.sync();

        host.runtime
            .run_command("demo", "go", Some("/work"), 6)
            .unwrap();

        let got = wait_for(&events, 1);
        assert_eq!(got[0]["type"], "type");
        assert_eq!(got[0]["session"], 6);
        // The line break the plugin asked for is gone, so nothing runs.
        assert_eq!(got[0]["text"], "git status");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_plugin_that_was_changed_on_disk_is_stopped_by_the_next_sync() {
        let (host, dir) = temp_host("changed");
        let events = subscribe(&host);
        let store = host.store().unwrap();
        store.install(&demo_plugin(), Source::Local).unwrap();
        store.enable("demo").unwrap();
        host.sync();
        assert!(host.runtime.running().contains(&"demo".to_owned()));

        // The file is replaced by something else behind TermZip's back (here, the same plugin with a changed
        // version, which is a different file).
        let changed = demo_plugin_with(&MANIFEST.replace("1.0.0", "1.0.1"));
        std::fs::write(dir.join("demo.wasm"), changed).unwrap();
        host.sync();

        assert!(host.runtime.running().is_empty());
        host.on_block(1, &block());
        std::thread::sleep(Duration::from_millis(300));
        assert!(events.lock().unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn demo_plugin_with(manifest: &str) -> Vec<u8> {
        let text = format!(
            r#"(module
  (@custom "termzip-plugin" "{}")
  (memory (export "memory") 1)
  (func (export "tz_alloc") (param i32) (result i32) (i32.const 1024))
  (func (export "tz_on_block") (param i32 i32) (result i64) (i64.const 0))
  (func (export "tz_run_command") (param i32 i32) (result i64) (i64.const 0)))"#,
            escape(manifest)
        );
        wat::parse_str(text).unwrap()
    }

    #[test]
    fn the_links_of_enabled_plugins_are_found_in_a_line() {
        let (host, dir) = temp_host("links");
        let store = host.store().unwrap();
        store.install(&demo_plugin(), Source::Local).unwrap();

        host.sync();
        assert!(
            host.links.read().unwrap().find("fixes #42").is_empty(),
            "not enabled yet"
        );

        store.enable("demo").unwrap();
        host.sync();
        let links = host.links.read().unwrap().find("fixes #42");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].url, "https://example.com/issues/42");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_plugin_that_cannot_start_is_reported_not_silently_skipped() {
        let (host, dir) = temp_host("broken");
        let store = host.store().unwrap();
        // A plugin that passes the loader but whose start function never returns.
        let manifest = "api = 1\nid = \"stuck\"\nname = \"Stuck\"\nversion = \"1.0.0\"\npermissions = [\"blocks\"]\n";
        let text = format!(
            r#"(module
  (@custom "termzip-plugin" "{}")
  (memory (export "memory") 1)
  (func $spin (loop $l (br $l)))
  (start $spin)
  (func (export "tz_alloc") (param i32) (result i32) (i32.const 1024))
  (func (export "tz_on_block") (param i32 i32) (result i64) (i64.const 0)))"#,
            escape(manifest)
        );
        store
            .install(&wat::parse_str(text).unwrap(), Source::Local)
            .unwrap();
        store.enable("stuck").unwrap();

        let problems = host.sync();

        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(
            problems[0].contains("“Stuck” could not start"),
            "{problems:?}"
        );
        assert!(host.runtime.running().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
