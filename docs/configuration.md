# Configuration

TermZip stores optional TOML settings in the operating system's standard per-user configuration directory:

- macOS: `~/Library/Application Support/termzip/config.toml`
- Linux: `$XDG_CONFIG_HOME/termzip/config.toml` or `~/.config/termzip/config.toml`
- Windows: `%APPDATA%\termzip\config.toml`

Missing files use built-in defaults. Every option can be changed in **Preferences** (⌘, on macOS, Ctrl+, elsewhere)
and is written when you press Save. The file contains preferences only — no shell history or terminal output.

```toml
theme = "dark"            # the appearance: "dark" | "light"
dark_theme = "termzip-dark"      # the theme used while the appearance is dark (see themes.md)
light_theme = "termzip-light"    # ... and while it is light
font_size = 14.0          # clamped to 10..28
# font_family = "JetBrains Mono"
line_height = 1.3         # clamped to 1.0..2.0
letter_spacing = 0.0      # pixels, clamped to 0..4
cursor_style = "block"    # "block" | "bar" | "underline"
cursor_blink = true
background_opacity = 1.0  # clamped to 0.5..1.0; below 1.0 the desktop shows through
background_blur = true    # blur what shows through, on macOS and Windows
scrollback_lines = 10000  # clamped to 1_000..200_000
# shell = "/bin/zsh"      # default: $SHELL / %COMSPEC%
# working_directory = "/path/to/project"
restore_session = true    # reopen tabs, splits, and folders on launch
# default_profile = "work"  # which profile a plain new tab uses; empty: the built-in one
```

Out-of-range values are clamped (a value that is not a number gets its default) and unknown keys are ignored. The
fonts, the spacing, the cursor, the opacity, scrollback, and the theme apply to the running terminal as soon as you
save; `shell` and `working_directory` are read when a session starts, so restart
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

A profile may also set the look of its own terminals; whatever it leaves out is the app's setting (see
[appearance](appearance.md#per-profile-looks)):

```toml
[[profiles]]
name = "Server"
theme = "dracula"          # the id of a theme
font_size = 16             # 10..28
font_family = "Fira Code"
cursor_style = "bar"
```

Bounds: 32 profiles, names up to 64 characters, 64 variables each, names of letters, digits and `_` (up to 128
characters), values up to 4 KiB. `TERM`, `COLORTERM`, `TERM_PROGRAM`, `TERM_PROGRAM_VERSION`, `ZDOTDIR`, and anything
starting with `TERMZIP_` cannot be set. Variables reach new shells only. See
[profiles and restore](profiles-and-restore.md).

## Keyboard shortcuts

`[keybindings]` holds only the shortcuts you changed, as `action = "Chord"`; an empty chord removes a shortcut. It is
written by **Preferences → Keyboard**, and the grammar, the key names and the action names are in
[keyboard](keyboard.md#in-configtoml). A chord that is not well formed is ignored when the file is read.

```toml
[keybindings]
splitRight = "Shift+Cmd+E"
find = ""
```

## The themes folder

Imported and custom [themes](themes.md) are one TOML file each in `themes/`, next to `config.toml`
(`~/Library/Application Support/termzip/themes/` on macOS). The folder is private to your user (`0700`, files `0600`) on
macOS and Linux, and files that cannot be used are skipped and listed in Preferences. `dark_theme` and `light_theme`
name a theme by its `id`; an id that names nothing falls back to TermZip's own at run time but is kept in the file, so
a theme you deleted by mistake can come back.

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

Set `TERMZIP_HOME` to use another folder for everything TermZip stores (the config file, the themes, the saved layout
and the shell-integration scripts). Tests, demos, and a second profile can use it without touching your real settings:

```bash
TERMZIP_HOME=/tmp/termzip-demo npm run tauri dev
```

An empty value is ignored. The folder is created on the first save and, on macOS and Linux, restricted to your user
(`0700`) — point it at a folder that only TermZip uses.

## Shell integration files

TermZip writes its integration scripts next to the config file under `shell-integration/`. For zsh it also stages a
`ZDOTDIR` shim that sources your own `~/.zshenv`/`~/.zshrc` first. These files are regenerated when their content
changes; you do not need to edit them.