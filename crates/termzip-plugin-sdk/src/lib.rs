//! Write a TermZip plugin in Rust.
//!
//! A plugin is one WebAssembly file (see `docs/plugin-api.md`). With this crate it is a few lines:
//!
//! ```ignore
//! use termzip_plugin_sdk::prelude::*;
//!
//! // plugin.toml sits next to Cargo.toml; it travels inside the .wasm file.
//! termzip_plugin_sdk::manifest!("../plugin.toml");
//!
//! #[derive(Default)]
//! struct Slow;
//!
//! impl Plugin for Slow {
//!     fn on_block(&mut self, block: &Block) -> Vec<Action> {
//!         match block.duration_ms {
//!             Some(ms) if ms > 10_000 => vec![label("slow", Tone::Warn)],
//!             _ => vec![],
//!         }
//!     }
//! }
//!
//! termzip_plugin_sdk::export!(Slow);
//! ```
//!
//! Build it for `wasm32-unknown-unknown` as a `cdylib`. The plugin runs in a sandbox with no imports: it cannot
//! reach files, the network, processes or the clock, so there is nothing for this crate to offer for them.

use termzip_plugin_api::API_VERSION;

pub use termzip_plugin_api::{
    API_VERSION as API, Action, BlockEvent as Block, CommandCall as Command, MAX_LABEL_CHARS,
    MAX_NOTIFY_CHARS, MAX_OUTPUT_BYTES, MAX_STATUS_CHARS, MAX_TYPE_BYTES, Permission, Reply, Tone,
};

/// What most plugins import.
pub mod prelude {
    pub use crate::{Action, Block, Command, Plugin, Tone, label, notify, open, status, type_text};
}

/// A plugin. Implement what it does; the rest has a default that does nothing.
///
/// There is one instance for as long as the plugin runs, so it may keep state between calls (a counter).
pub trait Plugin: Default + 'static {
    /// A command finished. Needs the `blocks` permission in the manifest, and `output` to see what it printed.
    /// The actions can only label the block, show a message or set the status item.
    fn on_block(&mut self, block: &Block) -> Vec<Action> {
        let _ = block;
        Vec::new()
    }

    /// The user ran one of the plugin's commands from the palette. Only here may a plugin type text (`type`
    /// permission) or open an address (`open` permission).
    fn run_command(&mut self, call: &Command) -> Vec<Action> {
        let _ = call;
        Vec::new()
    }
}

/// A badge on the block the event was about.
pub fn label(text: impl Into<String>, tone: Tone) -> Action {
    Action::Label {
        text: text.into(),
        tone,
    }
}

/// A short message. Needs the `notify` permission.
pub fn notify(text: impl Into<String>, tone: Tone) -> Action {
    Action::Notify {
        text: text.into(),
        tone,
    }
}

/// The text of the plugin's item in the status bar; an empty text removes it.
pub fn status(text: impl Into<String>) -> Action {
    Action::Status(text.into())
}

/// Text to type into the terminal the command was run from, without Enter. Needs the `type` permission.
pub fn type_text(text: impl Into<String>) -> Action {
    Action::Type(text.into())
}

/// Opens an `http` or `https` address. Needs the `open` permission.
pub fn open(url: impl Into<String>) -> Action {
    Action::Open(url.into())
}

/// Turns the bytes of an event into a call of the plugin and its answer back into bytes. Everything the glue
/// does with pointers is outside this type, so it is the same on every target and is tested on the host.
pub struct Runner<P> {
    plugin: P,
}

impl<P: Plugin> Runner<P> {
    pub fn new() -> Self {
        Self {
            plugin: P::default(),
        }
    }

    /// The reply to an event, as JSON. Input that is not an event gets an empty reply, never a crash.
    pub fn block(&mut self, input: &[u8]) -> Vec<u8> {
        let actions = match serde_json::from_slice::<Block>(input) {
            Ok(block) => self.plugin.on_block(&block),
            Err(_) => Vec::new(),
        };
        encode(actions)
    }

    /// The reply to a command, as JSON.
    pub fn command(&mut self, input: &[u8]) -> Vec<u8> {
        let actions = match serde_json::from_slice::<Command>(input) {
            Ok(call) => self.plugin.run_command(&call),
            Err(_) => Vec::new(),
        };
        encode(actions)
    }
}

impl<P: Plugin> Default for Runner<P> {
    fn default() -> Self {
        Self::new()
    }
}

fn encode(actions: Vec<Action>) -> Vec<u8> {
    serde_json::to_vec(&Reply { actions }).unwrap_or_else(|_| b"{\"actions\":[]}".to_vec())
}

/// Embeds the manifest in the plugin file, in the custom section the app reads. The path is relative to the
/// source file, like `include_bytes!`.
#[macro_export]
macro_rules! manifest {
    ($path:literal) => {
        #[cfg_attr(target_arch = "wasm32", unsafe(link_section = "termzip-plugin"))]
        #[used]
        static __TERMZIP_MANIFEST: [u8; include_bytes!($path).len()] = *include_bytes!($path);
    };
}

/// Exports the functions the app calls (`tz_alloc`, `tz_on_block`, `tz_run_command`) for a type that
/// implements [`Plugin`].
#[macro_export]
macro_rules! export {
    ($plugin:ty) => {
        ::std::thread_local! {
            static __TERMZIP_RUNNER: ::std::cell::RefCell<$crate::Runner<$plugin>> =
                ::std::cell::RefCell::new($crate::Runner::new());
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn tz_alloc(len: i32) -> i32 {
            $crate::__private::alloc(len as u32) as i32
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn tz_on_block(ptr: i32, len: i32) -> i64 {
            $crate::__private::reply(match $crate::__private::input(ptr as u32, len as u32) {
                Some(input) => __TERMZIP_RUNNER.with(|runner| runner.borrow_mut().block(&input)),
                None => Vec::new(),
            }) as i64
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn tz_run_command(ptr: i32, len: i32) -> i64 {
            $crate::__private::reply(match $crate::__private::input(ptr as u32, len as u32) {
                Some(input) => __TERMZIP_RUNNER.with(|runner| runner.borrow_mut().command(&input)),
                None => Vec::new(),
            }) as i64
        }
    };
}

/// What `export!` uses to move bytes in and out of the plugin's memory. Not part of the API.
#[doc(hidden)]
pub mod __private {
    use std::cell::RefCell;

    thread_local! {
        // The buffer the app writes an event into, and the one the reply is read from. Each call replaces the last,
        // so a plugin never leaks memory call after call.
        static INPUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
        static REPLY: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    }

    /// Room for `len` bytes, which the app fills. Returns where it is.
    pub fn alloc(len: u32) -> u32 {
        INPUT.with(|input| {
            let mut input = input.borrow_mut();
            *input = vec![0; len as usize];
            input.as_mut_ptr() as usize as u32
        })
    }

    /// The event the app wrote, if `ptr` and `len` name what `alloc` handed out.
    pub fn input(ptr: u32, len: u32) -> Option<Vec<u8>> {
        INPUT.with(|input| {
            let input = input.borrow();
            let ours = input.as_ptr() as usize as u32 == ptr && len as usize <= input.len();
            ours.then(|| input[..len as usize].to_vec())
        })
    }

    /// Keeps the reply where the app can read it and says where: the pointer in the high half, the length in the
    /// low half. Zero means there is nothing to say.
    pub fn reply(bytes: Vec<u8>) -> u64 {
        if bytes.is_empty() {
            return 0;
        }
        REPLY.with(|reply| {
            *reply.borrow_mut() = bytes;
            let reply = reply.borrow();
            pack(reply.as_ptr() as usize as u32, reply.len() as u32)
        })
    }

    pub fn pack(ptr: u32, len: u32) -> u64 {
        (u64::from(ptr) << 32) | u64::from(len)
    }
}

const _: () = assert!(API_VERSION == 1, "this SDK was written for plugin API 1");

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Counter {
        seen: u32,
    }

    impl Plugin for Counter {
        fn on_block(&mut self, block: &Block) -> Vec<Action> {
            self.seen += 1;
            let mut actions = vec![status(format!("{} seen", self.seen))];
            if block.exit_code.is_some_and(|code| code != 0) {
                actions.push(label("failed", Tone::Err));
            }
            actions
        }

        fn run_command(&mut self, call: &Command) -> Vec<Action> {
            vec![
                type_text(format!("echo {}", call.command)),
                open("https://example.com"),
            ]
        }
    }

    const EVENT: &str =
        r#"{"api":1,"index":3,"command":"make","cwd":"/w","exit_code":2,"duration_ms":50}"#;

    #[test]
    fn a_plugin_is_a_function_from_an_event_to_actions() {
        let mut runner = Runner::<Counter>::new();

        let reply: Reply = serde_json::from_slice(&runner.block(EVENT.as_bytes())).unwrap();

        assert_eq!(
            reply.actions,
            [
                Action::Status("1 seen".to_owned()),
                Action::Label {
                    text: "failed".to_owned(),
                    tone: Tone::Err
                },
            ]
        );
    }

    #[test]
    fn the_plugin_keeps_its_state_between_calls() {
        let mut runner = Runner::<Counter>::new();
        runner.block(EVENT.as_bytes());
        let second: Reply = serde_json::from_slice(&runner.block(EVENT.as_bytes())).unwrap();

        assert_eq!(second.actions[0], Action::Status("2 seen".to_owned()));
    }

    #[test]
    fn a_command_is_answered_with_what_the_plugin_returns() {
        let mut runner = Runner::<Counter>::new();
        let call = br#"{"api":1,"command":"go","cwd":null}"#;

        let reply: Reply = serde_json::from_slice(&runner.command(call)).unwrap();

        assert_eq!(
            reply.actions,
            [
                Action::Type("echo go".to_owned()),
                Action::Open("https://example.com".to_owned())
            ]
        );
    }

    #[test]
    fn input_that_is_not_an_event_gets_an_empty_reply_not_a_crash() {
        let mut runner = Runner::<Counter>::new();

        for junk in [&b""[..], b"nope", b"[]", br#"{"api":1}"#, b"\xff\xfe"] {
            assert_eq!(runner.block(junk), br#"{"actions":[]}"#, "{junk:?}");
            assert_eq!(runner.command(junk), br#"{"actions":[]}"#, "{junk:?}");
        }
        // And the plugin was not called for it.
        let reply: Reply = serde_json::from_slice(&runner.block(EVENT.as_bytes())).unwrap();
        assert_eq!(reply.actions[0], Action::Status("1 seen".to_owned()));
    }

    #[test]
    fn a_plugin_that_does_nothing_has_nothing_to_write() {
        #[derive(Default)]
        struct Quiet;
        impl Plugin for Quiet {}

        let mut runner = Runner::<Quiet>::new();
        assert_eq!(runner.block(EVENT.as_bytes()), br#"{"actions":[]}"#);
    }

    #[test]
    fn the_helpers_build_the_actions_of_the_api() {
        assert_eq!(
            label("x", Tone::Ok),
            Action::Label {
                text: "x".to_owned(),
                tone: Tone::Ok
            }
        );
        assert_eq!(
            notify("y", Tone::Warn),
            Action::Notify {
                text: "y".to_owned(),
                tone: Tone::Warn
            }
        );
        assert_eq!(status("z"), Action::Status("z".to_owned()));
        assert_eq!(type_text("t"), Action::Type("t".to_owned()));
        assert_eq!(
            open("https://a.test"),
            Action::Open("https://a.test".to_owned())
        );
    }

    #[test]
    fn the_reply_is_packed_the_way_the_app_unpacks_it() {
        assert_eq!(
            __private::pack(0x0001_0000, 27),
            (0x0001_0000u64 << 32) | 27
        );
        let packed = __private::pack(u32::MAX, u32::MAX);
        assert_eq!((packed >> 32) as u32, u32::MAX);
        assert_eq!(packed as u32, u32::MAX);
        // Nothing to say is zero, which the app reads as an empty reply.
        assert_eq!(__private::reply(Vec::new()), 0);
    }

    #[test]
    fn the_input_buffer_is_only_handed_back_for_what_alloc_gave_out() {
        let ptr = __private::alloc(8);

        assert!(__private::input(ptr, 8).is_some());
        assert_eq!(__private::input(ptr, 4).map(|bytes| bytes.len()), Some(4));
        // A wrong address or a length past the buffer is refused, not read.
        assert!(__private::input(ptr.wrapping_add(1), 8).is_none());
        assert!(__private::input(ptr, 9).is_none());
        // A new allocation replaces the old one.
        let again = __private::alloc(16);
        assert!(__private::input(again, 16).is_some());
    }
}
