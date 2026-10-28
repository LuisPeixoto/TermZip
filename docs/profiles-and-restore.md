# Profiles and session restore

## Profiles

A profile is a name with a **shell**, a **folder** and some **environment variables**. Everything it leaves empty
comes from the built-in profile, which is the shell and working directory in Preferences, so a profile only says what
differs.

Create them in **Preferences → Profiles** (add, edit, delete, and choose which one is the default), or write them in
[`config.toml`](configuration.md#profiles):

```toml
default_profile = "work"

[[profiles]]
id = "work"
name = "Work"
working_directory = "/srv/work"

[profiles.env]
AWS_PROFILE = "work"
```

- A **plain new tab** (⌘T or **+**) uses the default profile. With no default profile it uses the built-in one.
- The **caret** next to **+** opens a tab with a chosen profile, or with **Built-in** even when another profile is the
  default.
- A new tab or split without an explicit profile starts in the folder of the pane you were in. Choosing a profile from
  the menu starts in that profile's own folder.
- Variables reach **new** shells only; open shells keep the environment they started with.

### Limits and rules

| What | Limit |
|---|---|
| Profiles | 32 |
| Name | 64 characters |
| Variables per profile | 64 |
| Variable name | letters, digits and `_`, not starting with a digit, up to 128 characters |
| Variable value | 4 KiB, no NUL character |

`TERM`, `COLORTERM`, `TERM_PROGRAM`, `TERM_PROGRAM_VERSION`, `ZDOTDIR` and anything starting with `TERMZIP_` are managed
by TermZip and cannot be set: the shell integration depends on them. Preferences explain a mistake as soon as you
make it, and the backend explains it again when you save.

## Session restore

When you quit, TermZip remembers the **layout** and opens it again next time: your tabs, their names, the splits and
their sizes, the profile of each pane, the folder each shell was in, and which pane had the keyboard.

- It lives in `workspace.json` next to `config.toml` (see [configuration](configuration.md#moving-the-data-folder)). It
  holds structure only: **no terminal output, no commands and no environment are ever written to it.**
- It is saved a moment after the layout changes (a burst of `cd` commands or a divider drag costs one write), and once
  more when the window closes.
- The shells are **new**. Running programs, the scrollback and the blocks are not restored, and a folder that no longer
  exists falls back to the profile's folder.
- Turn it off in Preferences with **Restore tabs and folders on launch**. Turning it off also deletes the saved file, so
  nothing about your folders stays on disk.

The file is treated as untrusted input when it is read: at most 64 KiB, 32 tabs, 16 panes per tab and 8 levels of
splits, version 1. A file that breaks a limit or cannot be parsed is not partly restored; it is kept as
`workspace.json.invalid` and TermZip starts with one fresh tab.
