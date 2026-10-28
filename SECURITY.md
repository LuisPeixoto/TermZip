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
- Plugin loading is not implemented. Future extensions require an explicit isolation and permission design.

## Responsible use

TermZip is a terminal. Shell commands run with your privileges; review what you execute.