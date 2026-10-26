# Configuration

TermZip stores optional TOML settings in the operating system's standard per-user configuration directory:

- macOS: `~/Library/Application Support/termzip/config.toml`
- Linux: `$XDG_CONFIG_HOME/termzip/config.toml` or `~/.config/termzip/config.toml`
- Windows: `%APPDATA%\termzip\config.toml`

Missing files use built-in defaults. Theme, font, and scrollback can also be changed in **Preferences** (⌘,); TermZip
saves them automatically. The file contains preferences only — no shell history or terminal output.

```toml
theme = "dark"            # "dark" | "light"
font_size = 14.0          # clamped to 10..28
# font_family = "JetBrains Mono"
scrollback_lines = 10000  # clamped to 1_000..200_000
# shell = "/bin/zsh"      # default: $SHELL / %COMSPEC%
# working_directory = "/path/to/project"
```

Unknown or malformed values fall back to defaults; out-of-range values are clamped. `shell` and `working_directory`
are applied when a session starts — restart the shell (⌘R) after changing them.

## Shell integration files

TermZip writes its integration scripts next to the config file under `shell-integration/`. For zsh it also stages a
`ZDOTDIR` shim that sources your own `~/.zshenv`/`~/.zshrc` first. These files are regenerated when their content
changes; you do not need to edit them.