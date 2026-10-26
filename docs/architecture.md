# Architecture

TermZip is a Rust Cargo workspace. The first implementation keeps the desktop interface, terminal parser, shell
process, and shared settings in separate packages so that performance and compatibility can be tested independently.

```text
┌──────────────────────┐
│ apps/desktop         │  eframe / egui / wgpu
└──────────┬───────────┘
           │ draws snapshots; handles input and resize
┌──────────▼───────────┐       ┌──────────────────────┐
│ termzip-terminal     │──────▶│ termzip-core         │
│ alacritty_terminal   │       │ themes and settings  │
└──────────▲───────────┘       └──────────────────────┘
           │ PTY output
┌──────────┴───────────┐
│ termzip-pty          │  portable-pty; bounded reader channel
└──────────┬───────────┘
           │
        Shell / child process
```

## Workspace packages

| Package | Responsibility |
|---|---|
| `termzip-core` | Shared settings, theme tokens, and platform configuration paths. |
| `termzip-terminal` | VT parsing, screen buffers, ANSI colors, cursor, and bounded scrollback. |
| `termzip-pty` | Native PTY creation, shell environment, process lifecycle, resize, and bounded output delivery. |
| `termzip-desktop` | Native window, bottom composer, terminal canvas, preferences, and keyboard routing. |

## Data flow

1. The desktop starts the configured shell in a cross-platform PTY.
2. A dedicated reader thread transfers bounded chunks to the UI and requests a repaint.
3. `termzip-terminal` incrementally parses VT/ANSI bytes; output is never parsed on a separate lossy string path.
4. The UI snapshots visible cells, draws them, returns VT-generated device-query responses to the PTY, and updates
   both the emulator and operating-system PTY dimensions.
5. Composer mode submits a complete line to the persistent shell. Terminal mode forwards text, common control keys, and
   navigation sequences to interactive programs.

The bounded PTY channel provides backpressure if rendering stalls rather than growing without limit. Terminal output
and command history are not written to disk. Theme and font preferences are stored in the standard per-user config
directory.

## Rendering and compatibility

The current renderer is a lightweight egui cell painter on eframe's wgpu backend. `alacritty_terminal` handles VT
state; TermZip resolves the terminal color palette and maps it to UI cells. This is an initial rendering backend, not
a measured claim that TermZip is faster or smaller than other terminals. GPU text atlas rendering, damage tracking,
HiDPI tuning, and reproducible benchmarks remain future work.

The bottom composer is an input surface attached to a conventional PTY shell; it does not yet capture commands as
first-class output blocks. Full-screen applications can use Terminal mode. See
[terminal compatibility](terminal-compatibility.md) for known limitations.

## Security and privacy

- Child processes run with the launching user's permissions.
- No remote service, telemetry, or network call is part of the application.
- PTY output is transferred in bounded chunks and kept in memory.
- Configuration contains user preferences only; permission-sensitive plugin execution is not implemented.
- Shell output and escape sequences are untrusted input and must not become UI markup or executable commands.

## Technical decisions

- **Rust across the application.** The first build avoids a JavaScript runtime and embedded browser.
- **Reuse a terminal state machine.** `alacritty_terminal` provides mature VT parsing and scrollback, avoiding a custom
  ANSI emulator in the UI.
- **Use portable PTYs.** `portable-pty` gives the shell a real terminal on Unix and ConPTY on Windows.
- **Keep direct terminal mode.** A pinned input bar must not sacrifice the key-routing expected by full-screen
  programs.
- **Validate UI framework assumptions.** eframe/egui with wgpu is the initial cross-platform prototype; accessibility,
  native text input, graphics compatibility, and memory use must be measured before stabilizing the choice.
