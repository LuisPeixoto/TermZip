# Architecture

TermZip is a Rust workspace plus a Tauri 2 desktop application. The shell engine is small and testable; the interface
is a Svelte 5 + Tailwind 4 webview with xterm.js as the terminal renderer.

```text
┌────────────────────────────────────────────┐
│ apps/desktop (webview)                     │
│ Svelte 5 · Tailwind 4 · xterm.js (WebGL)   │
│ Toolbar · Terminal · Blocks · Composer     │
└───────────────┬────────────────────────────┘
                │ Tauri IPC: base64 data events, commands
┌───────────────▼────────────────────────────┐
│ apps/desktop/src-tauri (Rust)              │
│ Sessions · OSC 133 blocks · Settings       │
└──────┬──────────────────────┬──────────────┘
       │                      │
┌──────▼───────┐   ┌──────────▼──────────┐
│ termzip-pty  │   │ termzip-shell       │
│ PTY/ConPTY   │   │ Marker parser,      │
│ I/O, resize  │   │ launch config,      │
└──────┬───────┘   │ integration scripts │
       │           └─────────────────────┘
    Shell (zsh/bash/fish/PowerShell/cmd)
       │
┌──────▼──────────────┐
│ termzip-core        │
│ Settings, theme kind│
└─────────────────────┘
```

## Data flow

1. The frontend creates a session (`create_session`); Rust detects the shell, injects the integration script, and
   spawns it in a native PTY.
2. A reader thread forwards raw PTY bytes to the webview as base64 `pty-data` events and feeds the same bytes to the
   OSC 133 parser.
3. Markers drive a per-session `BlockTracker`; finished commands are emitted as `blocks-update` events (command text,
   exit code, duration, cwd).
4. The webview writes user input (`write_session`) — xterm.js encodes keys, bracketed paste, and control sequences.
5. `resize_session` keeps the emulator and the operating-system PTY in sync; a bounded backlog replays output
   captured before the webview attached.

Raw bytes flow to the terminal unchanged; the marker parser observes the stream without consuming it.

## Workspace packages

| Package | Responsibility |
|---|---|
| `termzip-core` | Settings (TOML, validated), theme kind, config paths. |
| `termzip-pty` | PTY creation, shell environment, reader thread, resize, child lifecycle. |
| `termzip-shell` | OSC 133/1337 marker parser, block tracker, integration scripts, shell launch config. |
| `termzip-desktop` | Tauri commands, session registry, event emission, app lifecycle. |

## Interface decisions

- **xterm.js renders the terminal.** A mature emulator gives correct keyboard encoding, mouse modes, bracketed paste,
  Unicode, and WebGL rendering without reimplementing VT behavior in the webview.
- **One emulator per session.** The webview owns terminal state; Rust owns processes, blocks, and settings. There is
  no duplicate VT state machine on the Rust side.
- **Blocks are observations, not edits.** TermZip never rewrites shell output; it records region boundaries so blocks
  can be listed, copied, and re-run.
- **The composer is an input surface.** It sends a line to the persistent shell; the shell keeps `cd`, environment
  variables, and job control between commands.

## Security and privacy

- Child processes run with the launching user's permissions; no daemons, no network listeners.
- Terminal output stays in memory; nothing is persisted except user preferences.
- The webview enforces a strict CSP (`default-src 'self'`), and IPC payloads are base64 bytes, never executable
  markup.
- Integration scripts are read-only helpers; they run with the shell's own permissions and add no network access.
- Shell output and escape sequences are untrusted input and must never become UI markup or commands.