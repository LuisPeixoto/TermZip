# Configuration

TermZip reads optional TOML settings from the operating system's standard per-user configuration directory:

- macOS: `~/Library/Application Support/termzip/config.toml`
- Linux: `$XDG_CONFIG_HOME/termzip/config.toml` or `~/.config/termzip/config.toml`
- Windows: `%APPDATA%\termzip\config.toml`

Missing files use built-in defaults. The theme and font size can be changed in **Preferences** and are saved by TermZip.
The file contains preferences and paths, not shell history or terminal output.

```toml
theme = "dark"
font_size = 15.0
scrollback_lines = 10000
# shell = "/bin/zsh"
# working_directory = "/path/to/a/project"
```

`shell` and `working_directory` are optional. If omitted, TermZip uses the operating system's default shell and the
directory from which TermZip was launched. A shell path is launched as a child process with the current user's normal
permissions.
