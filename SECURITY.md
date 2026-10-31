# Security policy

TermZip launches a shell with the permissions of the current user and renders terminal content in a Tauri webview.
Treat the application, its configuration, and its shell-integration scripts as part of your local execution
environment.

## Supported versions

Before the first stable release, security fixes are developed against the latest `main` branch. Once releases begin,
the latest release receives security fixes while the project is below 1.0.

## Reporting a vulnerability

Please do not report exploitable vulnerabilities in public issues. Use
[GitHub's private security advisory form](https://github.com/LuisPeixoto/TermZip/security/advisories/new). Include
the affected version, operating system, impact, and a reproduction or proof of concept when safe to do so.

## Security design

- The shell inherits the current user's normal operating-system permissions; TermZip spawns no daemons and opens no
  network listeners.
- The webview enforces a strict content-security policy (`default-src 'self'`; inline styles allowed for the
  terminal renderer; IPC only through Tauri's bridge).
- IPC payloads between the Rust core and the webview are base64-encoded bytes or JSON structs — never executable
  markup. Terminal output is never interpreted as HTML.
- Terminal output is kept in memory; nothing except user preferences is persisted to disk.
- Integration scripts are written to the user's config directory and add hooks to the user's shell; they perform no
  network access and do not modify user dotfiles beyond sourcing them.
- Settings are written atomically (temporary file, then rename) into a folder restricted to the user (`0700`, file
  `0600`) on macOS and Linux. A file that cannot be parsed is renamed to `config.toml.invalid` instead of being
  overwritten. `TERMZIP_HOME` moves the data folder; point it at a folder that only TermZip uses, because TermZip
  restricts its permissions.
- The path of the integration scripts is quoted for each shell (POSIX, fish, PowerShell) before it reaches a shell
  command line, so a quote, a space, or a `$(...)` in the data folder is never evaluated at startup. If the scripts
  cannot be prepared, the shell starts without integration.
- Command blocks keep what a command printed in memory only, bounded to 64 KiB per block and 200 blocks per session;
  the OSC marker parser holds at most 64 KiB of pending bytes. Output can contain secrets or personal data: nothing is
  written to disk, but anything you copy leaves TermZip through your clipboard.
- Clicking text in the terminal is validated by the backend before anything reaches the operating system: only
  `http`, `https` and `mailto` URLs, and files that exist (relative paths are resolved against the pane's folder, `..`
  and symlinks before the check). A program, script, installer or application is never started by a click: its folder
  is opened instead, also when it is reached through a symlink. Opening needs ⌘ (Ctrl elsewhere) held.
- The saved layout (`workspace.json`) holds structure only: tabs, names, splits, profiles and folders. It is read as
  untrusted input: at most 64 KiB, 32 tabs, 16 panes per tab and 8 levels of splits, and a file that breaks a limit or
  cannot be parsed is kept as `.invalid` instead of being partly used. Turning session restore off deletes it.
- Profile environment variables are validated (names, sizes, no NUL) and cannot override the variables the shell
  integration depends on (`TERM`, `COLORTERM`, `TERM_PROGRAM*`, `ZDOTDIR`, `TERMZIP_*`). They are stored in
  `config.toml` in plain text, so do not put secrets in them.
- Output not yet drawn by the terminal is bounded (1 MiB): the backend pauses the shell instead of queueing without
  limit, and gives up on the account if the page is silent for 10 seconds, so a stuck page cannot freeze a shell for
  good.
- Theme files are untrusted input. Importing reads one file you pick in a native dialog that Rust opens (the webview has
  no dialog permission), only if it is a regular file of at most 256 KiB; the readers for iTerm2 (plist), Windows
  Terminal (JSON), Ghostty, Kitty and Alacritty (TOML) return a sentence instead of panicking, clamp or refuse
  out-of-range values, and are tested against truncated, oversized, deeply nested and wrong-typed input. The result is
  written as one file in the themes folder (`0700`, files `0600`), never over another theme. An export is a plain write
  where you choose in a save dialog. A theme is data: colors only, no scripts, and its text is never interpreted as
  markup.
- Keyboard shortcuts are validated (a chord needs Ctrl, Alt or Cmd, so typing cannot be taken over; at most 64 entries)
  and a hand-edited `[keybindings]` can never make one key do two things.
- On macOS the translucent window uses Tauri's `macOSPrivateApi` (a private Apple API), which makes the app ineligible
  for the Mac App Store. It is used only to make the window transparent and to ask the system for its blur.
- Plugins are WebAssembly files that run in a sandbox with no imports: they cannot reach files, the network, processes
  or the clock. Each permission is shown before a plugin is turned on, the approval belongs to the SHA-256 of the file,
  and what a plugin answers is checked and cleaned before it is used. The catalog is verified with a built-in key and
  the network is used only when you press a button. See the threat model below.

## Plugin threat model

A plugin is code written by someone else that you choose to run inside TermZip. This is what the design assumes, what
it prevents, and what it cannot.

**What is protected:** your files, your secrets (what you type and what commands print), your network, and your shell.

**Who is assumed hostile:** a plugin file you install (it may be written to harm you); a network attacker or a hosting
account that tampers with the catalog or a download; a plugin author's mistake (a loop, a crash, a huge answer).

**What prevents harm**

- *The sandbox.* A plugin is a core WebAssembly module run by `wasmi`, an interpreter written in Rust (no generated
  machine code). A module that imports anything is refused, so a plugin has **no function to call out with**: no files,
  network, processes, clock or randomness. Each call has a fuel budget (50 million units), a memory cap (16 MiB) and a
  stack cap, and runs on its own thread with a bounded queue, so a loop, a flood or a huge allocation only costs that
  plugin its call, and three failures in a row stop it. TermZip itself never waits for a plugin.
- *A narrow way out.* A plugin's only output is a JSON reply of at most 64 KiB and 32 actions. Every action is checked
  against the plugin's permissions, and its text loses control, bidirectional and zero-width characters and is cut to
  a limit. Nothing a plugin says is interpreted as markup or as a command.
- *Typing is never running.* A plugin can type only as the answer to a palette command **you** ran, only with the `type`
  permission, as a paste into that terminal, with line breaks and control characters removed (4 KiB at most). It cannot
  press Enter, so nothing runs until you read the line and decide. A reaction to a finished command can never type.
- *Opening an address is a click you asked for.* `open` works only as the answer to a command you ran, with the `open`
  permission, for `http` and `https` addresses with an ASCII host and no `user@` part, through the same checks as a click.
  Link patterns put terminal text into an address only after percent-encoding it, and never into the host name.
- *Permissions and approvals.* You see one sentence per permission before a plugin is turned on. The approval is the
  SHA-256 of the file: a plugin that is replaced, edited or updated is off until you approve the new file. `state.json`
  is read as untrusted input, and one that cannot be read approves nothing.
- *The catalog.* The index and its minisign signature are fetched together, and the index is **not parsed** until the
  signature (the prehashed kind; the legacy kind is refused) matches the public key built into TermZip. The download
  must then have the size and SHA-256 the signed index promises, and carry a manifest with the same id, version and
  permissions. All requests use HTTPS with certificate checks (`rustls`), at most three redirects, bounded sizes and a
  timeout. The network is used only when you press **Load catalog** or **Install**, never in the background, and
  `plugins_catalog = false` turns it off entirely. A debug build (never a release build) can be pointed at a catalog on
  `localhost` for development; the signature is still checked, against the key given with it.
- *Key custody.* The signing key is kept as secrets of the catalog repository and is used only by a workflow that a
  maintainer starts by hand, which also checks the signature it made. The public key is built into TermZip, so changing
  it needs a new release.

**What it does not prevent** (read this before you turn plugins on)

- A plugin with `blocks` or `output` sees your commands and, with `output`, what they print, which can include secrets.
  It cannot send them anywhere by itself, but a plugin that also has `open` can encode what it saw into the address it
  opens when you run one of its commands. Give `output` and `open` only to plugins you trust; the interface marks them.
- A plugin can type a misleading command at your prompt. You read it before pressing Enter; read it.
- A plugin can label a block or show a message that is untrue.
- A plugin from a file is **not verified by anyone**. The signature proves a catalog entry came from the catalog's key,
  not that the plugin is good: whoever holds that key, or the repository's secrets, can publish a plugin there. You
  still see its permissions and approve it, and an update is a new file that needs a new approval.
- A bug in `wasmi`, in `rustls` or in the interpreter's limits would weaken the sandbox. Keep TermZip updated.
- Anyone who can write to your TermZip data folder already has your user's rights; the approvals bind the plugin's
  contents, not the folder.

Plugin and catalog problems are reported like any other vulnerability, privately (see above).

## Responsible use

TermZip is a terminal. Shell commands run with your privileges; review what you execute.