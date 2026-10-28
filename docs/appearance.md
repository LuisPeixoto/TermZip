# Appearance

Besides [themes](themes.md), **Preferences → Terminal** and **Appearance** control how the terminal is drawn and how
the window looks. Everything here applies to the open terminals as soon as you save (the dialog shows the theme and the
opacity while you choose, and **Cancel** puts the saved look back).

## The terminal

| Setting | Range | Default | `config.toml` key |
|---|---|---|---|
| Font size | 10 – 28 px | 14 | `font_size` |
| Font family | any installed monospace family | JetBrains Mono, SF Mono, Cascadia Code, Menlo, Consolas… | `font_family` |
| Line height | 1.0 – 2.0 (a multiple of the font size) | 1.3 | `line_height` |
| Letter spacing | 0 – 4 px | 0 | `letter_spacing` |
| Cursor | block, bar or underline | block | `cursor_style` |
| Blink the cursor | on / off | on | `cursor_blink` |

A sample at the top of the **Terminal** section uses the font, spacing and cursor you are choosing and stays in view
while you scroll the controls. A pane that does not have the keyboard keeps an **outlined** cursor, so it is clear where
typing goes. Changing the line height or the letter spacing changes how many rows and columns fit, and the shell is
resized to match.

## The window

| Setting | Range | Default | `config.toml` key |
|---|---|---|---|
| Window opacity | 50 – 100 % | 100 % | `background_opacity` |
| Blur what shows through | on / off | on | `background_blur` |

At **100 %** the window is opaque, exactly as in 0.2. Below that the background and the panels around the terminal let
the desktop show through, while dialogs, menus and the text itself stay opaque so everything remains readable. The
terminal is drawn on a clear background, so the opacity is applied once.

| System | What you get |
|---|---|
| macOS | The system's vibrancy blur behind the window (`UnderWindowBackground`, following whether the window is active). |
| Windows | Acrylic blur behind the window. |
| Linux | Transparency only, and only if your window manager composites windows; there is no blur. The toggle is hidden and a note says so. |

The blur is applied only while the window is translucent. The window is always created transparent (a window cannot
change that later) and paints itself opaque unless you lower the opacity.

**Note for packagers.** On macOS translucency needs Tauri's private API (`macOSPrivateApi`), which makes the app
ineligible for the Mac App Store. See [SECURITY.md](../SECURITY.md).

## Per-profile looks

A [profile](profiles-and-restore.md#profiles) can set its own **terminal theme**, **font size**, **font family** and
**cursor**. Whatever it leaves empty is the app's setting. A pane draws its terminal with its profile's look, while
the window around it (toolbar, tabs, the blocks rail) keeps the app's theme, so moving between panes never repaints the
window.

- A profile with a theme of its own paints its own background behind the terminal, as translucent as the window.
- A profile whose theme no longer exists simply follows the app.
- The theme of a profile may be of the other appearance (a light terminal in a dark window).
- The blocks rail colors the output with the app's theme, not the focused pane's.
- Changes to a profile's look reach the open terminals of that profile when you save.

## Limits

- Opacity below 50 % is not offered: text becomes hard to read over most backgrounds.
- Windows acrylic and Linux transparency have not been checked on real systems yet (see the
  [compatibility matrix](compatibility-matrix.md)).
- A translucent window turns on the terminal's transparency support, which its renderer documents as costing some
  performance; at 100 % it stays off.
