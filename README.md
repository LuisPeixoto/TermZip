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
run again.

## Highlights

- **Type where you read.** The terminal is the only input: your shell's line editor, history, completion, and
  keybindings work exactly as they do in any terminal.
- **Real command blocks.** With an integrated shell, every command becomes a card with its command text, exit code,
  duration, working directory, and an ANSI-colored output preview — copyable and re-runnable in one click.
- **A real terminal, always.** The canvas is a full terminal emulator: vim, htop, tmux, SSH, and REPLs work as usual,
  with selection, clipboard, bracketed paste, and resize.
- **Private by design.** No accounts, no telemetry, no cloud. Terminal contents never leave your machine.
- **Rust core, native shell.** Sessions run in a real PTY via a small Rust engine; the interface is a fast Tauri app.
- **Zinc & blue.** A calm, high-contrast visual system with complete dark and light themes.

## Screenshots

<p align="center">
  <img src="docs/images/termzip-tabs-light.png" width="880" alt="TermZip in the light theme with three tabs, two panes side by side, and the command blocks rail for the focused pane" />
  <br />
  <sub>Tabs and split panes. Each pane has its own shell, and the blocks rail follows the focused one.</sub>
</p>

<p align="center">
  <img src="docs/images/termzip-tabs-dark.png" width="880" alt="TermZip in the dark theme with the find box open, highlighting two matches of PASS in the left pane" />
  <br />
  <sub>The dark theme with the find box open: matches are highlighted and counted, and Enter steps through them.</sub>
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
| Toggle the blocks rail | ⌘B | Ctrl+Shift+B |
| Restart the shell | ⌘R | Ctrl+Shift+R |
| Preferences | ⌘, | Ctrl+, |
| Copy / paste / select all in the terminal | ⌘C / ⌘V / ⌘A | Ctrl+Shift+C / V / A |

On Windows and Linux the application shortcuts use Ctrl+Shift, so plain Ctrl+R (reverse search), Ctrl+B and the other
Ctrl keys always reach your shell. Ctrl+C still interrupts the running command.

## Documentation

| Guide | What it covers |
|---|---|
| [Configuration](docs/configuration.md) | Where settings live, every option, `TERMZIP_HOME`, what happens to a broken file. |
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
| App icon source | `assets/` |

## Contributing

Issues and focused pull requests are welcome. Read [CONTRIBUTING.md](CONTRIBUTING.md) for branch naming, Conventional
Commits, and the checks every change must pass.

## Security

TermZip runs a shell with your user's permissions. Report vulnerabilities privately using GitHub's
[security advisory form](https://github.com/LuisPeixoto/TermZip/security/advisories/new). See
[SECURITY.md](SECURITY.md) for the current threat model.