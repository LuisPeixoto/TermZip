# Configuration

TermZip stores optional TOML settings in the operating system's standard per-user configuration directory:

- macOS: `~/Library/Application Support/termzip/config.toml`
- Linux: `$XDG_CONFIG_HOME/termzip/config.toml` or `~/.config/termzip/config.toml`
- Windows: `%APPDATA%\termzip\config.toml`

Missing files use built-in defaults. Every option can be changed in **Preferences** (⌘, on macOS, Ctrl+, elsewhere)
and is written when you press Save. The file contains preferences only — no shell history or terminal output.

```toml
theme = "dark"            # "dark" | "light"
font_size = 14.0          # clamped to 10..28
# font_family = "JetBrains Mono"
scrollback_lines = 10000  # clamped to 1_000..200_000
# shell = "/bin/zsh"      # default: $SHELL / %COMSPEC%
# working_directory = "/path/to/project"
restore_session = true    # reopen tabs, splits, and folders on launch
# default_profile = "work"  # which profile a plain new tab uses; empty: the built-in one
```

Out-of-range values are clamped and unknown keys are ignored. Font size, font family, scrollback, and theme apply to
the running terminal as soon as you save; `shell` and `working_directory` are read when a session starts, so restart
the shell (⌘R, or Ctrl+Shift+R on Windows and Linux) after changing them.

## Profiles

A profile is a shell, a folder, and some environment variables. Empty fields come from the built-in profile (the
`shell` and `working_directory` above). Profiles can be edited in **Preferences → Profiles** or written by hand:

```toml
default_profile = "work"

[[profiles]]
id = "work"                       # derived from the name when missing
name = "Work"
# shell = "/bin/bash"
working_directory = "/srv/work"

[profiles.env]
AWS_PROFILE = "work"
```

Bounds: 32 profiles, names up to 64 characters, 64 variables each, names of letters, digits and `_` (up to 128
characters), values up to 4 KiB. `TERM`, `COLORTERM`, `TERM_PROGRAM`, `TERM_PROGRAM_VERSION`, `ZDOTDIR`, and anything
starting with `TERMZIP_` cannot be set. Variables reach new shells only. See
[profiles and restore](profiles-and-restore.md).

## The saved layout

With `restore_session = true` TermZip writes `workspace.json` next to `config.toml`: tabs, their names, the splits, the
profile and folder of each pane. It never holds terminal output, commands, or environment. Turning `restore_session`
off (in Preferences or here) deletes it. A file that breaks a limit or cannot be parsed is kept as
`workspace.json.invalid`.

## A file that cannot be read

If the file is not valid TOML or a value has the wrong type, TermZip does not overwrite it. It renames the file to
`config.toml.invalid` next to the original, starts with the defaults, and your next save creates a fresh
`config.toml`. Fix the old file by hand and rename it back if you want your settings again.

Saves are atomic: the new contents go to a temporary file that is renamed into place, so a crash cannot leave a
half-written config. On macOS and Linux the folder is private to you (`0700`) and the file is `0600`.

## Moving the data folder

Set `TERMZIP_HOME` to use another folder for everything TermZip stores (the config file and the shell-integration
scripts). Tests, demos, and a second profile can use it without touching your real settings:

```bash
TERMZIP_HOME=/tmp/termzip-demo npm run tauri dev
```

An empty value is ignored. The folder is created on the first save and, on macOS and Linux, restricted to your user
(`0700`) — point it at a folder that only TermZip uses.

## Shell integration files

TermZip writes its integration scripts next to the config file under `shell-integration/`. For zsh it also stages a
`ZDOTDIR` shim that sources your own `~/.zshenv`/`~/.zshrc` first. These files are regenerated when their content
changes; you do not need to edit them.