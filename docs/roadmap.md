# Roadmap

Items are prioritized within each phase. Features move to complete only after their acceptance checks pass. Open an
issue before beginning a larger architectural change.

## ✅ Phase 0: Research and feasibility

- ✅ Research of terminal workflows, shell-integration protocols, and platform conventions on macOS, Windows, and Linux.
- ✅ Rust workspace, CI, contribution workflow, and release conventions chosen.
- ✅ Original visual direction chosen: minimal premium, zinc & blue palette. The first design had a bottom composer;
  it was removed before v0.1 and the terminal itself is the only input.
- ✅ Cross-platform PTY and VT prototype validated on macOS with CI coverage on all three platforms.
- [ ] Measure cold launch, idle CPU, memory, scrollback, and rendering with reproducible workloads.
- [ ] Validate keyboard input, accessibility, and shell behavior on real Windows and Linux systems.

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

## ✅ Phase 4: Personalization · v0.3

- ✅ Themes recolor the whole window: twelve audited built-in themes with their authors and licenses, and import from
  TermZip, iTerm2, Windows Terminal, Ghostty, Kitty, and Alacritty files plus export to TermZip's format. See
  [themes](themes.md).
- ✅ Cursor style and blink, line height, letter spacing, a translucent window with native blur (vibrancy on macOS,
  acrylic on Windows), and a look of its own per profile. See [appearance](appearance.md).
- ✅ A complete shortcut editor (record, conflicts, warnings for keys a shell uses, reset) and a command palette over
  actions, tabs, profiles, and themes. See [keyboard](keyboard.md).
- [ ] Manual Windows/Linux validation of the translucent window and the new dialogs; tracked as validation debt for
  Phase 7.

## ✅ Phase 5: Extensions · v0.4

- ✅ A stable plugin API (version 1) and its versioning policy: a manifest, JSON events and actions, golden-tested, and
  a Rust SDK with four example plugins. See [plugin API](plugin-api.md).
- ✅ A WebAssembly runtime (`wasmi`) with a fuel budget, a memory cap, bounded queues and explicit permissions. A plugin
  has no imports, so it cannot reach files, the network, processes or the clock. See [plugins](plugins.md).
- ✅ Four extension points: palette commands, reactions to finished commands (label, message, status), link patterns,
  and status-bar items. Typing into the terminal happens only for a palette command, never presses Enter, and cannot
  be done from a reaction.
- ✅ Discovery and installation: a catalog whose index is verified with a built-in minisign key before it is read, the
  size and SHA-256 of every download checked, and installing from a file marked "not verified". Approvals belong to the
  file's SHA-256. Disable and uninstall. The network is used only when you press a button and can be turned off.
- ✅ A plugin threat model in [SECURITY](../SECURITY.md#plugin-threat-model).
- Deviations from the plan, for the record: the permission is named `blocks` (not `commands`), an `open` permission was
  added, and a fuel budget replaces the wall-clock watchdog (it is deterministic, and a loop cannot outlive its fuel).
- [ ] Manual Windows/Linux validation of plugins (install from a file, the palette, links, the catalog); tracked as
  validation debt for Phase 7.

## ✅ Phase 6: Developer workflows · v0.5

- ✅ Snippets with `{{values}}` and an optional folder: a form in the command palette (⇧⌘S lists only snippets) and
  **Preferences → Snippets**. The text is typed at the prompt and never run, with every control character removed in
  Rust. See [snippets](snippets.md).
- ✅ Project tasks in the palette: the scripts of `package.json`, Cargo's commands and aliases, Makefile targets, justfile
  recipes and `deno.json` tasks, found the way each tool finds its file, with plain names only. Profiles can be tied to
  folders. See [project tasks](project-tasks.md).
- ✅ SSH and WSL profiles: `ssh … -- host` with the person's own agent and `~/.ssh/config`, no key, password or identity
  file stored, hosts suggested from `~/.ssh/config`; `wsl.exe` with the installed distributions listed. Values are
  checked twice and can never be read as an option. See [SSH and WSL](ssh-and-wsl.md).
- ✅ Developer integrations that never use the network: the git branch and state in the status bar (`core.fsmonitor`
  turned off, no locks, a timeout) and *Open the repository in the browser*, both off with `git_status = false`. See
  [git](git.md).
- ✅ `tools/live-tests.sh`: a signed-in SSH session against a throwaway `sshd`, run by hand.
- Deviations from the plan, for the record: the tasks are read when the palette opens (nothing is cached or watched),
  a remote profile never inherits a folder of this computer (found while wiring it), and the same `git_status` switch
  covers the repository address as well as the status bar.
- [ ] Manual Windows validation of WSL profiles (starting a distribution, the list from `wsl.exe`) and of the other new
  screens on Windows and Linux; tracked as validation debt for Phase 7.

## ✅ Phase 7: Stabilization · v1.0

- ✅ Performance budgets, measured with a reproducible bench mode (`TERMZIP_BENCH`) and `termzip-bench`, held in
  `docs/performance-budgets.toml` and checked in CI. Found and fixed on the way: a flood now merges queued reads (79,000 to
  200,000 lines per second, 1,048 to 611 MB). See [performance](performance.md#budgets).
- ✅ Conformance: 21 xterm.js sequence tests and PTY conformance tests. Accessibility: a screen reader setting, reduced
  motion, WCAG AA contrast of the interface in every theme, and axe-core checks of every screen (their findings fixed). See
  [accessibility](accessibility.md).
- ✅ Signing wired for macOS and Windows behind repository secrets, an update check that happens only on request with signed
  updates (tested end to end, a tampered file refused), and a Homebrew tap, a Scoop bucket and Winget manifests. See
  [updates](updates.md) and [distribution](distribution.md).
- ✅ Security pre-audit: `cargo deny` and `npm audit` in CI, twelve fuzz targets, and a written review with the scope for an
  independent audit. See [security review](security-review.md).
- ✅ A `Smoke` workflow that launches the built app on macOS, Windows and Linux. It found three long-standing bugs on
  Windows that no unit test could (a blank terminal, a PowerShell integration that recursed, and PowerShell blocks without
  their command); the bench found a marker parser bug on large outputs. All fixed.
- ✅ 1.0.0: what is promised to stay stable is in [stability](stability.md).
- Deviations from the plan, for the record: the cask is one file for both Macs instead of one per architecture (Homebrew's
  own style), the app budgets are checked with a wide slack on the CI runners (a runner starts the app cold), and the
  Winget manifests are written but not submitted.
- [ ] Manual validation by a person on Windows and Linux, and of screen reader support: the
  [validation checklist](validation-checklist.md) is the debt 1.0 carries.

## After 1.0

Outside what one person on one computer can finish:

- An **independent security audit** ([scope](security-review.md#for-an-independent-audit)).
- **Notarization** (an Apple Developer ID) and **Windows signing** (Azure Trusted Signing): the workflow is ready and turns
  on when the secrets exist ([distribution](distribution.md#signing-credentials)).
- **Interactive validation** on real Windows and Linux computers, with a screen reader.
- The **Winget** submission and **Flathub**.
- Ideas, unscheduled: an `api = 2` for plugins if something needs it, announcing command blocks to screen readers, and a
  faster flood on Windows.
