# SSH and WSL profiles

A [profile](profiles-and-restore.md) can start an **SSH connection** or a **WSL distribution** instead of a shell on
this computer. The tab opens already connected.

## SSH

In **Preferences → Profiles**, choose **Add a profile** and set *Starts* to **An SSH connection**:

| Field | |
|---|---|
| Host | A name from your SSH config (the field suggests the `Host` names of `~/.ssh/config`), a host name or an address. |
| User | Empty uses the one `ssh` would. |
| Port | Empty uses the one `ssh` would. |

```toml
[[profiles]]
name = "Build box"
kind = "ssh"
host = "build-box"      # a Host of ~/.ssh/config, or a name or an address
user = "deploy"         # optional
port = 2222             # optional
```

TermZip runs `ssh [-l user] [-p port] -- host`. **It stores no key, password, passphrase or identity-file path.**
Signing in is `ssh`'s own business: your agent, your keys, `~/.ssh/config`, `known_hosts` and its questions all work as
they do in any terminal, in the terminal. Put `IdentityFile`, `ProxyJump` and the like in `~/.ssh/config`.

- The tab is named `ssh host` until the remote shell sets a title.
- **A split opens a new connection** to the same host, in its home folder. The folder of the pane is a folder of this
  computer and means nothing on the other side, so it is not used.
- A profile's environment variables go to the local `ssh` (to point `SSH_AUTH_SOCK` somewhere, say).
- The shell and folder fields do not apply to an SSH profile.

### No command blocks on the other side

The remote shell has no [shell integration](shell-integration.md), so a remote terminal is a plain terminal: no command
blocks, and no folder for the git item or the project tasks (they are for terminals on this computer). To get blocks on
the other side, copy the integration script for the remote shell (it is in the `shell-integration/` folder of the
[data folder](configuration.md#shell-integration-files)) and `source` it from that shell's startup file.

## WSL

On Windows, set *Starts* to **A WSL distribution**:

```toml
[[profiles]]
name = "Ubuntu"
kind = "wsl"
distribution = "Ubuntu-22.04"    # empty: the default distribution
working_directory = "~/code"     # a folder inside the distribution; empty: its home
```

TermZip runs `wsl.exe [-d distribution] [--cd folder]`. The distribution field suggests the installed ones (from
`wsl.exe -l -q`, Docker Desktop's own distributions left out). The option is offered on Windows only, and a profile
that is already WSL stays editable elsewhere (a config you brought from Windows); starting it says that WSL profiles
work on Windows only.

A WSL profile never inherits the built-in profile's folder, which is a folder of Windows.

## What is checked

The host, the user and the distribution end up as arguments of another program, so they are checked twice, in the
settings and again where the command is built:

- letters, digits and `. _ : -` for a host (so an IPv6 address works), `. _ -` for a user and a distribution;
- **never starting with `-`**, which `ssh` would read as an option (`-oProxyCommand=…` is refused);
- at most 253 characters for a host and 64 for the others, and a port from 1 to 65535.

A profile that fails these is explained when you save it. One edited by hand is cleaned when read, and an SSH profile
left without a usable host is a message when it is started, never a shell on this computer by mistake.

The host suggestions read only the `Host` names of `~/.ssh/config` and of the files it includes (two levels, inside
`~/.ssh` only). Patterns (`*`, `?`, `!`) are skipped, and keys, users, ports and proxy commands are never read. The file
is read as untrusted input: at most 256 KiB, 32 files and 200 names.

## Limits

- No blocks, git item or project tasks for a remote terminal.
- No remote folder tracking: a split starts at the host's home, not where you were.
- WSL: tested by unit tests; interactive validation on Windows is still to do (see the
  [compatibility matrix](compatibility-matrix.md)).
