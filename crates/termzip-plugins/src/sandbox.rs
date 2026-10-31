//! One plugin instance in a WebAssembly sandbox, and the calls into it.
//!
//! The sandbox is the whole point of the design: a plugin gets no imports, so it cannot reach the file system,
//! the network, a process or the clock. It is given bytes, it returns bytes. What it can still do is waste
//! resources, which is bounded here: **fuel** for every call (a loop ends when it runs out, whatever the
//! speed of the machine), a **memory** limit, a **stack** limit and a limit on the size of a reply.
//!
//! The instance is kept between calls so a plugin may remember something (a counter); its fuel is renewed
//! for every call. Memory it leaks is its own, up to the limit.

use termzip_plugin_api::{
    API_VERSION, Action, BlockEvent, CommandCall, MAX_REPLY_BYTES, Manifest, Permission,
};
use wasmi::{
    Config, Engine, Instance, Linker, Memory, Module, Store, StoreLimits, StoreLimitsBuilder,
    TrapCode, TypedFunc,
};

/// Work one call may do. Roughly one unit per WebAssembly instruction, so this is tens of milliseconds.
pub const FUEL_PER_CALL: u64 = 50_000_000;
/// The linear memory a plugin may use.
pub const MAX_MEMORY_BYTES: usize = 16 * 1024 * 1024;
/// The deepest the plugin may recurse, as slots of the interpreter's stack.
const MAX_STACK_SLOTS: usize = 64 * 1024;
/// More actions than this in one reply are ignored.
const MAX_ACTIONS: usize = 32;

/// Why a call went wrong. The reply of such a call is dropped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fault {
    /// The plugin ran out of fuel: it looped, or did more than a call may.
    OutOfFuel,
    /// The plugin trapped (an `unreachable`, a bad memory access, a stack overflow...).
    Trapped(String),
    /// The plugin returned something that cannot be a reply.
    BadReply(String),
}

impl Fault {
    /// A sentence for the user.
    pub fn describe(&self) -> String {
        match self {
            Self::OutOfFuel => {
                "it did more work than one call may (it may be stuck in a loop)".to_owned()
            }
            Self::Trapped(why) => format!("it crashed: {why}"),
            Self::BadReply(why) => format!("it sent a reply TermZip cannot use: {why}"),
        }
    }
}

pub struct Sandbox {
    store: Store<StoreLimits>,
    memory: Memory,
    alloc: TypedFunc<i32, i32>,
    on_block: Option<TypedFunc<(i32, i32), i64>>,
    run_command: Option<TypedFunc<(i32, i32), i64>>,
    permissions: Vec<Permission>,
}

impl std::fmt::Debug for Sandbox {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("Sandbox").finish_non_exhaustive()
    }
}

fn config() -> Config {
    let mut config = Config::default();
    config.consume_fuel(true);
    config.set_max_stack_height(MAX_STACK_SLOTS);
    // One memory is all a plugin needs, and one is all the limits below account for.
    config.wasm_multi_memory(false);
    config
}

fn trap_message(error: &wasmi::Error) -> Fault {
    match error.as_trap_code() {
        Some(TrapCode::OutOfFuel) => Fault::OutOfFuel,
        Some(code) => Fault::Trapped(code.trap_message().to_owned()),
        None => Fault::Trapped(error.to_string()),
    }
}

impl Sandbox {
    /// Compiles and starts the plugin. Everything that can go wrong is explained, so a plugin that cannot
    /// run is refused when it is enabled and not at the first command.
    pub fn load(bytes: &[u8], manifest: &Manifest) -> Result<Self, String> {
        let engine = Engine::new(&config());
        let module = Module::new(&engine, bytes)
            .map_err(|error| format!("TermZip cannot run this plugin: {error}"))?;

        let limits = StoreLimitsBuilder::new()
            .memory_size(MAX_MEMORY_BYTES)
            .table_elements(10_000)
            .instances(1)
            .tables(8)
            .memories(1)
            // A failed growth is the plugin's to handle; a trap would end it for no reason.
            .trap_on_grow_failure(false)
            .build();
        let mut store = Store::new(&engine, limits);
        store.limiter(|limits| limits);
        store
            .set_fuel(FUEL_PER_CALL)
            .map_err(|error| error.to_string())?;

        // No imports: nothing is defined for the plugin to call.
        let linker = Linker::new(&engine);
        let instance: Instance = linker
            .instantiate_and_start(&mut store, &module)
            .map_err(|error| format!("The plugin could not start: {error}"))?;

        let memory = instance
            .get_memory(&store, "memory")
            .ok_or("The plugin has no memory export.")?;
        let alloc = instance
            .get_func(&store, "tz_alloc")
            .and_then(|f| f.typed::<i32, i32>(&store).ok())
            .ok_or("The plugin's tz_alloc must take an i32 and return an i32.")?;
        let handler = |name: &str| {
            instance
                .get_func(&store, name)
                .and_then(|f| f.typed::<(i32, i32), i64>(&store).ok())
        };
        let on_block = manifest
            .permissions
            .contains(&Permission::Blocks)
            .then(|| {
                handler("tz_on_block")
                    .ok_or("The plugin's tz_on_block must take two i32 and return an i64.")
            })
            .transpose()?;
        let run_command = (!manifest.commands.is_empty())
            .then(|| {
                handler("tz_run_command")
                    .ok_or("The plugin's tz_run_command must take two i32 and return an i64.")
            })
            .transpose()?;

        Ok(Self {
            store,
            memory,
            alloc,
            on_block,
            run_command,
            permissions: manifest.permissions.clone(),
        })
    }

    /// Tells the plugin a command finished. `None` when it does not watch commands.
    pub fn on_block(&mut self, event: &BlockEvent) -> Option<Result<Vec<Action>, Fault>> {
        let func = self.on_block?;
        let mut event = event.clone();
        event.api = API_VERSION;
        if !self.permissions.contains(&Permission::Output) {
            event.output = None;
        }
        let input = serde_json::to_vec(&event).ok()?;
        Some(self.call(func, &input))
    }

    /// Runs one of the plugin's commands. `None` when it has none.
    pub fn run_command(&mut self, call: &CommandCall) -> Option<Result<Vec<Action>, Fault>> {
        let func = self.run_command?;
        let mut call = call.clone();
        call.api = API_VERSION;
        let input = serde_json::to_vec(&call).ok()?;
        Some(self.call(func, &input))
    }

    fn call(
        &mut self,
        func: TypedFunc<(i32, i32), i64>,
        input: &[u8],
    ) -> Result<Vec<Action>, Fault> {
        self.store
            .set_fuel(FUEL_PER_CALL)
            .map_err(|error| Fault::Trapped(error.to_string()))?;

        let length = i32::try_from(input.len())
            .map_err(|_| Fault::BadReply("the input is too big".to_owned()))?;
        let pointer = self
            .alloc
            .call(&mut self.store, length)
            .map_err(|e| trap_message(&e))?;
        if pointer == 0 {
            return Err(Fault::BadReply(
                "tz_alloc returned 0, so it had no room".to_owned(),
            ));
        }
        self.memory
            .write(&mut self.store, pointer as u32 as usize, input)
            .map_err(|_| Fault::BadReply("tz_alloc returned memory outside its own".to_owned()))?;

        let packed = func
            .call(&mut self.store, (pointer, length))
            .map_err(|e| trap_message(&e))? as u64;
        if packed == 0 {
            return Ok(Vec::new()); // nothing to say
        }
        let (at, len) = (
            (packed >> 32) as u32 as usize,
            (packed & 0xffff_ffff) as u32 as usize,
        );
        if len > MAX_REPLY_BYTES {
            return Err(Fault::BadReply(format!(
                "it is {len} bytes and a reply may be at most {MAX_REPLY_BYTES}"
            )));
        }
        let mut reply = vec![0u8; len];
        self.memory
            .read(&self.store, at, &mut reply)
            .map_err(|_| Fault::BadReply("it points outside the plugin's memory".to_owned()))?;
        parse_reply(&reply)
    }
}

/// Reads a reply: JSON with an `actions` list. An action this version does not know is skipped, so a plugin
/// written for a later version of the API still works; text that is not a reply at all is a fault.
pub fn parse_reply(bytes: &[u8]) -> Result<Vec<Action>, Fault> {
    let value: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|error| Fault::BadReply(format!("it is not JSON ({error})")))?;
    let Some(object) = value.as_object() else {
        return Err(Fault::BadReply("it is not a JSON object".to_owned()));
    };
    let Some(actions) = object.get("actions") else {
        return Ok(Vec::new());
    };
    let Some(list) = actions.as_array() else {
        return Err(Fault::BadReply("“actions” is not a list".to_owned()));
    };
    Ok(list
        .iter()
        .take(MAX_ACTIONS)
        .filter_map(|item| serde_json::from_value::<Action>(item.clone()).ok())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::fixtures::{escape, module};
    use termzip_plugin_api::Tone;

    const MANIFEST: &str = "api = 1\nid = \"t\"\nname = \"T\"\nversion = \"1.0.0\"\npermissions = [\"blocks\", \"notify\", \"output\"]\n\n[[commands]]\nid = \"go\"\ntitle = \"Go\"\n";

    fn manifest(text: &str) -> Manifest {
        toml::from_str(text).unwrap()
    }

    fn event() -> BlockEvent {
        BlockEvent {
            api: 1,
            index: 3,
            command: Some("make".to_owned()),
            cwd: Some("/w".to_owned()),
            exit_code: Some(2),
            duration_ms: Some(1500),
            output: Some("boom".to_owned()),
        }
    }

    /// A plugin that answers every call with this JSON, kept in its data segment.
    fn replying(json: &str) -> Vec<u8> {
        module(
            MANIFEST,
            &format!(
                r#"(memory (export "memory") 2)
  (data (i32.const 2048) "{}")
  (func (export "tz_alloc") (param i32) (result i32) (i32.const 1024))
  (func (export "tz_on_block") (param i32 i32) (result i64)
    (i64.or (i64.shl (i64.const 2048) (i64.const 32)) (i64.const {})))
  (func (export "tz_run_command") (param i32 i32) (result i64)
    (i64.or (i64.shl (i64.const 2048) (i64.const 32)) (i64.const {})))"#,
                escape(json),
                json.len(),
                json.len()
            ),
        )
    }

    fn load(bytes: &[u8]) -> Sandbox {
        Sandbox::load(bytes, &manifest(MANIFEST)).unwrap_or_else(|error| panic!("{error}"))
    }

    fn command() -> CommandCall {
        CommandCall {
            api: 1,
            command: "go".to_owned(),
            cwd: Some("/w".to_owned()),
        }
    }

    #[test]
    fn a_plugin_answers_with_actions() {
        let mut sandbox = load(&replying(
            r#"{"actions":[{"label":{"text":"slow","tone":"warn"}},{"status":"1 failed"},{"notify":{"text":"done"}}]}"#,
        ));

        let actions = sandbox.on_block(&event()).unwrap().unwrap();
        assert_eq!(
            actions,
            [
                Action::Label {
                    text: "slow".to_owned(),
                    tone: Tone::Warn
                },
                Action::Status("1 failed".to_owned()),
                Action::Notify {
                    text: "done".to_owned(),
                    tone: Tone::Info
                },
            ]
        );
        assert_eq!(sandbox.run_command(&command()).unwrap().unwrap().len(), 3);
    }

    /// The reply depends on the length of what the plugin was given, which proves what it was given.
    fn by_input_length(expected: usize) -> Vec<u8> {
        module(
            MANIFEST,
            &format!(
                r#"(memory (export "memory") 2)
  (data (i32.const 2048) "{matched}")
  (data (i32.const 2200) "{other}")
  (func (export "tz_alloc") (param i32) (result i32) (i32.const 1024))
  (func (export "tz_on_block") (param i32 i32) (result i64)
    (if (result i64) (i32.eq (local.get 1) (i32.const {expected}))
      (then (i64.or (i64.shl (i64.const 2048) (i64.const 32)) (i64.const {matched_len})))
      (else (i64.or (i64.shl (i64.const 2200) (i64.const 32)) (i64.const {other_len})))))
  (func (export "tz_run_command") (param i32 i32) (result i64) (i64.const 0))"#,
                matched = escape(STATUS_MATCH),
                other = escape(STATUS_OTHER),
                matched_len = STATUS_MATCH.len(),
                other_len = STATUS_OTHER.len(),
            ),
        )
    }

    const STATUS_MATCH: &str = r#"{"actions":[{"status":"match"}]}"#;
    const STATUS_OTHER: &str = r#"{"actions":[{"status":"other"}]}"#;

    fn status_of(result: Result<Vec<Action>, Fault>) -> String {
        match result.unwrap().as_slice() {
            [Action::Status(text)] => text.clone(),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn the_plugin_gets_the_event_as_json_and_output_only_with_the_permission() {
        let mut with_output = event();
        with_output.api = 1;
        let full = serde_json::to_string(&with_output).unwrap().len();
        let mut without = with_output.clone();
        without.output = None;
        let bare = serde_json::to_string(&without).unwrap().len();
        assert!(full > bare);

        // The plugin has the output permission: it receives the full event.
        let mut sandbox = load(&by_input_length(full));
        assert_eq!(status_of(sandbox.on_block(&event()).unwrap()), "match");

        // Without the permission the same event arrives without the output.
        let no_output = MANIFEST.replace(", \"output\"", "");
        let mut sandbox = Sandbox::load(&by_input_length(bare), &manifest(&no_output)).unwrap();
        assert_eq!(status_of(sandbox.on_block(&event()).unwrap()), "match");
    }

    #[test]
    fn a_plugin_only_gets_the_calls_it_asked_for() {
        let bytes = replying(r#"{"actions":[]}"#);
        // No `blocks` permission: it is never told about commands.
        let no_blocks = "api = 1\nid = \"t\"\nname = \"T\"\nversion = \"1.0.0\"\n\n[[commands]]\nid = \"go\"\ntitle = \"Go\"\n";
        let mut sandbox = Sandbox::load(&bytes, &manifest(no_blocks)).unwrap();
        assert!(sandbox.on_block(&event()).is_none());
        assert!(sandbox.run_command(&command()).is_some());

        // No commands: nothing to run.
        let only_blocks =
            "api = 1\nid = \"t\"\nname = \"T\"\nversion = \"1.0.0\"\npermissions = [\"blocks\"]\n";
        let mut sandbox = Sandbox::load(&bytes, &manifest(only_blocks)).unwrap();
        assert!(sandbox.on_block(&event()).is_some());
        assert!(sandbox.run_command(&command()).is_none());
    }

    fn with_body(on_block: &str, extra: &str) -> Vec<u8> {
        module(
            MANIFEST,
            &format!(
                r#"(memory (export "memory") 2)
  (data (i32.const 2048) "{alive}")
  (func (export "tz_alloc") (param i32) (result i32) (i32.const 1024))
  (func (export "tz_run_command") (param i32 i32) (result i64) (i64.const 0))
  {extra}
  (func (export "tz_on_block") (param i32 i32) (result i64) {on_block})"#,
                alive = escape(ALIVE),
            ),
        )
    }

    const ALIVE: &str = r#"{"actions":[{"status":"alive"}]}"#;

    /// WebAssembly text that evaluates to the packed pointer and length of `ALIVE` in `with_body`'s memory.
    fn reply_alive() -> String {
        format!(
            "(i64.or (i64.shl (i64.const 2048) (i64.const 32)) (i64.const {}))",
            ALIVE.len()
        )
    }

    #[test]
    fn a_loop_runs_out_of_fuel_instead_of_hanging_and_the_next_call_gets_fresh_fuel() {
        // The first call loops forever; later ones answer.
        let bytes = with_body(
            &format!(
                "(if (result i64) (i32.eqz (global.get $n))
                   (then (global.set $n (i32.const 1)) (loop $forever (br $forever)) (i64.const 0))
                   (else {}))",
                reply_alive()
            ),
            "(global $n (mut i32) (i32.const 0))",
        );
        let mut sandbox = load(&bytes);

        let started = std::time::Instant::now();
        assert_eq!(
            sandbox.on_block(&event()).unwrap().unwrap_err(),
            Fault::OutOfFuel
        );
        assert!(
            started.elapsed() < std::time::Duration::from_secs(10),
            "fuel must end the loop quickly"
        );
        assert_eq!(status_of(sandbox.on_block(&event()).unwrap()), "alive");
    }

    #[test]
    fn a_trap_is_a_fault_not_a_crash_of_the_host() {
        let mut sandbox = load(&with_body("(unreachable)", ""));
        let fault = sandbox.on_block(&event()).unwrap().unwrap_err();
        assert!(
            matches!(&fault, Fault::Trapped(why) if why.contains("unreachable")),
            "{fault:?}"
        );
        assert!(fault.describe().starts_with("it crashed"));

        // A memory access outside the plugin's memory is a trap too.
        let mut sandbox = load(&with_body(
            "(drop (i32.load (i32.const 99999999))) (i64.const 0)",
            "",
        ));
        assert!(matches!(
            sandbox.on_block(&event()).unwrap(),
            Err(Fault::Trapped(_))
        ));
    }

    #[test]
    fn endless_recursion_is_stopped() {
        let bytes = with_body("(call $again) (i64.const 0)", "(func $again (call $again))");
        let mut sandbox = load(&bytes);
        assert!(sandbox.on_block(&event()).unwrap().is_err());
    }

    #[test]
    fn a_plugin_cannot_take_more_memory_than_the_limit() {
        // Asking for more than 16 MiB to begin with is refused when the plugin is loaded.
        let greedy = module(
            MANIFEST,
            r#"(memory (export "memory") 300)
  (func (export "tz_alloc") (param i32) (result i32) (i32.const 1024))
  (func (export "tz_on_block") (param i32 i32) (result i64) (i64.const 0))
  (func (export "tz_run_command") (param i32 i32) (result i64) (i64.const 0))"#,
        );
        assert!(Sandbox::load(&greedy, &manifest(MANIFEST)).is_err());

        // Growing past the limit fails inside the plugin (memory.grow answers -1) instead of succeeding. It
        // replies "ok" for a refused growth and "grew" for a granted one.
        const REFUSED: &str = r#"{"actions":[{"status":"refused"}]}"#;
        const GREW: &str = r#"{"actions":[{"status":"grew"}]}"#;
        let growing = module(
            MANIFEST,
            &format!(
                r#"(memory (export "memory") 2)
  (data (i32.const 2048) "{}")
  (data (i32.const 2200) "{}")
  (func (export "tz_alloc") (param i32) (result i32) (i32.const 1024))
  (func (export "tz_run_command") (param i32 i32) (result i64) (i64.const 0))
  (func (export "tz_on_block") (param i32 i32) (result i64)
    (if (result i64) (i32.eq (memory.grow (i32.const 300)) (i32.const -1))
      (then (i64.or (i64.shl (i64.const 2048) (i64.const 32)) (i64.const {})))
      (else (i64.or (i64.shl (i64.const 2200) (i64.const 32)) (i64.const {})))))"#,
                escape(REFUSED),
                escape(GREW),
                REFUSED.len(),
                GREW.len()
            ),
        );
        let mut sandbox = load(&growing);
        assert_eq!(status_of(sandbox.on_block(&event()).unwrap()), "refused");
    }

    #[test]
    fn a_plugin_that_cannot_even_start_is_refused() {
        // A start function that never returns uses up its fuel.
        let stuck = module(
            MANIFEST,
            r#"(memory (export "memory") 1)
  (func $spin (loop $l (br $l)))
  (start $spin)
  (func (export "tz_alloc") (param i32) (result i32) (i32.const 1024))
  (func (export "tz_on_block") (param i32 i32) (result i64) (i64.const 0))
  (func (export "tz_run_command") (param i32 i32) (result i64) (i64.const 0))"#,
        );
        assert!(
            Sandbox::load(&stuck, &manifest(MANIFEST))
                .unwrap_err()
                .contains("could not start")
        );

        // Imports are refused too: there is nothing to give it.
        let importing = wat::parse_str(
            r#"(module (import "env" "log" (func)) (memory (export "memory") 1)
              (func (export "tz_alloc") (param i32) (result i32) (i32.const 1024))
              (func (export "tz_on_block") (param i32 i32) (result i64) (i64.const 0))
              (func (export "tz_run_command") (param i32 i32) (result i64) (i64.const 0)))"#,
        )
        .unwrap();
        assert!(Sandbox::load(&importing, &manifest(MANIFEST)).is_err());

        // So is something that is not a module at all.
        assert!(
            Sandbox::load(b"not wasm", &manifest(MANIFEST))
                .unwrap_err()
                .contains("cannot run")
        );
    }

    #[test]
    fn exports_with_the_wrong_shape_are_refused() {
        let wrong = module(
            MANIFEST,
            r#"(memory (export "memory") 1)
  (func (export "tz_alloc") (result i32) (i32.const 1024))
  (func (export "tz_on_block") (param i32 i32) (result i64) (i64.const 0))
  (func (export "tz_run_command") (param i32 i32) (result i64) (i64.const 0))"#,
        );
        assert!(
            Sandbox::load(&wrong, &manifest(MANIFEST))
                .unwrap_err()
                .contains("tz_alloc")
        );
    }

    #[test]
    fn a_reply_that_cannot_be_used_is_a_fault() {
        let bad = |packed: &str| with_body(&format!("(i64.const {packed})"), "");
        let fault = |bytes: Vec<u8>| load(&bytes).on_block(&event()).unwrap().unwrap_err();

        // Outside the memory (2 pages = 131072 bytes).
        let outside = (200_000u64 << 32) | 10;
        assert!(
            matches!(fault(bad(&outside.to_string())), Fault::BadReply(why) if why.contains("outside"))
        );
        // A reply bigger than the limit is dropped without reading it.
        let huge = (2048u64 << 32) | (MAX_REPLY_BYTES as u64 + 1);
        assert!(
            matches!(fault(bad(&huge.to_string())), Fault::BadReply(why) if why.contains("at most"))
        );
        // Not JSON (the data at 2048 begins `{"actions"...`; a cut one is not valid).
        let cut = (2048u64 << 32) | 5;
        assert!(
            matches!(fault(bad(&cut.to_string())), Fault::BadReply(why) if why.contains("not JSON"))
        );
        // No room: tz_alloc returns 0, or a pointer outside the memory.
        let no_room = module(
            MANIFEST,
            r#"(memory (export "memory") 1)
  (func (export "tz_alloc") (param i32) (result i32) (i32.const 0))
  (func (export "tz_on_block") (param i32 i32) (result i64) (i64.const 0))
  (func (export "tz_run_command") (param i32 i32) (result i64) (i64.const 0))"#,
        );
        assert!(matches!(
            load(&no_room).on_block(&event()).unwrap(),
            Err(Fault::BadReply(_))
        ));
        let wild = module(
            MANIFEST,
            r#"(memory (export "memory") 1)
  (func (export "tz_alloc") (param i32) (result i32) (i32.const -65536))
  (func (export "tz_on_block") (param i32 i32) (result i64) (i64.const 0))
  (func (export "tz_run_command") (param i32 i32) (result i64) (i64.const 0))"#,
        );
        assert!(matches!(
            load(&wild).on_block(&event()).unwrap(),
            Err(Fault::BadReply(_))
        ));
    }

    #[test]
    fn a_plugin_with_nothing_to_say_says_zero() {
        let mut sandbox = load(&with_body("(i64.const 0)", ""));
        assert_eq!(sandbox.on_block(&event()).unwrap().unwrap(), []);
    }

    #[test]
    fn replies_are_read_leniently_but_must_be_replies() {
        assert_eq!(parse_reply(b"{}").unwrap(), []);
        assert_eq!(parse_reply(br#"{"actions":[]}"#).unwrap(), []);
        // Actions of a kind this version does not know, or that are malformed, are skipped; the rest are kept.
        let mixed = br#"{"actions":[{"vibrate":1},{"status":"kept"},{"label":{}},"junk",7]}"#;
        assert_eq!(
            parse_reply(mixed).unwrap(),
            [Action::Status("kept".to_owned())]
        );

        for bad in [
            &b"nope"[..],
            b"[]",
            b"3",
            br#"{"actions":"x"}"#,
            br#"{"actions":{"a":1}}"#,
            b"",
        ] {
            assert!(
                parse_reply(bad).is_err(),
                "{:?}",
                String::from_utf8_lossy(bad)
            );
        }
        // Past 32 actions the rest is ignored.
        let many = format!(
            r#"{{"actions":[{}]}}"#,
            vec![r#"{"status":"x"}"#; 100].join(",")
        );
        assert_eq!(parse_reply(many.as_bytes()).unwrap().len(), MAX_ACTIONS);
    }

    #[test]
    fn a_plugin_keeps_its_state_between_calls() {
        // The n-th call answers with the n-th of three replies (a counter in a global), each in a 64-byte slot.
        let reply = |n: u32| format!(r#"{{"actions":[{{"status":"{n}"}}]}}"#);
        let bytes = module(
            MANIFEST,
            &format!(
                r#"(memory (export "memory") 2)
  (data (i32.const 2048) "{}")
  (data (i32.const 2112) "{}")
  (data (i32.const 2176) "{}")
  (global $n (mut i32) (i32.const 0))
  (func (export "tz_alloc") (param i32) (result i32) (i32.const 1024))
  (func (export "tz_run_command") (param i32 i32) (result i64) (i64.const 0))
  (func (export "tz_on_block") (param i32 i32) (result i64)
    (global.set $n (i32.add (global.get $n) (i32.const 1)))
    (i64.or
      (i64.shl (i64.extend_i32_u (i32.add (i32.const 2048) (i32.mul (i32.sub (global.get $n) (i32.const 1)) (i32.const 64)))) (i64.const 32))
      (i64.const {})))"#,
                escape(&reply(1)),
                escape(&reply(2)),
                escape(&reply(3)),
                reply(1).len()
            ),
        );
        let mut sandbox = load(&bytes);

        assert_eq!(status_of(sandbox.on_block(&event()).unwrap()), "1");
        assert_eq!(status_of(sandbox.on_block(&event()).unwrap()), "2");
        assert_eq!(status_of(sandbox.on_block(&event()).unwrap()), "3");
    }
}
