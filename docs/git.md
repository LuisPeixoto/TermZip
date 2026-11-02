# Git in the status bar

The status bar shows the git state of the folder of the active terminal:

```text
⑂ main ↑2 ↓1 ●
```

| Part | Meaning |
|---|---|
| `main` | The branch. A detached HEAD shows `detached 6004959`. |
| `↑2 ↓1` | Commits the branch has that its upstream does not (`↑`), and the other way round (`↓`). Not shown when it is in sync or has no upstream. |
| `●` | Files changed: staged, edited, new and not tracked. A red `!` instead means a merge conflict. |

Hover it for the details: `main · 2 commits ahead, 1 commit behind · 2 staged, 1 modified, 4 untracked`.

It is read when the terminal's folder changes, when a command finishes (a `git commit` shows up at once, in a shell with
[integration](shell-integration.md)) and when the window comes back to the front. It is never read on a timer, and a
burst of changes costs one run. Nothing is shown outside a repository.

## Open the repository

When the repository has an `origin` on a code host, the [command palette](keyboard.md#the-command-palette) has **Open
the repository in the browser**, with the host next to it. It opens `https://host/owner/repo`.

`https://…/repo.git`, `git@host:owner/repo.git`, `ssh://git@host:2222/…` and `git://…` all work. **Only the host and the
path are used**: a user name, a password or a token written in the address is dropped (never opened or shown), and an
SSH port is not turned into a web port. Anything that is not a plain host and a plain path (a folder on this computer, a
`file://` address, odd characters) is refused, and the address is opened through the same check as a click in the
terminal (only `http` and `https`).

## Turning it off

**Preferences → General → Use git in the status bar and the palette**, or in `config.toml`:

```toml
git_status = false
```

With it off, git is never started: not for the status bar and not for the repository address. It is on by default.

## What it does, and what it does not

- It runs `git status --porcelain=v2 --branch -z --no-renames` and `git config --get remote.origin.url` **on your
  computer**. There is no `fetch` and no network: `↑↓` compare with the last time you fetched.
- It runs for terminals on this computer only; an [SSH or WSL](ssh-and-wsl.md) terminal shows nothing.
- It needs `git` on your `PATH`; without it nothing is shown.
- A very big repository on a slow disk can take longer than the **2 seconds** git is given; then it is stopped and the item
  is empty. Counts are cut at 4 MiB of git's output and shown with a `+` (`12+ modified`).
- It uses git's own rules, including `safe.directory`: a repository owned by someone else is not read, as in git.

### A repository you just cloned

The config of a repository can ask git to run a program on every `git status` (`core.fsmonitor`). TermZip turns that
off for its runs, so looking at a repository you do not trust starts nothing. It also sets `GIT_OPTIONAL_LOCKS=0`, so
reading the state can never make your own `git commit` fail on the index lock, and `GIT_TERMINAL_PROMPT=0`, so nothing
waits for a password. See [SECURITY](../SECURITY.md#security-design).
