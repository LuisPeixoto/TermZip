# Security review

This is the maintainers' own review of TermZip before 1.0: what can reach the program, what it trusts, what was checked
and what was found. It is **not** an independent audit. [What an audit should cover](#for-an-independent-audit) is at the
end, so that one can be commissioned with a clear scope.

For the promises TermZip makes to users, and how to report a problem, see [SECURITY.md](../SECURITY.md).

## What TermZip is, for a reviewer

A desktop program that starts a shell with the permissions of the person using it. The Rust core owns every process, file
and network request; the Svelte page draws and has no capability beyond a short list of commands. There are no daemons,
no listening sockets and no background network traffic: the network is used only when a person presses **Load catalog**,
**Install** a plugin, or **Check for updates** (or turned on the optional check at launch).

## Trust boundaries

| Boundary | What is on the other side | Treated as |
| --- | --- | --- |
| Shell output → core | Anything a program prints, including escape sequences | Untrusted bytes |
| Core → webview (IPC) | Base64 bytes and JSON structs | Data, never markup |
| Webview → core (commands) | Requests from page code | A caller that can be wrong, so every argument is validated |
| Files in the data folder | `config.toml`, `workspace.json`, themes, plugin state | Untrusted input (the folder is the user's, but files get edited and corrupted) |
| Files of a project | `package.json`, Makefile, justfile, Cargo config | Untrusted input |
| Imported theme files | iTerm2, Windows Terminal, Ghostty, Kitty and Alacritty formats | Untrusted input |
| Plugin files | WebAssembly modules | Hostile code in a sandbox |
| The catalog and its downloads | A signed index served over HTTPS | Untrusted until the signature and hashes match |
| The update feed | `latest.json` and the update files | Untrusted until the signature matches the key built into the app |
| `~/.ssh/config`, git repositories | Host names, repository config | Untrusted input |
| Package channels | The Homebrew tap, the Scoop bucket, Winget | Copies of the release; the hashes come from the release's `SHA256SUMS` |

## Every input and its bounds

| Input | Bound | Where it is enforced |
| --- | --- | --- |
| Pending bytes of an unfinished OSC marker | 64 KiB, then dropped | `termzip-shell` marker parser |
| Output kept per command block | 64 KiB; 200 blocks per session | block tracker |
| Output not yet drawn by the terminal | 1 MiB (the shell is paused), 10 s of silence ends the account | `flow.rs` |
| Input typed before the shell is ready | 64 KiB | `lib/flow.ts` |
| `workspace.json` | 64 KiB, 32 tabs, 16 panes per tab, 8 split levels | core |
| Theme files | regular files of at most 256 KiB | theme importers |
| Project files | regular files of at most 256 KiB, 200 tasks | `termzip-tasks` |
| Snippet and task text typed at the prompt | control characters removed | core |
| Keyboard shortcuts | a chord needs Ctrl, Alt or Cmd; 64 entries | core |
| Profile environment | names and sizes checked; reserved names refused | core |
| SSH and WSL values | letters, digits, a few marks; never a leading `-`; `--` before the host | `termzip-shell`, settings |
| Clicked links | `http`, `https`, `mailto`, existing files; never started as programs | backend |
| Plugin call | 50 million fuel, 16 MiB memory, own thread; three failures stop it | `termzip-plugins` |
| Plugin reply | 64 KiB, 32 actions, text cleaned and cut | `termzip-plugins` sanitizer |
| Catalog index and downloads | signature first, then size and SHA-256; HTTPS only; three redirects | catalog |
| Update notes shown in Preferences | control and hidden characters removed, 2,000 characters, shown as plain text | `updates.rs` |
| `git` output | 2 s timeout; `core.fsmonitor` off; no network | `termzip-git` |

## The webview

- The content-security policy is `default-src 'self'`, with inline styles only (the terminal renderer needs them) and
  IPC only through Tauri's bridge.
- The main window's capability is `core:default`, dragging the window, and writing text to the clipboard. The page has
  **no** permission for dialogs, the file system, the shell, HTTP or the updater: files are chosen in native dialogs that
  Rust opens, and every network request is made by Rust.
- Terminal output is written to xterm.js as bytes, never as HTML. Text that comes from outside (update notes, plugin
  messages, theme names, tab names) is rendered as text.

## The update path

- The updater plugin checks a signature (minisign) against the public key in `tauri.conf.json` before it installs
  anything. The private key and its password are secrets of the repository, used only by the release workflow; a copy is
  kept by the maintainer.
- A person has to press **Check for updates**, and then **Install and restart**. Nothing is installed by itself and the
  optional check at launch is off by default.
- Tested end to end on a throwaway copy of the app: a tampered update file was refused, with a sentence saying the
  signature did not match, and the installed program was unchanged (same binary hash); the genuine file installed and the
  app restarted as the new version.
- Until the release builds are signed with an Apple Developer ID and a Windows certificate, the operating system will warn
  on first launch. The update signature protects the *update*, not the first download: check `SHA256SUMS`.
- If the signing key were lost or leaked, a release with a new key could not update existing installs: they would need a
  manual download. This is the cost of the key being built into the app.

## What this review found

Found by reading, by tests, or by the new fuzz targets and the smoke runs, and fixed in 1.0:

| Finding | Effect | Fix |
| --- | --- | --- |
| The marker parser lost track of a marker after 64 KiB of plain output | A command with a large output never finished its block | Index adjusted after trimming; tests with 64 KiB and more |
| Input typed before the session existed was dropped | On Windows the terminal stayed blank: ConPTY asks for the cursor position first and waits | A bounded queue flushes when the session is ready |
| The PowerShell integration recursed through its own prompt and never reported the command | Hundreds of empty blocks, a missing prompt | Rewritten around the line editor; found by the Windows smoke run |
| A command reported before any input marker was ignored | PowerShell blocks had no command text | The tracker opens the block when the command arrives |
| A test dependency (`happy-dom`) had a high advisory | Test environment only | Moved to a fixed version |
| `text-white` and several muted colors failed WCAG AA contrast | Accessibility | Tokens enforced at 4.5:1, also for imported themes |

Supply chain: `cargo deny` (advisories, licenses, bans, sources) and `npm audit --omit=dev --audit-level=high` run on every
push. One advisory is accepted and explained in `deny.toml` (RUSTSEC-2024-0370, a build-time macro helper of Tauri's GTK
bindings on Linux, with no runtime code and no safe upgrade).

Fuzzing: `fuzz/` has twelve targets (OSC parser, theme import, plugin module and reply, catalog index, snippets, task
files, ssh config, git output, wsl list, `workspace.json`, keybindings), run weekly and on demand by `fuzz.yml`.
A first on-demand run (2 minutes per target, 2026-11-02) found no crash in any of them. Two defects were found while
writing the targets, not by fuzzing: a snippet target split a string inside a UTF-8 character (a bug of the target itself)
and the target setup needed explicit `[[bin]]` sections. A longer run before each release is cheap and is the first thing
to repeat.

## Known limits

- The release builds are **unsigned** by an operating-system certificate until the credentials exist (see
  [distribution](distribution.md)); macOS builds are not notarized.
- The plugin sandbox relies on `wasmi`; a bug there would weaken it. A plugin with `output` and `open` can leak what it
  sees (see SECURITY.md); the interface marks that combination.
- `macOSPrivateApi` is used for the translucent window.
- Windows and Linux are checked by CI and the smoke run, not yet by a person using them every day.
- No independent audit has been done.

## For an independent audit

If one is commissioned, these are the parts where an expert's time is best spent, in this order:

1. **The plugin sandbox**: the `wasmi` configuration (fuel, memory, stack, imports refused), the reply sanitizer and the
   permission model (`crates/termzip-plugins`).
2. **Signature verification**: the catalog (minisign, prehashed, legacy refused, index not parsed before it matches) and
   the updater (key custody, what a leaked key allows, the endpoint over HTTPS).
3. **The shell-integration scripts and the way they reach a shell** (`crates/termzip-shell/src/scripts.rs`): quoting for
   POSIX, fish and PowerShell, and the OSC protocol they speak.
4. **The IPC surface**: every command in `apps/desktop/src-tauri/src/commands.rs` and what it checks.
5. **The untrusted file readers**: themes, project files, `workspace.json`, `config.toml`, ssh config.
6. **The release pipeline**: workflow permissions, secrets, what a compromised dependency could do in `release.yml`.

Materials: this document, SECURITY.md, `docs/architecture.md`, `docs/plugin-api.md`, `deny.toml`, `fuzz/`, the CI
workflows, and the tests next to each reader.
