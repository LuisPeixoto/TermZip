# Troubleshooting

Each heading is a symptom, followed by the usual cause and the fix. If yours is not here, open an issue with the
operating system, the shell, and what you expected (see
[terminal compatibility](terminal-compatibility.md#reporting-a-problem)).

## Blocks never appear

The status bar says **plain terminal** or **no block capture**: the shell is not integrated. Integration works for
zsh, bash, fish, and PowerShell; cmd.exe, tcsh, and nushell run as a plain terminal. Check the shell path in
Preferences and make sure it points at the real shell binary, not a wrapper script that swallows `--rcfile` or `-C`.
See [shell integration](shell-integration.md).

If the status bar says **shell integration** but a command still has no block, it may be a password prompt or hidden
input, which cannot be captured.

## A block shows “command not captured”

The shell printed the command's boundaries but TermZip could not read the command text, usually because the input was
hidden (a password) or the line was edited by a plugin that redraws it. The output and exit code are still recorded.

## A block shows “no output captured”

The command printed nothing (`cd`, `export`, a successful `git add`). That is expected.

## The shortcuts do nothing on Windows or Linux

The application shortcuts use **Ctrl+Shift** on Windows and Linux (`Ctrl+Shift+R` restarts, `Ctrl+Shift+B` toggles
blocks) so that plain Ctrl+R and Ctrl+B reach your shell. Preferences is `Ctrl+,`. The status bar shows the shortcuts
for your platform.

## Typing does nothing

Click the terminal once. TermZip gives the keyboard back to the terminal when the window regains focus or a dialog
closes, but clicking elsewhere in the interface (for example on a block button) moves the keyboard focus there.

## “Shell exited” and nothing responds

The shell ended (you ran `exit`, or it crashed). Use **Restart** in the toolbar or the restart shortcut. If the shell
exits immediately every time, the path in Preferences is probably wrong; clear it to use your default shell.

## “Could not start the shell”

The shell path in Preferences does not exist or is not executable, or the working directory does not exist. Clear the
field or fix the path, then restart the shell.

## My preferences were reset

TermZip could not read `config.toml` (invalid TOML or a value of the wrong type), so it started with the defaults. The
old file was kept next to it as `config.toml.invalid`; fix it by hand and rename it back to get your settings. See
[configuration](configuration.md#a-file-that-cannot-be-read).

## A font or shell change did not apply

Font size, font family, scrollback, and theme apply as soon as you save. The shell path and the working directory are
read when a session starts: restart the shell after changing them.

## Icons show as empty boxes

The terminal font does not contain the glyphs your tools print (for example the icons from `eza --icons` need a Nerd
Font). Install one and set it as **Font family** in Preferences.

## Right-click does not open a menu

There is no context menu yet. Copy with ⌘C (Ctrl+Shift+C on Windows and Linux) and paste with ⌘V (Ctrl+Shift+V).

## Linux: the app does not build or start

TermZip needs the WebKitGTK packages: `libwebkit2gtk-4.1-dev`, `libappindicator3-dev`, and `librsvg2-dev` to build,
and WebKitGTK 4.1 at runtime. On a minimal system install them first, then run `npm run tauri dev` again.

## I want to try TermZip without touching my settings

Start it with a separate data folder:

```bash
TERMZIP_HOME=/tmp/termzip-demo npm run tauri dev
```

See [configuration](configuration.md#moving-the-data-folder).
