# Validation checklist

What continuous integration cannot tell you. CI builds the app on macOS, Windows and Linux, runs the tests, and the
`Smoke` workflow launches the built app on each and checks that a window, a prompt and the block of a large flood arrive
(see the [compatibility matrix](compatibility-matrix.md)). A person using the app on each system is still the best test.
Run this list on a clean user account before a release and on each platform you can reach, and note anything that is not
as described in an issue.

Use a throwaway data folder so your own settings are not touched: set `TERMZIP_HOME` to an empty folder.

## Every platform

- [ ] The app starts, a prompt appears, and typing, Backspace, arrows, Tab completion and history work.
- [ ] `ls` / `dir`, then `cd` somewhere: the block shows the command, a zero exit code and a duration; the folder follows.
- [ ] A failing command (`false`, or `exit 1` in a subshell) shows a failed block.
- [ ] `seq 1 1000000` (PowerShell: `1..1000000`) finishes, and Ctrl+C stops a longer one at once.
- [ ] Tabs: new, rename, reorder, close. Splits: right, down, move between panes, resize.
- [ ] Quit and reopen: tabs, splits and folders are back. With *Restore session* off, they are not.
- [ ] Find (⌘F / Ctrl+Shift+F): highlights and a count. The block filter works.
- [ ] ⌘+click / Ctrl+click on a URL and on a file path opens it; a click on a program only shows its folder.
- [ ] Preferences: change the theme, the font size and the cursor; they apply at once. Import a theme file.
- [ ] The command palette lists actions, tabs, profiles, themes, snippets and (in a project) tasks; choosing a task types it
      and does not press Enter.
- [ ] Preferences → Plugins: install a plugin from a file, see its permissions, turn it on, use it, turn it off.
- [ ] Preferences → General → Check for updates: the check answers; nothing is installed without a click.
- [ ] A translucent window (opacity below 1.0) shows the desktop through it and the text stays readable.
- [ ] Reduced motion in the system settings stops the cursor blink.
- [ ] With *Screen reader support* on and the platform's reader running (VoiceOver, NVDA, Orca), the prompt and new output
      are read, the tabs and Preferences are announced, and focus is never lost after closing a dialog.

## Full-screen programs

- [ ] `vim` (or `nvim`): open, edit, save, quit; the screen is restored on exit.
- [ ] `less` on a long file, `top` or `htop`, `tmux` with a split: arrows, mouse, resize.
- [ ] `ssh` to a host you own, with an SSH profile and with a plain `ssh` typed by hand.

## macOS

- [ ] The first launch of a download that is not notarized: right-click → Open works, and the cask's note says so.
- [ ] Vibrancy (blur) with a translucent window. `brew install --cask luispeixoto/termzip/termzip` installs it.

## Windows

- [ ] PowerShell 7 and Windows PowerShell 5.1: a prompt, blocks with the command text and exit codes, Enter and history
      keep working (TermZip rebinds Enter in PSReadLine to read the line; see [shell integration](shell-integration.md)).
- [ ] `cmd.exe` works as a plain terminal (no blocks).
- [ ] A WSL profile lists the installed distributions and starts one.
- [ ] Acrylic blur with a translucent window. SmartScreen's first-launch warning is expected while the build is unsigned.
- [ ] The NSIS installer and the `.msi` install and uninstall cleanly; `scoop install termzip` works from the bucket.

## Linux

- [ ] The AppImage starts (`chmod +x`), and the `.deb` and `.rpm` install on a distribution that has WebKitGTK 4.1.
- [ ] Wayland and X11: the window, the clipboard (Ctrl+Shift+C / V) and the translucent window.
- [ ] bash, zsh and fish each give blocks.
