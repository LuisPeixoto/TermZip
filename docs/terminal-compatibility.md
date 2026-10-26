# Terminal compatibility

TermZip is at the first terminal-emulation milestone. It currently uses the VT parser and screen grid from
`alacritty_terminal`, starts a shell with `portable-pty`, and presents a separate command composer at the bottom.

## Current modes

- **Composer mode:** type a complete command in the pinned footer and press Enter. The command is sent to the existing
  shell, so working directory and process state are retained.
- **Terminal mode:** click the terminal canvas to focus it. Text, Enter, Tab, Backspace, Escape, common arrow/navigation
  keys, and Ctrl+A–Z are forwarded to the process. Press Escape to return to Composer mode.
- **Scrollback:** scroll the terminal canvas when the running program is not using mouse reporting.
- **Selection and clipboard:** drag across visible cells to select text; copy with `⌘C` on macOS or `Ctrl+Shift+C` on
  Windows/Linux, and paste using bracketed-paste delimiters when the running program enables them.
- **Resize:** terminal rows and columns follow the available canvas size.

## Known limitations

- The shell's own prompt and the bottom composer are both visible. Shell hooks and command/output blocks are not
  implemented yet.
- The first key-routing implementation does not cover every function key, key protocol, mouse-reporting mode, paste
  protocol, or platform-specific shortcut.
- Text selection currently covers the visible screen only; scrollback selection, mouse-reporting clicks, hyperlink
  navigation, and image rendering are not implemented.
- Complex shell editing, multiline commands, bracketed paste, and completion behavior need broader testing.
- Initial PTY support is validated on the host macOS environment; Windows and Linux CI build/test coverage must pass
  before those platforms are called verified for interactive use.

Report a compatibility problem with the operating system, shell, expected behavior, and a minimal reproduction.
Remove command output, environment variables, and file paths that you do not want to publish.
