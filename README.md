<p align="center">
  <img src="assets/icon.png" width="96" alt="TermZip mark" />
</p>

<h1 align="center">TermZip</h1>

<p align="center">
  <b>A fast, beautiful terminal for people who build things.</b><br />
  An open-source desktop terminal written in Rust that turns every command into a block you can copy and re-run.
</p>

<p align="center">
  <a href="https://github.com/LuisPeixoto/TermZip/actions/workflows/ci.yml"><img src="https://github.com/LuisPeixoto/TermZip/actions/workflows/ci.yml/badge.svg" alt="CI status" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-2563eb.svg" alt="MIT license" /></a>
  <a href="https://github.com/LuisPeixoto/TermZip/releases"><img src="https://img.shields.io/github/v/release/LuisPeixoto/TermZip?include_prereleases&color=60a5fa" alt="Latest release" /></a>
  <img src="https://img.shields.io/badge/platforms-macOS%20%7C%20Windows%20%7C%20Linux-52525b.svg" alt="Supported platforms" />
  <img src="https://img.shields.io/badge/built%20with-Rust%20%2B%20Tauri-orange.svg" alt="Built with Rust and Tauri" />
  <a href="CONTRIBUTING.md"><img src="https://img.shields.io/badge/PRs-welcome-brightgreen.svg" alt="PRs welcome" /></a>
</p>

<p align="center">
  <a href="#getting-started">Getting started</a> ·
  <a href="docs/roadmap.md">Roadmap</a> ·
  <a href="docs/architecture.md">Architecture</a> ·
  <a href="docs/tabs-and-panes.md">Tabs and panes</a> ·
  <a href="docs/themes.md">Themes</a> ·
  <a href="docs/keyboard.md">Keyboard</a> ·
  <a href="docs/plugins.md">Plugins</a> ·
  <a href="docs/snippets.md">Snippets</a> ·
  <a href="docs/ssh-and-wsl.md">SSH and WSL</a> ·
  <a href="docs/configuration.md">Configuration</a> ·
  <a href="docs/shell-integration.md">Shell integration</a> ·
  <a href="docs/troubleshooting.md">Troubleshooting</a> ·
  <a href="docs/terminal-compatibility.md">Compatibility</a> ·
  <a href="docs/compatibility-matrix.md">Test matrix</a> ·
  <a href="docs/distribution.md">Distribution</a> ·
  <a href="CONTRIBUTING.md">Contributing</a>
</p>

TermZip is a native desktop terminal for macOS, Windows, and Linux. You type straight into a real terminal, and a
side rail turns every finished command into a **block** with its exit code, duration, and output — ready to copy or
run again. Tabs and split panes keep several shells in one window, and your layout comes back on the next launch.

## Highlights

- **Type where you read.** The terminal is the only input: your shell's line editor, history, completion, and
  keybindings work exactly as they do in any terminal.
- **Real command blocks.** With an integrated shell, every command becomes a card with its command text, exit code,
  duration, working directory, and an ANSI-colored output preview — copyable and re-runnable in one click.
- **Tabs and split panes.** Several shells in one window, split right or down, resizable, each with its own blocks.
  Your tabs, splits, and folders are restored on the next launch (structure only, never terminal output).
- **Find and filter.** ⌘F searches the scrollback with highlights and a count; the blocks rail filters by text and by
  status.
- **Links that are safe to click.** ⌘+click opens URLs, hyperlinks, and files; a click can never start a program.
- **Profiles.** A shell, folder, and environment per profile, one click away from the new-tab button.
- **A real terminal, always.** The canvas is a full terminal emulator: vim, htop, tmux, SSH, and REPLs work as usual,
  with selection, clipboard, bracketed paste, and resize.
- **Private by design.** No accounts, no telemetry, no cloud. Terminal contents never leave your machine.
- **Rust core, native shell.** Sessions run in a real PTY via a small Rust engine; the interface is a fast Tauri app.
- **Themes you can bring.** Twelve audited themes (Catppuccin, Dracula, Nord, Gruvbox, Tokyo Night, One Dark, Rosé
  Pine and TermZip's own) recolor the whole window, and iTerm2, Windows Terminal, Ghostty, Kitty and Alacritty themes
  import as they are.
- **Yours to shape.** Cursor, line height, letter spacing, a translucent window with native blur, and a look of its
  own for each profile.
- **Every shortcut is yours.** Rebind any of them in Preferences, with a warning before you take a key your shell
  uses, and find anything with the command palette (⇧⌘P).
- **Plugins in a sandbox.** Small WebAssembly files add palette commands, block labels, status items and links. A plugin
  cannot read your files, use the network or start a program; it asks for each thing it may see or do, and you approve
  it first. A signed catalog is one click away, and you can write your own in Rust.
- **Snippets that never run on their own.** Keep the commands you type again, with `{{values}}` to fill in
  (`git checkout {{branch:main}}`). ⇧⌘S lists them, a small form asks for the values, and the text is typed at the prompt
  for you to read and run: TermZip never presses Enter.
- **Your project's tasks, one keystroke away.** The scripts of `package.json`, Cargo's commands, Makefile targets,
  justfile recipes and `deno.json` tasks are in the palette for the folder you are in, typed (not run) when chosen. A
  profile can be tied to a folder, so a new terminal there uses it.
- **SSH and WSL profiles.** A tab that opens already connected, with `ssh` and your own agent and `~/.ssh/config`:
  TermZip stores no key, password or identity file. On Windows, a profile can open a WSL distribution.
- **Git where you look, never the network.** The branch, how far it is from its upstream and whether files changed, in
  the status bar, and *Open the repository in the browser* in the palette. It runs `git` on your computer and can be
  turned off.
- **Zinc & blue by default.** A calm, high-contrast visual system with complete dark and light themes.

## Screenshots

<p align="center">
  <img src="docs/images/termzip-dracula.png" width="880" alt="TermZip in the Dracula theme with two tabs, two panes side by side, the command blocks rail with a slow label and a failure label from plugins, a plugin message, and a count of failures in the status bar" />
  <br />
  <sub>Dracula, one of the twelve built-in themes, with three plugins running: a red <b>exit 2</b> and an amber <b>slow</b> label on the blocks, a message when <code>sleep 11</code> ends, and <b>2 failed</b> in the status bar.</sub>
</p>

<p align="center">
  <img src="docs/images/termzip-themes.png" width="880" alt="The Appearance preferences listing the themes as cards, each previewed in its own colors, with Dracula marked as in use" />
  <br />
  <sub>Preferences → Appearance. Every theme is previewed in its own colors, and the window follows your choice before you save.</sub>
</p>

<p align="center">
  <img src="docs/images/termzip-palette-light.png" width="880" alt="TermZip in the Catppuccin Latte theme with the command palette open, filtered to the commands of the Snippets plugin, each marked as a plugin" />
  <br />
  <sub>Catppuccin Latte with the command palette (⇧⌘P): every action, tab, profile, and theme, and the commands of your plugins, marked as such.</sub>
</p>

<p align="center">
  <img src="docs/images/termzip-snippets.png" width="880" alt="The command palette showing a small form for the snippet Check out a branch, with the value branch filled in as release/0.5 and a note that the text is typed at the prompt and nothing runs until Enter is pressed" />
  <br />
  <sub>A snippet with a value to fill in (⇧⌘S). The text is typed at the prompt and waits for you to read it and press Enter; TermZip never presses it for you. The status bar shows the git branch, two commits ahead, with changed files.</sub>
</p>

<p align="center">
  <img src="docs/images/termzip-tasks.png" width="880" alt="The command palette filtered on build, listing the tasks of a Makefile, a justfile and a package.json that uses pnpm, each marked with its file, next to a profile for an SSH connection" />
  <br />
  <sub>Searching for <i>build</i> in a project folder: the tasks of its Makefile, justfile and <code>package.json</code> (with the package manager it uses), typed and never run.</sub>
</p>

<p align="center">
  <img src="docs/images/termzip-ssh-profile.png" width="880" alt="The Profiles preferences with an SSH profile open for editing: the host, the user and the port, and a note that TermZip stores no key, password or identity file" />
  <br />
  <sub>Preferences → Profiles. An SSH profile connects with your own agent and <code>~/.ssh/config</code>: TermZip stores no key, password or identity file. A profile can also be tied to folders, like <i>API</i>.</sub>
</p>

<p align="center">
  <img src="docs/images/termzip-plugin-catalog.png" width="880" alt="The Catalog tab of the Plugins preferences listing four signed plugins with their authors, sizes, and the permissions each asks for" />
  <br />
  <sub>Preferences → Plugins → Catalog. The list is checked against a signature before it is trusted, and every plugin shows what it will ask for.</sub>
</p>

## Getting started

Install [Rust](https://rustup.rs) and [Node.js](https://nodejs.org) 20+, then:

```bash
git clone https://github.com/LuisPeixoto/TermZip.git
cd TermZip/apps/desktop
npm install
npm run tauri dev
```

Build a release bundle:

```bash
npm run tauri build
```

On Linux you also need the WebKitGTK development packages (`libwebkit2gtk-4.1-dev`, `libappindicator3-dev`,
`librsvg2-dev`).

## How command blocks work

TermZip injects a small integration script into zsh, bash, fish, and PowerShell that emits the
[semantic-prompt protocol](https://gitlab.freedesktop.org/per_1/terminal-wg/issues/5) (OSC 133). TermZip reads those
markers to build blocks — your prompt, aliases, and plugins are untouched. Shells without integration (cmd.exe, tcsh)
still work as a plain terminal; only block capture is unavailable.

See [docs/shell-integration.md](docs/shell-integration.md) and
[docs/terminal-compatibility.md](docs/terminal-compatibility.md) for details and current limitations.

## Keyboard shortcuts

| Action | macOS | Windows and Linux |
|---|---|---|
| New tab | ⌘T | Ctrl+Shift+T |
| Close the active pane (the last one closes the tab) | ⌘W | Ctrl+Shift+W |
| Split right / split down | ⌘D / ⇧⌘D | Ctrl+Shift+D / Ctrl+Shift+E |
| Move between panes | ⌥⌘ + arrow | Ctrl+Alt + arrow |
| Next / previous tab | ⇧⌘] / ⇧⌘[ | Ctrl+Tab / Ctrl+Shift+Tab |
| Go to tab 1 to 9 | ⌘1 … ⌘9 | Ctrl+Shift+1 … 9 |
| Command palette | ⇧⌘P | Ctrl+Shift+P |
| Insert a snippet | ⇧⌘S | Ctrl+Shift+S |
| Find in the terminal | ⌘F | Ctrl+Shift+F |
| Toggle the blocks rail | ⌘B | Ctrl+Shift+B |
| Restart the shell | ⌘R | Ctrl+Shift+R |
| Preferences | ⌘, | Ctrl+, |
| Copy / paste / select all in the terminal | ⌘C / ⌘V / ⌘A | Ctrl+Shift+C / V / A |

On Windows and Linux the application shortcuts use Ctrl+Shift, so plain Ctrl+R (reverse search), Ctrl+B and the other
Ctrl keys always reach your shell. Ctrl+C still interrupts the running command. Every shortcut can be changed in
**Preferences → Keyboard** (see [keyboard](docs/keyboard.md)).

## Documentation

| Guide | What it covers |
|---|---|
| [Configuration](docs/configuration.md) | Where settings live, every option, `TERMZIP_HOME`, what happens to a broken file. |
| [Tabs and panes](docs/tabs-and-panes.md) | Tabs, splits, focus, the layout tree, and the limits. |
| [Search and filter](docs/search-and-filter.md) | Find in the terminal and filter the blocks. |
| [Links](docs/links.md) | What is recognized, what opens, and why a click never starts a program. |
| [Themes](docs/themes.md) | The gallery and its credits, importing and exporting, the theme file, and how the interface colors are derived. |
| [Appearance](docs/appearance.md) | Cursor, spacing, window opacity and blur on each system, and per-profile looks. |
| [Keyboard and command palette](docs/keyboard.md) | Every default shortcut, the editor, `[keybindings]`, and what the palette lists. |
| [Plugins](docs/plugins.md) | What a plugin can do, installing from a file or the signed catalog, permissions, and the limits. |
| [Plugin API](docs/plugin-api.md) | For authors: the manifest, the ABI, actions, limits, the versioning policy, and the Rust SDK. |
| [Profiles and restore](docs/profiles-and-restore.md) | Shell, folder, and environment per profile; what is saved between launches. |
| [Snippets](docs/snippets.md) | Commands with `{{values}}`, the palette form, folders, and why a snippet is typed and never run. |
| [Project tasks and folder profiles](docs/project-tasks.md) | The tasks of `package.json`, Cargo, Makefile, justfile and `deno.json`, and profiles tied to folders. |
| [SSH and WSL](docs/ssh-and-wsl.md) | Profiles that connect, what is stored (nothing secret), what is checked, and the limits. |
| [Git](docs/git.md) | The status bar item, opening the repository, turning it off, and how a repository you cloned cannot run anything. |
| [Performance and limits](docs/performance.md) | Flow control for floods of output, block events, and every bound. |
| [Shell integration](docs/shell-integration.md) | The OSC 133 markers, supported shells, and the limits of block capture. |
| [Terminal compatibility](docs/terminal-compatibility.md) | What is implemented, verified, and still missing. |
| [Compatibility matrix](docs/compatibility-matrix.md) | What has been exercised on each operating system. |
| [Architecture](docs/architecture.md) | Crates, the session stream, and the design rules. |
| [Troubleshooting](docs/troubleshooting.md) | Symptom, cause, and fix for the problems seen so far. |
| [Distribution](docs/distribution.md) | Building bundles and the release workflow. |
| [Roadmap](docs/roadmap.md) | Finished phases and what comes next. |

## Project structure

| Area | Location |
|---|---|
| Desktop app (Tauri + Svelte + xterm.js) | `apps/desktop` |
| Rust shell (sessions, blocks, settings) | `apps/desktop/src-tauri` |
| Shell integration and OSC 133 parser | `crates/termzip-shell` |
| Cross-platform PTY (ConPTY on Windows) | `crates/termzip-pty` |
| Settings and shared tokens | `crates/termzip-core` |
| Plugin API types, the host (sandbox, store, catalog) and the SDK | `crates/termzip-plugin-api`, `crates/termzip-plugins`, `crates/termzip-plugin-sdk` |
| Project tasks and the git state of a folder | `crates/termzip-tasks`, `crates/termzip-git` |
| Tests that need something real (a throwaway `sshd`) | `tools` |
| Example plugins | `examples/plugins` |
| App icon source | `assets/` |

## Contributing

Issues and focused pull requests are welcome. Read [CONTRIBUTING.md](CONTRIBUTING.md) for branch naming, Conventional
Commits, and the checks every change must pass.

## Security

TermZip runs a shell with your user's permissions. Report vulnerabilities privately using GitHub's
[security advisory form](https://github.com/LuisPeixoto/TermZip/security/advisories/new). See
[SECURITY.md](SECURITY.md) for the current threat model.