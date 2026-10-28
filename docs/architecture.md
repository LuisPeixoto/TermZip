# Architecture

TermZip is a Rust workspace plus a Tauri 2 desktop application. The shell engine is small and testable; the interface
is a Svelte 5 + Tailwind 4 webview with xterm.js as the terminal renderer.

```text
┌────────────────────────────────────────────┐
│ apps/desktop (webview)                     │
│ Svelte 5 · Tailwind 4 · xterm.js (WebGL)   │
│ Tabs · Panes · Blocks · Status bar         │
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
   events, `block` events (one per finished command), `cwd` events, and a final `exit` event. The channel belongs to
   one session, so events carry no session id and nothing can arrive before the terminal is ready for it (there is no
   backlog to replay).
3. The same bytes feed the OSC 133 parser; markers drive a per-session `BlockTracker`, and each finished command is
   sent as a `block` event (command text, exit code, duration, cwd) and each change of directory as a `cwd` event.
   New blocks are told apart by their index, because the tracker drops the oldest ones past 200.
4. The webview writes user input (`write_session`) — xterm.js encodes keys, bracketed paste, and control sequences.
   A block's **Run** types its command into the shell the same way.
5. `resize_session` keeps the emulator and the operating-system PTY in sync.
6. **Flow control.** The reader counts the bytes it sends and the webview acknowledges what xterm has drawn
   (`ack_session`). Past 1 MiB unacknowledged the reader waits, which stops draining the bounded PTY channel and makes
   the shell block on its own `write`; it resumes below 256 KiB (see [performance](performance.md)).
7. The `exit` event is always sent when the shell ends, with the exit code when it could be read. Destroying a session
   from the webview (a restart) stays silent, and events of an older shell are ignored by the webview.

Raw bytes flow to the terminal unchanged; the marker parser observes the stream without consuming it.

## Workspace packages

| Package | Responsibility |
|---|---|
| `termzip-core` | Settings (TOML, validated), profiles, the saved workspace, atomic writes, config paths. |
| `termzip-pty` | PTY creation, shell environment, reader thread, resize, child lifecycle. |
| `termzip-shell` | OSC 133/1337 marker parser, block tracker, integration scripts, shell launch config. |
| `termzip-desktop` | Tauri commands, session registry, event emission, app lifecycle. |

## The interface: panes, tabs and the workspace

The webview keeps one `Pane` per shell (session, exit, title, folder, blocks, size, find box) and a workspace of tabs,
each with a layout tree of panes (`layout.ts`, pure data and immutable operations). Panes are drawn as a flat list
keyed by id and positioned from the tree's rectangles, so a terminal never moves inside the page when another pane
is split off or closed; every tab stays mounted. The backend never sees the tree: it only starts a session for a
profile and stores a validated copy of the layout (`workspace.json`) when the webview asks. See
[tabs and panes](tabs-and-panes.md) and [profiles and restore](profiles-and-restore.md).

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
- Terminal output stays in memory; nothing is persisted except user preferences and, unless turned off, the layout
  of tabs and panes with the folder of each (never output, commands, or environment).
- The webview enforces a strict CSP (`default-src 'self'`), and IPC payloads are base64 bytes, never executable
  markup.
- Integration scripts are read-only helpers; they run with the shell's own permissions and add no network access.
- Shell output and escape sequences are untrusted input and must never become UI markup or commands.
- Everything is bounded: block output (64 KiB per block, 200 blocks), captured input (128 KiB), the marker parser
  (64 KiB of pending bytes), output not yet drawn (1 MiB), and a saved layout (64 KiB, 32 tabs, 16 panes per tab).
- Clicking text in the terminal goes to the backend, which only opens http(s) and mailto URLs and files that exist,
  and never starts a program (see [links](links.md)).