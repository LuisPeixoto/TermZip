# Example plugins

Four small plugins, one for each thing a TermZip plugin can do. They are written with
[`termzip-plugin-sdk`](../../crates/termzip-plugin-sdk) and double as templates: copy a folder, change `plugin.toml`
and `src/lib.rs`, and build.

| Plugin | Needs | What it does |
| --- | --- | --- |
| [`long-command-notify`](long-command-notify) | `blocks`, `notify` | A "slow" label and a message when a command that ran for more than ten seconds ends. |
| [`exit-status`](exit-status) | `blocks` | A red label with the exit code on every failed command, and "N failed" in the status bar. |
| [`issue-links`](issue-links) | nothing | `#123` and `owner/repo#123` in the output become links to the issue on GitHub. |
| [`snippets`](snippets) | `type` | Palette commands that type a command you use often, without pressing Enter. |

## Build

```sh
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown
```

Each plugin is one file in `target/wasm32-unknown-unknown/release/` (about 75 KB), named after the package with `_`
for `-`: `exit_status.wasm`, `issue_links.wasm` and so on.

## Try one

Open **Preferences → Plugins → Install from file…**, pick a `.wasm` file and turn the plugin on. TermZip lists what
it may do and waits for you to confirm. A file you install yourself is marked **not verified**: nobody checked it
but you.

`issue-links` points `#123` at the TermZip repository. Change the second `[[links]]` entry in its `plugin.toml` to
your own repository and build it again.

## Tests

```sh
cargo test                                    # each plugin's own tests, on your machine
cargo test -p termzip-plugins --test examples -- --ignored   # from the repository root, after building
```

The second one loads the built `.wasm` files into the same reader, store and sandbox the app uses and checks what
they answer. CI runs both.

## Share a plugin

A build keeps the paths of your source code for panic messages, and that puts the name of your home folder inside the
file. Remove it before you give the file to anyone:

```sh
RUSTFLAGS="--remap-path-prefix=$HOME=~ --remap-path-prefix=$(git rev-parse --show-toplevel)=src" \
  cargo build --release --target wasm32-unknown-unknown
```

## Your own plugin

A plugin is a Rust library that builds as a `cdylib`, with a `plugin.toml` next to `Cargo.toml`:

```rust
use termzip_plugin_sdk::prelude::*;

termzip_plugin_sdk::manifest!("../plugin.toml");

#[derive(Default)]
struct Hello;

impl Plugin for Hello {
    fn on_block(&mut self, block: &Block) -> Vec<Action> {
        vec![status(format!("last exit: {:?}", block.exit_code))]
    }
}

termzip_plugin_sdk::export!(Hello);
```

The macros write `unsafe` into your crate, so it must not forbid `unsafe_code`. That is why these examples are their
own workspace and not part of the main one.
