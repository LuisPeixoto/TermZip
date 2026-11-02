# Compatibility matrix

This matrix records what has actually been exercised, not only what compiles. Update it when a platform is tested on
real hardware.

| Area | macOS | Windows | Linux |
|---|---|---|---|
| Rust workspace tests | ✅ CI + local | ✅ CI | ✅ CI |
| Tauri desktop compiles | ✅ local + CI | ✅ CI | ✅ CI |
| Svelte check, Vitest, Vite build | ✅ local + CI | ✅ CI | ✅ CI |
| PTY/ConPTY smoke test | ✅ CI | ✅ CI (`cmd.exe /C echo`) | ✅ CI |
| zsh integration (PTY test) | ✅ local + CI | n/a | ✅ CI |
| bash integration (PTY test) | ✅ local + CI | n/a | ✅ CI |
| Session stream tests (`cargo test -p termzip-desktop`) | ✅ local + CI | n/a (Unix shells) | ✅ CI |
| Manual interactive validation | ✅ typing, blocks, themes, shortcuts, restart, preferences, tabs, splits, restore, profiles, find, filter, links (validation) | ⏳ pending | ⏳ pending |
| Themes: switching, previewing, importing an iTerm2 file in the native dialog | ✅ manual validation | ⏳ pending | ⏳ pending |
| Theme importers (real Dracula and Catppuccin files in every format, hostile input) | ✅ local + CI | ✅ CI | ✅ CI |
| Translucent window and blur | ✅ vibrancy, manual validation | ⏳ acrylic, pending | ⏳ transparency, pending |
| Shortcut editor and command palette | ✅ manual validation | ⏳ pending | ⏳ pending |
| vim / top / tmux / SSH | ⏳ pending | ⏳ pending | ⏳ pending |
| Snippets: palette, the form, typing without Enter, Preferences | ✅ manual validation | ⏳ pending | ⏳ pending |
| Project tasks and profiles tied to folders | ✅ manual validation | ⏳ pending | ⏳ pending |
| SSH profile: starts `ssh` and signs in to a real `sshd` (`tools/live-tests.sh`) | ✅ local | ⏳ pending | ⏳ pending |
| SSH profile editor and host suggestions from `~/.ssh/config` | ✅ manual validation | ⏳ pending | ⏳ pending |
| WSL profile: arguments and the UTF-16 list (unit tests) | ✅ local + CI | ✅ CI | ✅ CI |
| WSL profile: starting a distribution, the list from `wsl.exe` | n/a | ⏳ pending (validation debt) | n/a |
| Git in the status bar and Open the repository (real repositories in tests) | ✅ local + CI | ✅ CI | ✅ CI |
| Git item in the running app | ✅ manual validation | ⏳ pending | ⏳ pending |
| Packaged installer | ✅ debug app build (no bundle) | ⏳ release workflow | ⏳ release workflow |
| The built app launches, reaches a prompt and finishes a 100,000-line flood (`Smoke` workflow) | ✅ CI (Apple silicon runner) | ✅ CI (PowerShell) | ✅ CI (Xvfb) |
| xterm.js conformance (21 sequence tests) | ✅ local + CI | ✅ CI | ✅ CI |
| PTY conformance (`TERM`, resize, UTF-8, Ctrl+C, large paste) | ✅ local + CI | ✅ CI (ConPTY resize and UTF-8) | ✅ CI |
| Contrast of every theme, axe-core on every screen, reduced motion | ✅ local + CI | ✅ CI | ✅ CI |
| Screen reader support, used with a real screen reader | ⏳ pending (VoiceOver) | ⏳ pending (NVDA) | ⏳ pending (Orca) |
| Update check and install, tampered file refused | ✅ throwaway copy of the app | ⏳ pending | ⏳ pending |
| Homebrew cask installs (`--appdir`) | ✅ local | n/a | n/a |
| Scoop and Winget manifests | n/a | ⏳ pending (not run on Windows) | n/a |
| Fuzzing of every parser (12 targets, `Fuzz` workflow) | n/a | n/a | ✅ CI (no crash) |
| Performance budgets | ✅ local reference + CI | ✅ CI (headless) | ✅ CI (headless) |

Legend: ✅ verified, ⏳ pending, n/a not applicable.

The release workflow builds signed/unsigned bundles depending on repository secrets. A successful CI build and smoke run
are not a substitute for a person opening the packaged app on each platform: that is the
[validation checklist](validation-checklist.md), and the Windows and Linux rows marked pending are the debt 1.0 carries.

## Reproduce the macOS checks

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo test -p termzip-desktop
cd apps/desktop
npm run check && npm run test && npm run build
npm run tauri build -- --debug --no-bundle
```

Tests that need something real (a throwaway `sshd` for the SSH profile) are `#[ignore]`d and run with
`tools/live-tests.sh`; it starts what it needs on a free local port and deletes it when it ends, without touching your
`~/.ssh`.