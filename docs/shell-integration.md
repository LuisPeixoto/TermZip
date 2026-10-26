# Shell integration

TermZip detects the shell you configured (or `$SHELL` / `%COMSPEC%`) and, when the shell supports it, preloads a small
integration script that emits the semantic-prompt protocol:

- `OSC 133 ; A` — prompt starts
- `OSC 133 ; B` — prompt ends, command input starts
- `OSC 133 ; C` — the command starts executing
- `OSC 133 ; D ; <code>` — the command finished, with its exit code

TermZip's scripts also emit two private markers used for nicer blocks:

- `OSC 1337 ; CurrentDir=<path>` — the shell's working directory
- `OSC 1337 ; CommandB64=<base64>` — the exact command line

## Supported shells

| Shell | Integration | Notes |
|---|---|---|
| zsh | Full | Injected through a `ZDOTDIR` shim that sources your own `~/.zshenv`/`~/.zshrc`; hooks via `add-zsh-hook`. |
| bash | Full | Injected through `--rcfile` after sourcing `~/.bashrc`; uses a `DEBUG` trap. |
| fish | Full | Loaded with `fish -C`; uses `fish_preexec` / `fish_postexec` events. |
| PowerShell | Partial | Blocks include the command (from history), exit code, and duration; there is no pre-exec hook, so the input region is empty. |
| cmd.exe | None | Plain terminal; no block capture. |
| Other (tcsh, nushell…) | None | Plain terminal; no block capture. |

The status bar shows **shell integration** when markers are available and **plain terminal** when they are not.

## What integration does not do

- It never changes your prompt text, aliases, or plugins — it only appends marker emission hooks.
- It never sends anything over the network.
- Compound bash commands emit one block for the whole line; bash's `DEBUG` trap fires per simple command, so nested
  subcommands are folded into the first block.
- Password prompts and hidden input cannot be captured; those blocks show “command not captured”.

## Manual sanity check

With an integrated shell running inside TermZip, the following should produce a block with exit code 0:

```console
$ echo termzip-ok
```

If blocks never appear, check the shell path in **Preferences** and make sure the file is the real shell binary
(a wrapper script may swallow `--rcfile` or `-C` flags).