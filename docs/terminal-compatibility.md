# Terminal compatibility

TermZip uses xterm.js as its terminal emulator inside the Tauri webview and `portable-pty` for process sessions.
This document tracks what is verified, what is implemented but not exhaustively tested, and what is still missing.

## Implemented

- Full-screen and interactive programs (vim, less, top, tmux, SSH, REPLs) through the real terminal canvas.
- Keyboard encoding by the emulator itself: arrows, navigation, function sequences, Ctrl+letter, alt, repeat.
- Bracketed paste (automatic), mouse reporting passthrough, alternate screen, scrollback, resize.
- Selection with copy (⌘C / Ctrl+Shift+C), paste (⌘V / Ctrl+Shift+V), select-all (⌘A / Ctrl+Shift+A).
- Command blocks with command text, exit code, duration, working directory, and a bounded output preview on
  integrated shells.
- The terminal is the only input: your shell's line editor, history, completion, and keybindings apply unchanged.
- Application shortcuts that never collide with shell keys (⌘ on macOS, Ctrl+Shift elsewhere).
- Tabs and split panes, each pane a real PTY; search in the scrollback; clickable URLs, OSC 8 hyperlinks, and file paths.

## Verified locally (macOS)

- zsh integration: prompt, `cd`, environment changes persist; blocks appear with correct exit codes and durations.
- PTY smoke test on the CI matrix (Unix shells and `cmd.exe /C echo` on Windows).
- VT-level behavior is inherited from xterm.js, which maintains its own extensive compatibility test suite.
- Tabs, splits, session restore, profiles, find, the block filter, and links, driven in the running app; a flood of two
  million lines with Ctrl+C (see [performance](performance.md)).

## Tested in the repository

- **The emulator** (`apps/desktop/tests/conformance.spec.ts`, 21 tests with `@xterm/headless`, the same version as the
  terminal): cursor movement, erase, SGR including truecolor and 256 colors, the alternate screen, scroll regions, wide
  characters and emoji, combining marks, OSC 8 hyperlinks, bracketed paste, mouse modes, and the cursor style. A
  dependency upgrade that changes one of these fails the test.
- **The PTY** (`crates/termzip-pty/tests/conformance.rs`): on Unix, `TERM` and the size reach the program, a resize
  changes `stty size`, UTF-8 round-trips, Ctrl+C reaches the foreground job, a 300 KB paste arrives whole, and closing
  ends the child; on Windows, ConPTY resize and UTF-8.
- **The whole app** (the `Smoke` workflow): the built app is launched on macOS, Linux (under Xvfb) and Windows (with
  PowerShell), types a flood of 100,000 lines and checks that the prompt and the finished block arrive.

## Known limitations

- **Windows and Linux interactive use is not yet manually verified by a person.** CI compiles, lints, tests, and the smoke
  run launches the app on both and checks a prompt and a flood, but vim, tmux and SSH sessions there still need
  hands-on validation (see the [validation checklist](validation-checklist.md)).
- Bash compound commands are recorded as a single block; the `DEBUG` trap folds subcommands into the first block.
- On Windows, a flood is much slower than elsewhere (about 4,600 lines per second for 100,000 lines on a CI runner,
  against 200,000 on a Mac): the console layer and PowerShell's own output are the limit.
- Very large outputs are shown in full in the terminal; each block keeps at most 64 KiB of output and reports when it
  was truncated.
- Block output strips terminal control sequences for display while preserving SGR colors; cursor-addressed output
  (for example progress animations) is shown as sequential text.
- Hyperlinks (OSC 8) open with ⌘+click (Ctrl+click elsewhere), http and https only; see [links](links.md).

## Reporting a problem

Open an issue with the operating system, shell, expected behavior, actual behavior, and a minimal reproduction
(for example, a command that renders incorrectly). Remove command output, environment variables, and file paths you
do not want to publish.