# Roadmap

Items are prioritized within each phase. Features move to complete only after their acceptance checks pass. Open an
issue before beginning a larger architectural change.

## ✅ Phase 0: Research and feasibility

- ✅ Static REA inspection of the supplied Warp, Ghostty, and Terminal.app binaries and macOS bundle metadata.
- ✅ Review of ProxyZip's repository structure, roadmap, CI, contribution workflow, and Rust workspace conventions.
- ✅ Original visual direction chosen: minimal premium, zinc & blue palette. The first design had a bottom composer;
  it was removed before v0.1 and the terminal itself is the only input.
- ✅ Cross-platform PTY and VT prototype validated on macOS with CI coverage on all three platforms.
- [ ] Measure cold launch, idle CPU, memory, scrollback, and rendering with reproducible workloads.
- [ ] Validate keyboard input, accessibility, and shell behavior on real Windows and Linux systems.

Binary inspection does not establish runtime performance or reconstruct proprietary source code.

## ✅ Phase 1: Project foundation

- ✅ Rust workspace: shared core, PTY, shell integration, and the Tauri desktop shell.
- ✅ Interface rebuilt on Tauri 2 + Svelte 5 + Tailwind 4 + xterm.js, replacing the first egui prototype.
- ✅ Zinc & blue design tokens with complete dark and light themes.
- ✅ README, architecture, configuration, shell integration, security, and community files.
- ✅ GitHub Actions: Rust matrix plus desktop job (svelte-check, vitest, vite build, clippy) on three platforms.
- ✅ Release workflow building desktop bundles and checksums; Dependabot for cargo, npm, and Actions.
- ✅ First end-to-end CI run of the new workflows on all three platforms.

## ✅ Phase 2: Usable terminal MVP · v0.1

- ✅ Native shell in a PTY; raw output streamed to xterm.js off the read thread.
- ✅ Typing happens directly in the terminal: the shell's own line editor, history, and completion.
- ✅ Shell integration (OSC 133) for zsh, bash, fish, and partial PowerShell; cmd.exe runs plain.
- ✅ Command blocks with command, exit code, duration, cwd, ANSI output preview, copy, output copy, and re-run.
- ✅ Interactive programs keep working: alternate screen, bracketed paste, selection, clipboard, resize.
- ✅ Preferences: theme, font size/family, scrollback, shell path, working directory — persisted and clamped.
- ✅ Automated matrix on macOS, Windows and Linux; manual macOS validation recorded in
  [compatibility-matrix.md](compatibility-matrix.md).
- [ ] Manual Windows/Linux interactive validation (zsh/bash/fish/PowerShell, vim, less, top/htop, tmux, SSH);
  tracked as validation debt for Phase 7.
- ✅ Scrollback selection and mouse reporting are handled by xterm.js; clipboard shortcuts cover selections.
- ✅ Session lifecycle: exit status, restart action, and the shell-provided window title.
- ✅ Hardening before v0.1 · one ordered channel per session (no duplicated startup output, the exit is always
  reported) · platform-aware shortcuts that never steal shell keys · preferences applied to the live terminal ·
  per-shell quoting of the integration path · UTF-8 safe block text · `TERMZIP_HOME` · a broken config is kept as
  `config.toml.invalid` · atomic saves · a release workflow that checks the tag and builds both macOS targets.

## ✅ Phase 3: Everyday productivity · v0.2

- ✅ Multiple tabs and split panes: new, close, rename, reorder, drag the dividers, move between panes by keyboard;
  every tab stays mounted. See [tabs and panes](tabs-and-panes.md).
- ✅ Search the terminal (⌘F, case and regular expression, a count) and filter the blocks by text and status. See
  [search and filter](search-and-filter.md).
- ✅ Session restore of tabs, splits, and folders (structure only, never output) and profiles with a shell, folder, and
  environment, edited in Preferences. See [profiles and restore](profiles-and-restore.md).
- ✅ Clickable URLs, OSC 8 hyperlinks, and file paths with ⌘+click; the backend only opens what exists and never starts
  a program. See [links](links.md).
- ✅ Large-output budgets: one event per block instead of the whole list, flow control between the shell and the
  terminal, and a virtual list of blocks. See [performance](performance.md).
- Keyboard customization moved to Phase 4, where the shortcut editor lives.

## 📋 Phase 4: Personalization · v0.3

- [ ] Theme import/export and a gallery of audited themes.
- [ ] Cursor styles, opacity, spacing, per-profile shell settings.
- [ ] A complete keyboard shortcut editor (customization moved here from Phase 3) and a discoverable command palette.

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