# Changelog

Notable changes to TermZip are documented here.

## [Unreleased]

## [0.5.0] - 2026-11-02

Developer workflows: snippets, the tasks of your project, SSH and WSL profiles, and git in the status bar.

### Added

- Snippets: commands you keep, with `{{value}}` or `{{value:default}}` to fill in (`git checkout {{branch:main}}`),
  optionally limited to a folder. **⇧⌘S** (Ctrl+Shift+S) opens the palette showing only snippets, a small form asks for
  the values, and the text is **typed at the prompt, never run**: every control character is removed in Rust, so a value
  cannot press Enter. Edit them in **Preferences → Snippets** or in `[[snippets]]`.
- Project tasks in the command palette: the scripts of `package.json` (with npm, pnpm, yarn or bun), Cargo's commands and
  aliases, Makefile targets, justfile recipes and `deno.json` tasks, for the folder of the terminal. They follow how
  each tool finds its file (the nearest one up to the repository root; `make` only the current folder), only plain
  names are offered, and choosing one types it.
- Profiles can be tied to folders (`folders = ["~/work/api"]`): a new terminal that starts there uses the profile, the
  most specific one winning, unless you chose another.
- SSH profiles (`kind = "ssh"`, `host`, `user`, `port`) and WSL profiles (`kind = "wsl"`, `distribution`, Windows only).
  TermZip runs `ssh … -- host` and `wsl.exe` and **stores no key, password or identity file**; the host field suggests
  the `Host` names of `~/.ssh/config` and the distribution field the installed ones. The host, user and distribution
  are checked twice and can never be read as an option.
- The git branch and state in the status bar (`↑↓` against the upstream, a dot for changed files, a red `!` in a merge
  conflict) and **Open the repository in the browser** in the palette, with credentials in the address dropped. It runs
  `git` on your computer and never uses the network, `git_status = false` turns it off (and git is then never started),
  and a repository you just cloned cannot make it run a program.
- `tools/live-tests.sh`, which signs in to a throwaway `sshd` through an SSH profile (run by hand, never by CI).
- Guides: snippets, project tasks and folder profiles, SSH and WSL, and git.

### Changed

- The command palette also lists snippets, project tasks and the repository, and has a snippets-only mode.
- Preferences has a **Snippets** section, a *Starts* selector and a *folders* field in the profile form, and a git
  switch in **General**.
- A remote profile starts where it says, and never inherits the working folder of this computer.

### Fixed

- A profile tied to a folder is also used by the first terminal of a launch, which starts in the default working folder.
- The help text of snippets said *Press* where it meant *Write*, and the profile help did not mention folders or SSH.

## [0.4.0] - 2026-10-31

Extensions: plugins that run in a sandbox, a stable API to write them against, and a signed catalog to find them.

### Added

- Plugins: small WebAssembly files that add palette commands, label a block or show a message when a command finishes,
  put an item in the status bar, and turn text in the output into links. A plugin has no imports, so it cannot read
  your files, use the network, start programs or read the clock; it runs in `wasmi` with a fuel budget per call, a
  16 MiB memory cap and a bounded queue, and three failures in a row stop it.
- Permissions the plugin asks for and you approve before turning it on: `blocks` (finished commands), `output` (what
  they printed), `type`, `notify` and `open`. The approval belongs to the SHA-256 of the file, so a plugin that is
  replaced, edited or updated stays off until you approve the new file.
- Typing into the terminal only as the answer to a palette command you ran, as a paste without Enter and with line breaks
  and control characters removed; opening an address only the same way, for `http` and `https`.
- Preferences → Plugins: Installed (state, permissions, where it came from, a switch with a confirmation, uninstall) and
  Catalog (load, install, update). **Install from file…** marks a file as not verified.
- The plugin catalog: its index is verified with a minisign key built into TermZip before it is read, every download is
  checked for size, SHA-256, id, version and permissions, and everything uses HTTPS. The network is used only when you
  press **Load catalog** or **Install**; `plugins_catalog = false` (a checkbox in Preferences) turns it off.
- Plugin API 1 with a versioning policy: a manifest in a custom section of the file, JSON events and actions, and a Rust
  SDK (`termzip-plugin-sdk`). Four example plugins in `examples/plugins` (long-command notifier, exit status, issue
  links and snippets), built and run in the app's own sandbox by CI.
- A plugin threat model in `SECURITY.md`, and guides: plugins and the plugin API.

### Changed

- `TERMZIP_HOME` also moves the `plugins/` folder.
- Block events are also reported to the plugin host, which does nothing when no plugin is on.

## [0.3.0] - 2026-10-28

Personalization: themes you can bring, a terminal and a window you can shape, and every shortcut yours.

### Added

- Themes recolor the whole window, not only the terminal: twelve built-in themes (TermZip Dark and Light, Catppuccin
  Mocha and Latte, Dracula, Nord, Gruvbox Dark and Light, Tokyo Night, One Dark, Rosé Pine and Rosé Pine Dawn) with their
  authors and licenses, a gallery with a preview of each, and one theme remembered per appearance. Choosing one shows
  it at once; Cancel puts the saved one back.
- Import themes from TermZip, iTerm2 (`.itermcolors`), Windows Terminal (JSON), Ghostty, Kitty, and Alacritty files,
  export any theme as a small TOML file, and keep your own in the `themes/` folder. A file that cannot be used is
  explained in a sentence and never stops TermZip.
- Cursor style (block, bar, underline) and blink, line height, and letter spacing, with a live sample.
- A translucent window: an opacity slider from 50 to 100 % with the system's blur behind it on macOS and Windows.
- A profile can have its own terminal theme, font size, font family, and cursor.
- A shortcut editor in Preferences → Keyboard: press Record and the keys, see which action lost the chord and which
  keys a shell uses, reset one or all. Shortcuts are saved as `[keybindings]` in `config.toml`.
- A command palette (⇧⌘P, Ctrl+Shift+P): fuzzy search over every action with its shortcut, each tab, a new tab per
  profile, every theme, and the appearance and blur switches.
- Guides: themes, appearance, and keyboard and the command palette.

### Changed

- Preferences are organized in sections (General, Appearance, Terminal, Keyboard, Profiles) in a wider dialog.
- The window is created transparent. It paints itself opaque unless you lower the opacity, so nothing changes for
  existing settings.
- On macOS the translucent window uses Tauri's private API, which rules out the Mac App Store.
- Every tooltip and hint shows the shortcut you set, and an action without a shortcut shows none.
- A hand-edited `config.toml` from 0.2 loads exactly as before: every new key has a default.

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