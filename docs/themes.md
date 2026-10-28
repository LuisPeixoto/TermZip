# Themes

A theme sets the colors of the **whole window**: the terminal, the colored output in the command blocks, and the
interface around them (toolbar, tabs, panels, buttons), which is derived from the terminal colors. TermZip ships a
gallery of themes, imports the ones you already have from other terminals, and exports any theme as a small file.

## Choosing a theme

Open **Preferences → Appearance**. The cards show a few lines of a terminal in each theme's own colors, grouped as
dark and light themes.

- **Each appearance remembers its own theme.** The *dark* slot and the *light* slot each hold one theme, and the sun
  and moon button in the toolbar switches between the two (it is also in the command palette). Choosing a card sets
  the slot of that theme's appearance and switches to it.
- **You see it before you save it.** The window repaints as you pick, and **Cancel** puts the saved theme back.
- **A profile can have a theme of its own** for its terminals (see [appearance](appearance.md#per-profile-looks)); the
  window around them keeps the app's theme.
- If the theme you chose no longer exists (a custom theme you deleted), or belongs to the other appearance, TermZip
  falls back to *TermZip Dark* or *TermZip Light*, so the window is never left without colors.

## The gallery

| Theme | Appearance | By | License | Source |
|---|---|---|---|---|
| TermZip Dark | dark | Luis Peixoto | MIT | [TermZip](https://github.com/LuisPeixoto/TermZip) |
| TermZip Light | light | Luis Peixoto | MIT | [TermZip](https://github.com/LuisPeixoto/TermZip) |
| Catppuccin Mocha | dark | The Catppuccin Org | MIT | [catppuccin/catppuccin](https://github.com/catppuccin/catppuccin) |
| Catppuccin Latte | light | The Catppuccin Org | MIT | [catppuccin/catppuccin](https://github.com/catppuccin/catppuccin) |
| Dracula | dark | Zeno Rocha and the Dracula Theme contributors | MIT | [draculatheme.com](https://draculatheme.com) |
| Nord | dark | Sven Greb (Arctic Ice Studio) | MIT | [nordtheme.com](https://www.nordtheme.com) |
| Gruvbox Dark | dark | Pavel Pertsev (morhetz) | MIT | [morhetz/gruvbox](https://github.com/morhetz/gruvbox) |
| Gruvbox Light | light | Pavel Pertsev (morhetz) | MIT | [morhetz/gruvbox](https://github.com/morhetz/gruvbox) |
| Tokyo Night | dark | enkia | MIT | [enkia/tokyo-night-vscode-theme](https://github.com/enkia/tokyo-night-vscode-theme) |
| One Dark | dark | GitHub (Atom) | MIT | [atom/one-dark-syntax](https://github.com/atom/atom/tree/master/packages/one-dark-syntax) |
| Rosé Pine | dark | The Rosé Pine contributors | MIT | [rosepinetheme.com](https://rosepinetheme.com) |
| Rosé Pine Dawn | light | The Rosé Pine contributors | MIT | [rosepinetheme.com](https://rosepinetheme.com) |

The palettes are their authors'; TermZip does not adjust them. Each is stored with its author, license and source in
`crates/termzip-core/themes/`.

### What a theme must pass to be listed

Every built-in theme is checked by the tests against its own background, and one that fails is **left out instead of
being quietly corrected**:

| Check | Needs | Why |
|---|---|---|
| Text | 4.5 : 1 | WCAG AA for body text. |
| Cursor | 3 : 1 | WCAG for interface components. |
| Colored output (red, green, yellow, blue, magenta, cyan, normal and bright) | 2 : 1 | Colored text must stay distinguishable. The bar is lower than for text because light themes cannot reach 3 : 1 with a yellow that still looks yellow. |
| Bright black | 1.5 : 1 | Shells draw suggestions and comments with it: it may be dim, never invisible. |
| Appearance | matches the background | A "dark" theme must have a dark background. |

Solarized (Dark and Light) is **not** in the gallery for that reason: its bright black is the background color, and the
light variant's text is 4.1 : 1. You can still [import](#importing-a-theme) any Solarized file. An imported theme is
never refused for failing these checks.

## Importing a theme

**Preferences → Appearance → Import…** opens a file dialog (an import converts the file and keeps the result, it never
changes the original). The format is recognized from the **content**, not the extension:

| Format | Files | What is read |
|---|---|---|
| TermZip | `.toml` with a `[colors]` table and an `ansi` list | Everything, including author, license and interface overrides. |
| iTerm2 | `.itermcolors` (XML or binary plist) | Background, foreground, cursor, cursor text, selection and the 16 `Ansi N Color` entries, from the float components. |
| Windows Terminal | `.json`: one scheme, a list of schemes, or a whole `settings.json` | `background`, `foreground`, `cursorColor`, `selectionBackground` and the 16 named colors (`purple` is magenta). From a file with several schemes, the one named like the file, else the first. Comments in `settings.json` must be removed first. |
| Ghostty | `key = value` text, often without an extension | `background`, `foreground`, `cursor-color`, `cursor-text`, `selection-background` and `palette = N=#hex`. |
| Kitty | `key value` text (`.conf`) | `background`, `foreground`, `cursor`, `cursor_text_color`, `selection_background` and `color0` … `color15`. |
| Alacritty | `.toml` with `[colors.primary]`, `[colors.normal]`, … | `primary`, `cursor`, `selection`, `normal` and `bright` (hex as `#rrggbb` or the old `0xrrggbb`). The old YAML format is not read. |

- The theme is **named** after the file (`catppuccin-mocha.conf` becomes “catppuccin mocha”) unless the format has a
  name of its own. Its **appearance** (dark or light) is inferred from the background.
- Whatever a file leaves out (the ANSI colors, the cursor) comes from the TermZip theme of the same appearance, so an
  imported theme is always complete. Words where a color could be (`CellForeground`, `background`, `none`) are ignored.
- Colors outside 0–1 in an iTerm2 file are clamped; a value that is not a number is an error. iTerm2's color-space
  attribute is not applied.
- If the id is taken (by a built-in theme or another import) the new one gets `-2`, `-3`, … so nothing is replaced.
- A file that cannot be read is explained in a sentence (“Line 3: “nonsense” is not a color for color 3…”), and nothing
  is saved.

## Exporting a theme

**Export…** on any card writes it as a TermZip theme file where you choose. Export a built-in theme to start a theme of
your own: edit the colors, then import the file back.

## Custom themes

Imported themes are kept as one file each in the `themes/` folder of the [data folder](configuration.md#the-themes-folder).
You can also put files there by hand. A file that cannot be used is **skipped and listed under the gallery** with the
reason; it never prevents TermZip from starting. **Delete** (a second click confirms) removes the file of a custom
theme; built-in themes cannot be deleted.

## The theme file

```toml
id = "midnight"            # a-z, 0-9 and "-"; made from the name when left out
name = "Midnight"
appearance = "dark"        # "dark" | "light"
author = "Someone"         # optional
license = "MIT"            # optional
source = "https://example.com/midnight"   # optional

[colors]
background = "#0b0c10"
foreground = "#c5c6c7"
cursor = "#66fcf1"
# cursor_text = "#0b0c10"  # the character under a block cursor; the background when left out
selection = "#45a29e66"    # may have an alpha (#rrggbbaa) so text stays readable under it
# accent = "#66fcf1"       # the interface accent; the ANSI blue when left out
ansi = [
  "#0b0c10", "#e06c75", "#98c379", "#e5c07b", "#61afef", "#c678dd", "#56b6c2", "#abb2bf",
  "#5c6370", "#e06c75", "#98c379", "#e5c07b", "#61afef", "#c678dd", "#56b6c2", "#ffffff",
]

# Optional: colors of the interface itself. Whatever is left out is derived (see below).
[chrome]
panel = "#101116"
```

Colors are `#rgb`, `#rrggbb` or `#rrggbbaa`. The 16 `ansi` entries are black, red, green, yellow, blue, magenta, cyan,
white, then the bright version of each. The `[chrome]` table may set `panel`, `panel_2`, `elevated`, `hover`, `line`,
`line_strong`, `muted`, `faint`, `accent_soft`, `ok`, `warn` and `err`.

### How the interface colors are derived

A theme only has to say what it wants different. The panels, lines and text levels are **mixed from the theme's
background and text**, the accent is the ANSI blue (or `accent`), and the success, warning and error colors are the
ANSI green, yellow and red. Every derived text or status color is moved toward the text color just far enough to read at
**4.5 : 1** (secondary text) or **3 : 1** (faint text, accent and status colors) on both the panels and the terminal
background, so even an imported theme gives a readable interface. A value in `[chrome]` is used as it is: that is how
*TermZip Dark* and *TermZip Light* reproduce the original interface exactly.

## Limits

- A theme file is at most **256 KiB**, a name **64 characters**, and at most **128 custom themes** are loaded.
- Only regular files are read from the themes folder and from the import dialog: a directory, a device or a larger file
  is refused before it is opened.
- The block output uses the theme of the window, not the theme of a profile's terminal.
- The search highlights keep fixed colors per appearance.
- Themes cannot be edited inside TermZip yet, only exported, edited and imported again.
