# Architecture

TermZip is a Rust workspace plus a Tauri 2 desktop application. The shell engine is small and testable; the interface
is a Svelte 5 + Tailwind 4 webview with xterm.js as the terminal renderer.

```text
┌────────────────────────────────────────────┐
│ apps/desktop (webview)                     │
│ Svelte 5 · Tailwind 4 · xterm.js (WebGL)   │
│ Toolbar · Terminal · Blocks · Status bar   │
└───────────────┬────────────────────────────┘
                │ Tauri IPC: one channel per session, commands
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

1. The frontend opens a Tauri `Channel` and creates a session (`create_session`); Rust detects the shell, injects the
   integration script, and spawns it in a native PTY.
2. A reader thread sends everything the session reports on that channel, in order: raw PTY bytes as base64 `data`
   events, `blocks` events, and a final `exit` event. The channel belongs to one session, so events carry no session
   id and nothing can arrive before the terminal is ready for it (there is no backlog to replay).
3. The same bytes feed the OSC 133 parser; markers drive a per-session `BlockTracker`, and finished commands are sent
   as `blocks` events (command text, exit code, duration, cwd).
4. The webview writes user input (`write_session`) — xterm.js encodes keys, bracketed paste, and control sequences.
   A block's **Run** types its command into the shell the same way.
5. `resize_session` keeps the emulator and the operating-system PTY in sync.
6. The `exit` event is always sent when the shell ends, with the exit code when it could be read. Destroying a session
   from the webview (a restart) stays silent, and events of an older shell are ignored by the webview.

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
- **The terminal is the input surface.** You type into the persistent shell itself, so `cd`, environment variables,
  job control, history, and completion behave exactly as they do anywhere else. TermZip adds no second input box.
- **Application shortcuts never steal shell keys.** They are ⌘ on macOS and Ctrl+Shift elsewhere (`shortcuts.ts`),
  and the terminal hands them to the application instead of the shell.

## Security and privacy

- Child processes run with the launching user's permissions; no daemons, no network listeners.
- Terminal output stays in memory; nothing is persisted except user preferences.
- The webview enforces a strict CSP (`default-src 'self'`), and IPC payloads are base64 bytes, never executable
  markup.
- Integration scripts are read-only helpers; they run with the shell's own permissions and add no network access.
- Shell output and escape sequences are untrusted input and must never become UI markup or commands.
- Everything is bounded: block output (64 KiB per block, 200 blocks), captured input (128 KiB), and the marker parser
  (64 KiB of pending bytes).