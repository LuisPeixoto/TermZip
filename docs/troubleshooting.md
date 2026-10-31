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

## The window is not translucent, or not blurred

Lower **Window opacity** in Preferences → Appearance: at 100 % the window is opaque on purpose. **Blur** needs macOS or
Windows; on Linux the desktop only shows through if your window manager composites windows (a compositor such as the
one in GNOME, KDE or picom), and there is no blur. See [appearance](appearance.md#the-window).

## A theme import failed

The message says why. The usual causes: the file is not a theme TermZip reads (it reads TermZip, iTerm2 `.itermcolors`,
Windows Terminal JSON, Ghostty, Kitty and Alacritty TOML; the old Alacritty YAML is not supported), a Windows Terminal
`settings.json` still has comments (remove them), a color is a name (`red`) instead of `#rrggbb`, or the file is larger
than 256 KiB. See [themes](themes.md#importing-a-theme).

## A theme I added is missing

The gallery lists the files it could not use, with the reason, under the cards: a file with a color that is not valid,
more than 128 custom themes, an id already used by another theme, or a file that is not a regular file. Fix the file
and reopen Preferences. A theme of the other appearance cannot be the dark or light choice and falls back to TermZip's
own.

## A shortcut I set does nothing, or the shell no longer gets a key

Open **Preferences → Keyboard**: a shortcut that another action took shows **Not set**, and the note under the action
that took it says so. TermZip receives the keys of every shortcut before the shell does, so a shortcut on a key the
shell uses (Ctrl+R, Alt+B…) keeps that key from the shell; the editor warns when you choose one. **Reset** puts the
default back. A `[keybindings]` entry that is not a well-formed chord (`"D"`, `"Shift+D"`) is ignored. See
[keyboard](keyboard.md).

## Right-click does not open a menu

There is no context menu yet. Copy with ⌘C (Ctrl+Shift+C on Windows and Linux) and paste with ⌘V (Ctrl+Shift+V).

## ⌘W closes a pane, not the window

That is intended: ⌘W (Ctrl+Shift+W elsewhere) closes the active pane, and the last pane closes the tab. The window
closes with the red button, and ⌘Q quits. On macOS the menu's **Close Window** item is removed, because a menu
accelerator is handled before the page sees the key.

## A tab or a split cannot be added

A window holds at most 32 tabs, a tab at most 16 panes, and splits nest at most 8 levels. Close something first. The
same limits apply to the saved layout. See [tabs and panes](tabs-and-panes.md#limits).

## The last session did not come back as it was

TermZip restores the **layout** (tabs, names, splits, profiles, folders), not what was running: the shells are new, and
the scrollback and the blocks are not saved. A folder that no longer exists falls back to the profile's folder. If
nothing came back at all, **Restore tabs and folders on launch** may be off in Preferences, or `workspace.json` was
unreadable and kept as `workspace.json.invalid`. See [profiles and restore](profiles-and-restore.md#session-restore).

## A profile's variable is not in my shell

Variables reach **new** shells only, so open a new tab with the profile (the caret next to **+**) instead of expecting
a shell that was already running to change. Names TermZip manages (`TERM`, `ZDOTDIR`, `TERMZIP_…`) cannot be set, and
Preferences say so when you save.

## A link does nothing when I click it

Opening needs **⌘+click** (Ctrl+click on Windows and Linux); a plain click only selects. A file must exist (a relative
path is looked up in the folder of the pane), and a program is never started by a click: its folder is opened
instead. A refusal shows as a message. See [links](links.md).

## Find shows fewer matches than I expect

Find searches what the pane still holds, so it stops at the **scrollback** setting (1,000 to 200,000 lines), and it
marks at most 1,000 matches (`1000+ matches`). The block filter only looks at the last 200 blocks of the pane.

## A command prints a lot and the screen lags behind

The terminal slows the shell down on purpose when it has 1 MiB of output it has not drawn yet, so the screen stays
close to what the program has done and Ctrl+C stops it. If the lag is large the page may be busy; see
[performance](performance.md).

## Plugins

### A plugin says “Off: the file changed after you turned it on”

Your approval is for the exact file you turned on, so a plugin whose file was replaced, edited or updated stays off
until you approve the new one. Turn it on again and read its permissions. If you did not change it, do not.

### A plugin says it crashed three times in a row

A plugin that traps, loops or sends an answer TermZip cannot use is stopped after three failures, and each failure showed
a message with the reason. Turn it off and on to try again; if it keeps failing, the plugin has a bug. A call that works
resets the count. A plugin that is merely slow never slows the terminal: its own queue of 32 events fills up and the
rest are dropped for it.

### A plugin file is rejected

The list under Installed (or the dialog) says why in a sentence: the file is over 4 MiB, it is not a WebAssembly module
or is a component, it imports something (“A plugin cannot call anything outside its sandbox”), it has no manifest or more
than one, the manifest is wrong (the sentence names the field), it was written for another plugin API, a required export
is missing, or its file name does not match its id. A file in the `plugins/` folder is skipped with the same sentence.

### The catalog does not load

- *The catalog is turned off in Preferences.* Tick **Allow loading the plugin catalog** and press **Save**
  (`plugins_catalog` in [configuration](configuration.md)).
- *Could not reach the server* or *took too long*: TermZip needs HTTPS access to GitHub, where the catalog is published; check the
  connection and any proxy, then press **Reload the catalog**.
- *The catalog's signature does not match*: nothing in the list was used. It was changed on its way, or it was not signed
  with TermZip's catalog key; try again later and, if it persists, report it as a [security
  issue](../SECURITY.md#reporting-a-vulnerability).
- *…does not match the checksum* on install: the download is not the file the signed list promised, so it was not
  installed.

### A plugin's command does nothing, or types nothing

Run it from the command palette (⇧⌘P, Ctrl+Shift+P) with a terminal focused, and check that the plugin is **On**. Typing
needs the `type` permission, opening an address needs `open`, and a plugin can only do either as the answer to a command
you ran. Typed text is pasted without Enter, so it waits at the prompt.

### A plugin's link is not underlined

Link patterns apply to what the terminal shows, one line at a time (the first 8 KiB of each), to plugins that are On.
Where two plugin patterns overlap the earlier match wins. Point at the text: it is underlined with a hint, and ⌘+click
(Ctrl+click elsewhere) opens it. A pattern that would match every line is rejected when the plugin is installed.

## Linux: the app does not build or start

TermZip needs the WebKitGTK packages: `libwebkit2gtk-4.1-dev`, `libappindicator3-dev`, and `librsvg2-dev` to build,
and WebKitGTK 4.1 at runtime. On a minimal system install them first, then run `npm run tauri dev` again.

## I want to try TermZip without touching my settings

Start it with a separate data folder:

```bash
TERMZIP_HOME=/tmp/termzip-demo npm run tauri dev
```

See [configuration](configuration.md#moving-the-data-folder).
