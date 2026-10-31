# Plugin API

This is the contract between TermZip and a plugin, for people who write plugins. To use one, read
[plugins](plugins.md). The current version is **plugin API 1**.

A plugin is one WebAssembly file. It receives events as JSON, answers with a list of actions as JSON, and cannot do
anything else: it has no imports, so there is no function it could call to reach your files, the network, a process or
the clock. Any language that builds a core WebAssembly module can write one; for Rust there is an
[SDK](#the-rust-sdk) that hides everything below.

## The file

- A **core WebAssembly module** (not a component), at most 4 MiB.
- **No imports.** A module that imports anything is refused: “A plugin cannot call anything outside its sandbox.”
- One exported **`memory`** (a single linear memory; at most 16 MiB at run time).
- Exported functions, all taking and returning plain integers:

  | Export | Signature | Required when |
  |---|---|---|
  | `tz_alloc` | `(len: i32) -> i32` | always |
  | `tz_on_block` | `(ptr: i32, len: i32) -> i64` | the manifest has the `blocks` permission |
  | `tz_run_command` | `(ptr: i32, len: i32) -> i64` | the manifest lists commands |

  Extra exports are ignored.
- Exactly one **custom section named `termzip-plugin`** holding the [manifest](#the-manifest), as UTF-8 TOML, at most
  16 KiB.

The file is read without running a single instruction of it: its structure, manifest, imports and exports are checked
first, and a problem is reported as a sentence.

## The manifest

```toml
api = 1                          # the API this plugin was written for
id = "long-command-notify"       # a-z, 0-9 and single "-"; up to 64 characters; names the file and the approval
name = "Long command notifier"   # up to 64 characters
version = "0.1.0"                # semantic version: 1.2.3
description = "Tells you when a command that ran for over ten seconds has finished."   # up to 280 characters
author = "Luis Peixoto"          # optional, up to 128 characters
license = "MIT"                  # optional, up to 128 characters
homepage = "https://example.com" # optional, http or https
permissions = ["blocks", "notify"]

[[commands]]                     # an entry in the command palette
id = "say-hello"                 # a-z, 0-9 and "-"; what the plugin is told when the command runs
title = "Say hello"              # up to 64 characters

[[links]]                        # text in the output that becomes a link
pattern = '\B#([0-9]+)\b'        # a regular expression, up to 256 characters
url = "https://github.com/me/repo/issues/{1}"
```

Only `api`, `id`, `name` and `version` are required. The installed file must be named `<id>.wasm`.

A plugin has to do something: it needs a command, a link pattern or the `blocks` permission. A manifest with more than
32 commands or 16 link patterns is refused, and command ids must be unique.

### Permissions

| Permission | Lets the plugin | Notes |
|---|---|---|
| `blocks` | receive every finished command | its text, exit code, duration and folder |
| `output` | also receive what the command printed | needs `blocks`; at most 64 KiB |
| `type` | answer a palette command with text to type | needs at least one command |
| `notify` | show a message | |
| `open` | answer a palette command with a web address to open | needs at least one command |

There is nothing else to ask for: no files, network, processes or clock exist for a plugin. Commands, labels, status
items and link patterns need no permission. The user sees one sentence per permission before turning the plugin on.

### Link patterns

The app does the matching, not your code, so a pattern costs nothing at run time and needs no WebAssembly.

- The syntax is the Rust [`regex`](https://docs.rs/regex) crate's: no look-around and no back-references. Matching runs
  in time linear in the line, so no pattern can make the terminal slow. A pattern that matches the empty string is
  refused (it would link every line).
- The link covers the **whole match**. In the address, `{0}` is the whole match and `{1}`, `{2}`… are the groups; a
  number past the groups of the pattern is an error. A group that did not take part is empty.
- The text that is put into the address comes from the terminal, so it is **percent-encoded**, including `/`: one
  match is always one path segment. That is why a placeholder is **not allowed in the host name**, only in the path or
  the query. Use one group per segment (`{1}/{2}`), not one group for `owner/repo`.
- An address starts with `http://` or `https://` and has no spaces or control characters.
- Only the first 8 KiB of a line are searched, at most 32 links are made per line, and where two matches overlap the
  earlier one wins. A link opens with ⌘+click like any other [link](links.md).

## Calling a plugin

Each call is one event in and at most one reply out:

1. The app calls `tz_alloc(len)`. The plugin returns a pointer to `len` bytes it can spare in its `memory`. Returning 0
   is a failure.
2. The app writes the event, as JSON, at that pointer.
3. The app calls `tz_on_block(ptr, len)` for a finished command, or `tz_run_command(ptr, len)` for a command from the
   palette.
4. The plugin returns an `i64`. **0 means “nothing to say”.** Anything else packs where the reply is: the pointer in
   the high 32 bits and the length in the low 32 bits (`(ptr << 32) | len`), of a JSON reply in its `memory`. The reply
   must stay readable until the next call.

The instance stays alive while the plugin is on, so globals keep their values between calls. They are lost when the
plugin is turned off or TermZip quits. Calls to one plugin never overlap.

### A finished command: `tz_on_block`

```json
{"api":1,"index":3,"command":"make","cwd":"/work/app","exit_code":2,"duration_ms":12400}
```

| Field | Meaning |
|---|---|
| `api` | `1` |
| `index` | counts the commands of one terminal from 0; tells a block apart from the others |
| `command` | the command as it was typed, or `null` if it was not captured |
| `cwd` | the folder it ran in, or `null` |
| `exit_code` | or `null` if unknown |
| `duration_ms` | or `null` |
| `output` | **only with the `output` permission**: what it printed, cut to 64 KiB |

Read the event leniently: a later version of the API may add fields.

### A palette command: `tz_run_command`

```json
{"api":1,"command":"say-hello","cwd":"/work/app"}
```

`command` is the `id` from the manifest; `cwd` is the folder of the terminal it was run from, or `null`.

### The reply

```json
{"actions":[
  {"label":  {"text": "slow", "tone": "warn"}},
  {"notify": {"text": "make failed after 12 s", "tone": "err"}},
  {"status": "3 failed"},
  {"type":   "git status"},
  {"open":   "https://example.com/docs"}
]}
```

`tone` is one of `info` (the default), `ok`, `warn`, `err`. A reply that is not a JSON object, is over 64 KiB or points
outside the plugin's memory is a failure. At most 32 actions are read, and **an action this version does not know is
skipped**, so a plugin written for a later API keeps working.

Every action is checked by the app before anything happens:

| Action | Allowed | Limit and cleaning |
|---|---|---|
| `label` | only as the answer to a finished command | 24 characters; one per plugin per block, up to four labels per block |
| `notify` | with `notify`; in either answer | 200 characters |
| `status` | always | 48 characters; the plugin's item in the status bar; an empty text removes it |
| `type` | with `type`, **only as the answer to a command the user ran** | 4 KiB; line breaks and control characters removed; pasted, never followed by Enter |
| `open` | with `open`, **only as the answer to a command the user ran** | `http` or `https` with an ASCII host and no `user@` part; at most 2 KiB |

Text loses control characters and the invisible characters that reorder or hide text (bidirectional marks, zero-width
characters) and is cut to its limit. It is shown as plain text, never as markup. An action that is not allowed is
dropped without a message.

### Limits and failures

| What | Limit |
|---|---|
| Work per call | 50 million units of fuel; the fuel is refilled for every call |
| Memory | 16 MiB; a `memory.grow` that would pass it returns -1 for the plugin to handle |
| Stack | 64 Ki slots of the interpreter's stack |
| Reply | 64 KiB, at most 32 actions |
| Queue | 32 pending events per plugin; more are dropped for that plugin |

A trap, running out of fuel or a reply that cannot be used makes that call fail: its reply is dropped and the user sees
the reason. **Three failures in a row** stop the plugin until the user turns it off and on; a call that works resets
the count.

## Versioning

The `api` in the manifest names the version the plugin was written for. TermZip 0.4 understands API 1 and refuses
another one with “This plugin was written for plugin API N; this TermZip understands API 1.”

- **Inside `api = 1`** nothing a plugin relies on changes. Additions are optional: new fields in events and in the
  manifest, new actions (skipped by hosts that do not know them), new optional exports and new permissions. A plugin
  that ignores unknown fields keeps working on every TermZip that understands API 1.
- **A breaking change is `api = 2`.** From then on TermZip understands the current API **and the previous one** for at
  least one minor release, so there is time to move. A removal is announced in the changelog before it happens.
- The limits above can be raised inside an API version, never lowered.

## The Rust SDK

[`termzip-plugin-sdk`](../crates/termzip-plugin-sdk) writes the ABI for you: a `Plugin` trait, typed events and
actions, a macro that embeds the manifest and a macro that exports the functions. A whole plugin:

```toml
# Cargo.toml
[package]
name = "my-plugin"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]

[dependencies]
termzip-plugin-sdk = { git = "https://github.com/LuisPeixoto/TermZip", tag = "v0.4.0" }

[profile.release]
opt-level = "s"
lto = true
strip = true
panic = "abort"
```

```toml
# plugin.toml, next to Cargo.toml
api = 1
id = "my-plugin"
name = "My plugin"
version = "0.1.0"
permissions = ["blocks"]
```

```rust
// src/lib.rs
use termzip_plugin_sdk::prelude::*;

termzip_plugin_sdk::manifest!("../plugin.toml");

#[derive(Default)]
struct Mine {
    seen: u32,
}

impl Plugin for Mine {
    fn on_block(&mut self, block: &Block) -> Vec<Action> {
        self.seen += 1;
        match block.exit_code {
            Some(code) if code != 0 => vec![label(format!("exit {code}"), Tone::Err)],
            _ => vec![status(format!("{} commands", self.seen))],
        }
    }
}

termzip_plugin_sdk::export!(Mine);
```

```sh
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown
# target/wasm32-unknown-unknown/release/my_plugin.wasm
```

Then **Preferences → Plugins → Install from file…**, and turn it on. The file keeps whatever name it was built with; the
app copies it into its plugins folder as `<id>.wasm`.

What the SDK gives you:

- `Plugin` has `on_block(&mut self, &Block)` and `run_command(&mut self, &Command)`, both returning `Vec<Action>` and
  both doing nothing by default. The type must be `Default`; one instance lives as long as the plugin is on.
- `label`, `notify`, `status`, `type_text` and `open` build the actions; `Tone` is `Info`, `Ok`, `Warn` or `Err`.
- `Block` and `Command` are the events above. The limits (`MAX_LABEL_CHARS`, `MAX_NOTIFY_CHARS`, `MAX_STATUS_CHARS`,
  `MAX_TYPE_BYTES`, `MAX_OUTPUT_BYTES`) are exported from the crate root.
- Input that is not a valid event gets an empty reply instead of a crash.
- `manifest!` embeds `plugin.toml` in the `termzip-plugin` section; its path is relative to the source file, like
  `include_bytes!`. `export!` writes `tz_alloc`, `tz_on_block` and `tz_run_command`.

The macros write `unsafe` code into **your** crate (the exports must be `no_mangle`), so a plugin crate cannot forbid
`unsafe_code`, and it needs Rust 1.82 or later. The SDK itself contains none.

### Testing

Plain unit tests work: a `Plugin` is just a type, so call `on_block` and `run_command` and compare the actions. To test
the byte-level path too, `Runner::<Mine>::new().block(json_bytes)` returns the reply JSON exactly as the app would
read it. The [examples](../examples/plugins) do both, and the repository's CI also loads the built `.wasm` files into
the app's own reader and sandbox.

To see it in the real app without touching your own settings, start TermZip with a private data folder:

```sh
TERMZIP_HOME=/tmp/termzip-plugin-try npm run tauri dev
```

### Sharing a plugin

A build keeps the paths of your source for panic messages, which puts your home folder inside the file. Remove them
before you give the file to anyone:

```sh
RUSTFLAGS="--remap-path-prefix=$HOME=~ --remap-path-prefix=$(git rev-parse --show-toplevel)=src" \
  cargo build --release --target wasm32-unknown-unknown
```

## Other languages

Anything that produces a core module with the exports above works; there is no SDK for other languages yet. In short:
export a memory and `tz_alloc` (a bump allocator is enough: the app asks for one block per call), parse the event JSON,
build the reply JSON in memory, return `(ptr << 32) | len`, and put the manifest in a custom section named
`termzip-plugin` (in Rust that is a `link_section` attribute on a static, which is what the SDK's `manifest!` writes;
other toolchains have their own way). Check the result by installing the file: every problem is explained in a
sentence.
