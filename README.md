<p align="center">
  <img src="docs/images/termzip-mark.svg" width="96" alt="TermZip mark" />
</p>

<h1 align="center">TermZip</h1>

<p align="center">
  <b>A fast, thoughtful terminal for the people who build things.</b><br />
  An open-source desktop terminal, written in Rust, with its command line anchored at the bottom.
</p>

<p align="center">
  <a href="https://github.com/LuisPeixoto/TermZip/actions/workflows/ci.yml"><img src="https://github.com/LuisPeixoto/TermZip/actions/workflows/ci.yml/badge.svg" alt="CI status" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-8b7bff.svg" alt="MIT license" /></a>
  <a href="https://github.com/LuisPeixoto/TermZip/releases"><img src="https://img.shields.io/github/v/release/LuisPeixoto/TermZip?include_prereleases&color=61d6c6" alt="Latest release" /></a>
  <img src="https://img.shields.io/badge/platforms-macOS%20%7C%20Windows%20%7C%20Linux-667085.svg" alt="Supported platforms" />
  <img src="https://img.shields.io/badge/built%20with-Rust-orange.svg" alt="Built with Rust" />
  <a href="CONTRIBUTING.md"><img src="https://img.shields.io/badge/PRs-welcome-brightgreen.svg" alt="PRs welcome" /></a>
</p>

<p align="center">
  <a href="#getting-started">Getting started</a> ·
  <a href="docs/roadmap.md">Roadmap</a> ·
  <a href="docs/architecture.md">Architecture</a> ·
  <a href="docs/terminal-compatibility.md">Terminal compatibility</a> ·
  <a href="CONTRIBUTING.md">Contributing</a>
</p>

TermZip is an early-stage, native desktop terminal for macOS, Windows, and Linux. Its command composer stays at the
bottom of the window, keeping the next command close while output scrolls above it. Switch to terminal mode when an
interactive full-screen program needs direct keyboard input.

## What works today

- **Real shells.** TermZip opens the operating system's default shell in a native PTY.
- **Bottom-anchored composer.** Enter a command in the fixed footer; the shell keeps its normal process and directory
  state between commands.
- **Conventional terminal mode.** Pass text, common keys, and control characters through to interactive programs.
- **ANSI terminal grid.** A mature Rust terminal engine handles colors, cursor movement, screen buffers, and scrollback.
- **Terminal queries.** Standard device-status and device-attribute requests are answered back through the PTY.
- **Copy and paste.** Select visible terminal text with the mouse; copy it with `⌘C` or `Ctrl+Shift+C`, and paste
  using the terminal's bracketed-paste mode when enabled.
- **Dark and light themes.** A restrained, original visual system with a high-contrast terminal canvas.
- **Cross-platform foundation.** macOS, Windows, and Linux builds are checked in CI.
- **Private by default.** No accounts, network service, telemetry, or cloud sync.

TermZip is not yet a feature-complete terminal. Tabs, split panes, shell integration, structured command blocks,
plugin loading, installers, and measured performance budgets are future roadmap work. See the
[phase-by-phase roadmap](docs/roadmap.md) and [current limitations](docs/terminal-compatibility.md).

## Getting started

Install [Rust stable](https://rustup.rs), clone the project, and run the desktop application:

```bash
git clone https://github.com/LuisPeixoto/TermZip.git
cd TermZip
cargo run -p termzip-desktop
```

On Linux, install the native window-system and graphics development libraries required by
[eframe/winit](https://github.com/emilk/egui/tree/main/crates/eframe#prerequisites), including X11/Wayland headers and
Vulkan or OpenGL drivers. The GitHub Actions workflow documents the packages used by CI.

Run the workspace checks locally:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

Settings are stored in the platform's standard TermZip configuration directory as `config.toml`. The initial config
is optional; missing settings use safe defaults. See [Configuration](docs/configuration.md).

## Project structure

| Area | Location |
|---|---|
| Theme, settings, and shared data | `crates/termzip-core` |
| VT parsing, screen grid, and color resolution | `crates/termzip-terminal` |
| Cross-platform PTY and child process | `crates/termzip-pty` |
| Native desktop application | `apps/desktop` |
| Architecture and technical decisions | `docs/architecture.md` |
| Phased delivery plan | `docs/roadmap.md` |

## Contributing

Issues and focused pull requests are welcome. For substantial features, open an issue first so the approach and phase
are clear. Read [CONTRIBUTING.md](CONTRIBUTING.md) for branch names, Conventional Commit format, and checks.

## Security

TermZip starts a local shell with the permissions of the current user. Review commands before running them and do not
load untrusted plugins. Report vulnerabilities privately using GitHub's
[security advisory form](https://github.com/LuisPeixoto/TermZip/security/advisories/new).
