# Roadmap

Items are prioritized within each phase. Features move to complete only after their acceptance checks pass. Open an
issue before beginning a larger architectural change.

## ✅ Phase 0: Research and feasibility

- ✅ Static REA inspection of the supplied Warp, Ghostty, and Terminal.app binaries and macOS bundle metadata.
- ✅ Review of ProxyZip's repository structure, roadmap, CI, contribution workflow, and Rust workspace conventions.
- ✅ Original visual direction chosen: minimal premium, bottom-anchored composer, zinc & blue palette.
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
- [ ] First end-to-end CI run of the new workflows on all three platforms.

## 🚧 Phase 2: Usable terminal MVP · v0.1

- ✅ Native shell in a PTY; raw output streamed to xterm.js off the read thread.
- ✅ Bottom-anchored composer with multiline input, local history, and Enter-to-run.
- ✅ Shell integration (OSC 133) for zsh, bash, fish, and partial PowerShell; cmd.exe runs plain.
- ✅ Command blocks with command, exit code, duration, cwd, copy, and re-run.
- ✅ Interactive programs keep working: alternate screen, bracketed paste, selection, clipboard, resize.
- ✅ Preferences: theme, font size/family, scrollback, shell path, working directory — persisted and clamped.
- [ ] Manual compatibility matrix: zsh, bash, fish, PowerShell, cmd.exe, SSH, vim, less, top/htop, tmux per platform.
- [ ] Scrollback selection alongside visible-screen selection; mouse-report clicks forwarded to the shell.
- [ ] Session lifecycle polish: exit detection status, restart affordance in the toolbar, tab title updates.

## 📋 Phase 3: Everyday productivity · v0.2

- [ ] Multiple tabs and split panes.
- [ ] Search terminal history and output; block filtering.
- [ ] Session restore, profiles, links, and keyboard customization.
- [ ] Large-output virtualization and incremental rendering budgets.

## 📋 Phase 4: Personalization · v0.3

- [ ] Theme import/export and a gallery of audited themes.
- [ ] Cursor styles, opacity, spacing, per-profile shell settings.
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