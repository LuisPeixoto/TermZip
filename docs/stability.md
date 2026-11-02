# Stability

From 1.0, TermZip follows [semantic versioning](https://semver.org). This page lists what is promised to stay as it is
inside 1.x, and what is not. A change to a promise below needs 2.0, is announced in the changelog before it happens, and
comes with a migration when data is involved.

## Promised

| What | The promise |
| --- | --- |
| **`config.toml` keys** | Every key in [configuration](configuration.md) keeps its name, type and meaning. New keys may appear and have defaults; a file written by an older 1.x reads in a newer one, and unknown keys are ignored, so a file from a newer 1.x does not break an older one. |
| **Out-of-range values** | Still clamped, never an error. A file that cannot be read is kept as `config.toml.invalid`. |
| **`workspace.json`** | Structure only (tabs, names, splits, profiles, folders). An older file keeps loading; one that cannot be read is kept as `.invalid` and a fresh layout opens. |
| **The theme file format** | A TermZip theme file of 1.0 keeps loading. The import readers (iTerm2, Windows Terminal, Ghostty, Kitty, Alacritty) may learn more, not less. |
| **Plugin API 1** | The versioning policy in [plugin API](plugin-api.md#versioning) holds: nothing a plugin relies on changes inside `api = 1`, and a breaking change is `api = 2`, with the previous API understood for at least one minor release. Limits are raised, never lowered. |
| **Plugin permissions** | The names and meaning of the permissions, and that a plugin is off until a person approves the SHA-256 of its file. |
| **Where data lives** | The data folder (`~/Library/Application Support/termzip`, `$XDG_CONFIG_HOME/termzip`, `%APPDATA%\termzip`) and `TERMZIP_HOME` to move it. |
| **What never happens** | No telemetry, no accounts, no network use unless a person presses a button (catalog, plugin install, update check) or turns on the optional check at launch. Typing from a snippet, a task or a plugin never presses Enter. A click never starts a program. |
| **The shell contract** | The shell gets `TERM=xterm-256color`, `COLORTERM=truecolor` and `TERM_PROGRAM=TermZip`, and the integration speaks OSC 133 and `1337;CommandB64` / `CurrentDir`. |
| **Updates** | An update is installed only after a person asks, and only if its signature matches the key built into the app. |

## Not promised

- **The look of the interface**: layout, spacing, icons, wording of messages and the order of Preferences. Shortcuts keep
  their defaults inside 1.x, except where a default collides with a key a shell needs.
- **Internal interfaces**: the Tauri commands between the page and the core, the crates' Rust APIs (they are not
  published), the format of events between the core and the page.
- **Bench mode**: `TERMZIP_BENCH*` and the report it writes are for the project's own measurements.
- **Debug-only features**: `TERMZIP_UPDATE_ENDPOINT` and the catalog override exist only in debug builds.
- **The exact output of block capture** on a shell that has no integration, or on a program that rewrites the screen.
- **Performance numbers**: the [budgets](performance.md#budgets) are targets on the reference hardware and are checked
  with slack; they may move.
- **The catalog's contents and the package channels**: what is listed or published there can change.

## What a 1.x release may change

Fixes, new options (with defaults that keep today's behaviour), new plugin API additions that are optional, new themes,
new shells with integration, performance, accessibility and security improvements, and updates of dependencies.

If a security fix needs to break a promise above, it is made anyway and the changelog says so.
