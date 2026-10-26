# Roadmap

Items are prioritized within each phase. TermZip is early-stage; features move to complete only after their acceptance
checks pass. Open an issue before beginning a larger architectural change.

## ✅ Phase 0: Research and feasibility

- ✅ Static REA inspection of the supplied Warp, Ghostty, and Terminal.app binaries and macOS bundle metadata.
- ✅ Review of ProxyZip's repository structure, roadmap, CI, contribution workflow, and Rust workspace conventions.
- ✅ Select an original visual direction: minimal premium, with a bottom-anchored composer and a separate conventional
  terminal mode.
- ✅ Build the first cross-platform PTY and VT-emulator prototype.
- [ ] Measure cold launch, idle CPU, memory, scrollback, and rendering with reproducible workloads.
- [ ] Validate keyboard input, accessibility, and shell behavior on real macOS, Windows, and Linux systems.

The static observations and their limits are summarized in the project plan and architecture. Binary inspection does
not establish runtime performance or reconstruct proprietary source code.

## 🚧 Phase 1: Project foundation

- ✅ Rust workspace split into shared core, terminal emulation, PTY, and desktop packages.
- ✅ Dark and light theme tokens, preferences file, and first-run defaults.
- ✅ README, architecture, configuration, security, contributing, and community files.
- ✅ GitHub Actions for formatting, Clippy, tests, and builds on macOS, Windows, and Linux.
- [ ] Observe the first full three-platform CI run and fix any platform-specific failures.
- [ ] Add release/build metadata only after package targets are verified.

## 🚧 Phase 2: Usable terminal MVP · v0.1

- ✅ Native shell starts in a PTY; output is parsed off the shell's read thread and rendered in the desktop window.
- ✅ Bottom-anchored command composer and basic direct-input mode for interactive programs.
- ✅ ANSI colors, cursor movement, alternate screen, scrollback, and terminal resize are handled by the VT engine.
- ✅ Persistent theme/font preferences and bounded shell-output transport.
- [ ] Add shell integration markers and first-class command/output blocks.
- ✅ Visible-screen selection, copy shortcuts, paste, and bracketed-paste wrapping.
- [ ] Add scrollback selection, mouse-reporting clicks, and fuller keyboard/mouse protocols.
- [ ] Test Zsh, Bash, Fish, PowerShell, cmd.exe, SSH, vim, less, top/htop, and tmux by operating system.
- [ ] Fix the status/cursor behavior and publish compatibility results for each supported platform.

## 📋 Phase 3: Everyday productivity · v0.2

- [ ] Multiple tabs and split panes.
- [ ] Search terminal history and output; command history navigation and working-directory actions.
- [ ] Session restore, profiles, links, shell integration, and keyboard customization.
- [ ] Large-output virtualization and incremental/damage-based redraw.

## 📋 Phase 4: Personalization · v0.3

- [ ] Theme import/export and a gallery of audited themes.
- [ ] Font families, cursor styles, opacity, spacing, and per-profile shell settings.
- [ ] A complete keyboard shortcut editor and discoverable command palette.

## 📋 Phase 5: Extensions · v0.4

- [ ] Stable plugin API and versioning policy.
- [ ] WebAssembly plugin runtime with resource limits and explicit permissions.
- [ ] Plugin discovery, installation, disable/uninstall, and signed catalog metadata.

## 📋 Phase 6: Developer workflows · v0.5

- [ ] Reusable command workflows, snippets, project profiles, and task runners.
- [ ] SSH and WSL profiles without storing private keys or shell secrets in TermZip.
- [ ] Optional developer integrations, with network access disabled unless a feature explicitly needs it.

## 📋 Phase 7: Stabilization · v1.0

- [ ] Documented performance budgets measured on reference hardware.
- [ ] Broad cross-platform terminal conformance and accessibility testing.
- [ ] Signed/notarized installers, package-manager distribution, update policy, and release checksums.
- [ ] Plugin threat model, permissions, and security audit.
