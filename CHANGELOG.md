# Changelog

Notable changes to TermZip are documented here.

## [Unreleased]

### Added

- Desktop interface rebuilt on Tauri 2, Svelte 5, Tailwind 4, and xterm.js with a zinc & blue design system.
- Bottom-anchored command composer with multiline input and local history.
- Shell integration (OSC 133 + private 1337 markers) for zsh, bash, fish, and PowerShell with a bundled script per
  shell, injected without touching user configuration.
- Command blocks with command text, exit code, duration, working directory, ANSI-colored output preview, and copy /
  re-run / output-copy actions.
- Full terminal canvas (xterm.js): keyboard encoding, bracketed paste, selection with clipboard shortcuts, alternate
  screen, scrollback, mouse reporting, and resize on all platforms via PTY/ConPTY.
- Preferences for theme, font size, font family, scrollback lines, shell path, and working directory, persisted with
  validation and clamping.
- New workspace crate `termzip-shell` (marker parser, block tracker, launch configuration) with unit tests.
- GitHub Actions: desktop job (svelte-check, vitest, vite build, clippy), Tauri bundle release workflow, npm
  Dependabot updates.

### Changed

- Replaced the first egui prototype and the in-repository VT engine (`termzip-terminal`) with the Tauri + xterm.js
  architecture; terminal state lives in the webview, processes and blocks in Rust.
- `termzip-core` settings gained `font_family`, validation, and clamped ranges; existing configs keep working.

### Removed

- egui/eframe dependency and the hand-written terminal renderer.