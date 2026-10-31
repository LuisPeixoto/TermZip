//! The example plugins of `examples/plugins`, built for WebAssembly, against the real host: the same reader,
//! store, sandbox, action checks and link matcher the app uses. This is what proves that what the SDK writes is
//! a file the app accepts and that the examples do what their documentation says.
//!
//! The tests are ignored by default because they need the files built first (CI does both steps):
//!
//! ```text
//! cargo build --release --target wasm32-unknown-unknown --manifest-path examples/plugins/Cargo.toml
//! cargo test -p termzip-plugins --test examples -- --ignored
//! ```
//!
//! `TERMZIP_EXAMPLE_PLUGINS` names another folder with the `.wasm` files.

use std::path::PathBuf;

use termzip_plugin_api::{Action, BlockEvent, CommandCall, Manifest, Permission, Tone};
use termzip_plugins::{LinkMatcher, PluginStore, Sandbox, Source, check_actions, read_module};

const EXAMPLES: [(&str, &[Permission]); 4] = [
    (
        "long-command-notify",
        &[Permission::Blocks, Permission::Notify],
    ),
    ("exit-status", &[Permission::Blocks]),
    ("issue-links", &[]),
    ("snippets", &[Permission::Type]),
];

fn built(id: &str) -> Vec<u8> {
    let folder = std::env::var_os("TERMZIP_EXAMPLE_PLUGINS")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../examples/plugins/target/wasm32-unknown-unknown/release")
        });
    let path = folder.join(format!("{}.wasm", id.replace('-', "_")));
    std::fs::read(&path).unwrap_or_else(|error| {
        panic!(
            "{} is not there ({error}). Build the examples first: cargo build --release --target \
             wasm32-unknown-unknown --manifest-path examples/plugins/Cargo.toml",
            path.display()
        )
    })
}

fn load(id: &str) -> (Manifest, Sandbox) {
    let bytes = built(id);
    let manifest = read_module(&bytes)
        .unwrap_or_else(|error| panic!("{id}: {error}"))
        .manifest;
    let sandbox = Sandbox::load(&bytes, &manifest).unwrap_or_else(|error| panic!("{id}: {error}"));
    (manifest, sandbox)
}

fn finished(command: &str, exit_code: Option<i32>, duration_ms: Option<u64>) -> BlockEvent {
    BlockEvent {
        api: 1,
        index: 0,
        command: Some(command.to_owned()),
        cwd: Some("/work".to_owned()),
        exit_code,
        duration_ms,
        output: None,
    }
}

/// What the app keeps of the answer to a finished command.
fn on_block(manifest: &Manifest, sandbox: &mut Sandbox, event: &BlockEvent) -> Vec<Action> {
    let actions = sandbox
        .on_block(event)
        .expect("the plugin watches commands")
        .expect("the plugin answered");
    let checked = check_actions(actions, &manifest.permissions, false);
    assert_eq!(
        checked.dropped, 0,
        "the app dropped something the plugin said"
    );
    checked.actions
}

#[test]
#[ignore = "needs the examples built for WebAssembly (see the top of this file)"]
fn every_example_is_a_file_the_app_accepts_and_installs() {
    let folder = std::env::temp_dir().join(format!("termzip-examples-{}", std::process::id()));
    let store = PluginStore::new(&folder);

    for (id, permissions) in EXAMPLES {
        let bytes = built(id);
        let manifest = read_module(&bytes)
            .unwrap_or_else(|error| panic!("{id}: {error}"))
            .manifest;
        assert_eq!(manifest.id, id);
        assert_eq!(manifest.permissions, permissions, "{id}");
        assert!(bytes.len() < 256 * 1024, "{id} is {} bytes", bytes.len());

        let installed = store.install(&bytes, Source::Local).unwrap();
        assert!(!installed.enabled, "{id} must wait for approval");
        let enabled = store.enable(id).unwrap();
        assert!(enabled.enabled && !enabled.changed, "{id}");
        assert_eq!(store.approved_bytes(id).unwrap(), bytes);
        Sandbox::load(&bytes, &manifest).unwrap_or_else(|error| panic!("{id}: {error}"));
    }

    let listing = store.list();
    assert!(listing.problems.is_empty(), "{:?}", listing.problems);
    assert_eq!(listing.plugins.len(), EXAMPLES.len());
    assert!(listing.plugins.iter().all(|info| info.enabled));

    std::fs::remove_dir_all(&folder).unwrap();
}

#[test]
#[ignore = "needs the examples built for WebAssembly (see the top of this file)"]
fn long_command_notify_speaks_up_only_for_a_long_command() {
    let (manifest, mut sandbox) = load("long-command-notify");

    assert!(on_block(&manifest, &mut sandbox, &finished("ls", Some(0), Some(40))).is_empty());
    assert_eq!(
        on_block(
            &manifest,
            &mut sandbox,
            &finished("cargo build", Some(0), Some(12_400))
        ),
        [
            Action::Label {
                text: "slow".to_owned(),
                tone: Tone::Warn
            },
            Action::Notify {
                text: "cargo build finished in 12 s".to_owned(),
                tone: Tone::Ok
            },
        ]
    );
    let failed = on_block(
        &manifest,
        &mut sandbox,
        &finished("make", Some(2), Some(75_000)),
    );
    assert_eq!(
        failed[1],
        Action::Notify {
            text: "make failed (exit 2) after 1 min 15 s".to_owned(),
            tone: Tone::Err
        }
    );
}

#[test]
#[ignore = "needs the examples built for WebAssembly (see the top of this file)"]
fn exit_status_marks_failures_and_counts_them_between_calls() {
    let (manifest, mut sandbox) = load("exit-status");

    assert!(on_block(&manifest, &mut sandbox, &finished("true", Some(0), Some(1))).is_empty());
    assert_eq!(
        on_block(
            &manifest,
            &mut sandbox,
            &finished("false", Some(1), Some(1))
        ),
        [
            Action::Label {
                text: "exit 1".to_owned(),
                tone: Tone::Err
            },
            Action::Status("1 failed".to_owned()),
        ]
    );
    assert_eq!(
        on_block(
            &manifest,
            &mut sandbox,
            &finished("sleep 99", Some(130), Some(900))
        ),
        [Action::Label {
            text: "interrupted".to_owned(),
            tone: Tone::Warn
        }]
    );
    let again = on_block(
        &manifest,
        &mut sandbox,
        &finished("false", Some(3), Some(1)),
    );
    assert_eq!(again[1], Action::Status("2 failed".to_owned()));
}

#[test]
#[ignore = "needs the examples built for WebAssembly (see the top of this file)"]
fn issue_links_turns_issue_numbers_into_links_and_runs_no_code() {
    let (manifest, mut sandbox) = load("issue-links");
    // It has no permissions and no commands, so the app never calls into it.
    assert!(
        sandbox
            .on_block(&finished("ls", Some(0), Some(1)))
            .is_none()
    );
    assert!(
        sandbox
            .run_command(&CommandCall {
                api: 1,
                command: "x".to_owned(),
                cwd: None
            })
            .is_none()
    );

    let matcher = LinkMatcher::new([&manifest]);
    let line = "fixed #42 and LuisPeixoto/TermZip#7 but not abc#12 or #9x";
    let found = matcher.find(line);

    let urls: Vec<_> = found.iter().map(|link| link.url.as_str()).collect();
    assert_eq!(
        urls,
        [
            "https://github.com/LuisPeixoto/TermZip/issues/42",
            "https://github.com/LuisPeixoto/TermZip/issues/7"
        ]
    );
    assert_eq!(&line[found[0].start..found[0].end], "#42");
    assert_eq!(&line[found[1].start..found[1].end], "LuisPeixoto/TermZip#7");
}

#[test]
#[ignore = "needs the examples built for WebAssembly (see the top of this file)"]
fn snippets_type_each_command_and_never_press_enter() {
    let (manifest, mut sandbox) = load("snippets");
    assert!(
        sandbox
            .on_block(&finished("ls", Some(0), Some(1)))
            .is_none()
    );

    for command in &manifest.commands {
        let call = CommandCall {
            api: 1,
            command: command.id.clone(),
            cwd: Some("/work".to_owned()),
        };
        let actions = sandbox.run_command(&call).unwrap().unwrap();
        // The answer to a command the user ran may type; the app keeps it whole.
        let checked = check_actions(actions, &manifest.permissions, true);
        assert_eq!(checked.dropped, 0, "{}", command.id);
        let [Action::Type(text)] = checked.actions.as_slice() else {
            panic!(
                "{}: expected one text to type, got {:?}",
                command.id, checked.actions
            )
        };
        assert!(
            !text.is_empty() && !text.contains(['\n', '\r']),
            "{}",
            command.id
        );
    }

    let call = CommandCall {
        api: 1,
        command: "git-status".to_owned(),
        cwd: None,
    };
    assert_eq!(
        sandbox.run_command(&call).unwrap().unwrap(),
        [Action::Type("git status".to_owned())]
    );
    // An id the manifest does not declare types nothing.
    let unknown = CommandCall {
        api: 1,
        command: "rm-everything".to_owned(),
        cwd: None,
    };
    assert!(sandbox.run_command(&unknown).unwrap().unwrap().is_empty());
}
