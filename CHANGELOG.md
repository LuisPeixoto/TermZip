# Changelog

Notable changes to TermZip are documented here.

## [Unreleased]

## [0.2.0] - 2026-10-28

Everyday productivity: tabs, split panes, search, links, profiles, and a layout that comes back.

### Added

- Tabs and split panes: ⌘T, ⌘W, ⌘D, ⇧⌘D, ⌘1…9, ⇧⌘[ ⇧⌘], and ⌥⌘ + arrows on macOS (Ctrl+Shift or Ctrl+Alt elsewhere),
  draggable dividers, rename by double click, reorder by dragging. Every tab keeps its shell and its buffer.
- Session restore: tabs, names, splits, profiles, and the folder of each pane come back on the next launch. Only structure
  is saved, never terminal output; it can be turned off in Preferences, which also deletes what was saved.
- Profiles with a shell, a folder, and environment variables, edited in Preferences, with a default one and a menu next
  to the new-tab button.
- Find in the terminal with ⌘F: highlights, a count, next and previous, case and regular-expression toggles.
- A filter for the blocks rail by text (command and output, ignoring colors) and by status.
- Clickable URLs, OSC 8 hyperlinks, and file paths with `:line:column`, opened with ⌘+click (Ctrl+click elsewhere). A
  click opens only http, https, mailto, and files that exist, and never starts a program: scripts, installers, and
  applications only have their folder opened.
- Guides: tabs and panes, search and filter, links, profiles and restore, and performance and limits.

### Changed

- The blocks rail draws only the cards in view, and a finished command is sent as one event instead of the whole list.
- A shell that prints faster than the terminal can draw is paused at 1 MiB of undrawn output instead of queueing it
  without limit, so the screen stays close to a program such as `yes` or a huge `cat`, and Ctrl+C stops it.
- The terminal now takes the folder of the pane you were in for a new tab or split.

## [0.1.0] - 2026-10-27

First release: a usable terminal with command blocks.

### Added

- Desktop interface rebuilt on Tauri 2, Svelte 5, Tailwind 4, and xterm.js with a zinc & blue design system.
- The terminal is the only input: type straight into your shell, so its line editor, history, completion, and
  keybindings work unchanged.
- Application shortcuts that never steal shell keys: ⌘B / ⌘R / ⌘, on macOS and Ctrl+Shift+B / Ctrl+Shift+R / Ctrl+,
  on Windows and Linux, shown in the status bar and the tooltips.
- `TERMZIP_HOME` moves the data folder (settings and shell-integration scripts).
- A settings file that cannot be read is kept as `config.toml.invalid` instead of being overwritten, and saves are
  atomic.
- One ordered channel per shell session carries output, blocks, and the exit, so the first prompt is never drawn twice
  and the exit is always reported.
- A troubleshooting guide, a documentation table in the README, and design notes in `SECURITY.md`.
- Shell integration (OSC 133 + private 1337 markers) for zsh, bash, fish, and PowerShell with a bundled script per
  shell, injected without touching user configuration.
- Command blocks with command text, exit code, duration, working directory, ANSI-colored output preview, and copy /
  re-run / output-copy actions. Re-run types the command into the shell and returns the keyboard to the terminal.
- Full terminal canvas (xterm.js): keyboard encoding, bracketed paste, selection with clipboard shortcuts, alternate
  screen, scrollback, mouse reporting, and resize on all platforms via PTY/ConPTY.
- Preferences for theme, font size, font family, scrollback lines, shell path, and working directory, persisted with
  validation and clamping. Theme, font, and scrollback apply to the running terminal as soon as they are saved.
- New workspace crate `termzip-shell` (marker parser, block tracker, launch configuration) with unit tests.
- GitHub Actions: desktop job (svelte-check, vitest, vite build, shell tests, clippy, a debug application build), a
  release workflow that checks the tag against the version and builds both macOS targets, npm Dependabot updates.

### Changed

- Replaced the first egui prototype and the in-repository VT engine (`termzip-terminal`) with the Tauri + xterm.js
  architecture; terminal state lives in the webview, processes and blocks in Rust.
- `termzip-core` settings gained `font_family`, validation, and clamped ranges; existing configs keep working.

### Removed

- egui/eframe dependency and the hand-written terminal renderer.