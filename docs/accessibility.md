# Accessibility

What TermZip does so that more people can use it, how it is checked, and what is not done yet.

## Screen readers

**Preferences → Terminal → Screen reader support** turns on xterm.js's screen reader mode for every terminal. The
terminal then keeps a parallel text version of the screen that assistive technology can read, and announces new output
politely. It is off by default because it costs some speed on a flood of output. It applies to the running terminals as
soon as you save (`screen_reader` in `config.toml`).

The rest of the interface uses native elements (buttons, inputs, checkboxes, dialogs) with names, so a reader announces
them. The tabs are a group of buttons with the active one marked as current; the command palette and Preferences are
dialogs that take the keyboard while they are open.

## Reduced motion

When the system asks for reduced motion, TermZip stops the cursor from blinking and turns off the interface's
transitions and animations. This follows the system setting live (macOS: Accessibility → Display → Reduce motion;
Windows: Accessibility → Visual effects → Animation effects; GNOME: Accessibility → Reduce animation).

## Contrast

The colors of the interface are derived from the theme (see [themes](themes.md)). Text and the colors that carry meaning
(muted and faint text, the accent, and the success, warning and error colors) are held to **4.5:1** against every
surface they are drawn on, in all twelve built-in themes in dark and light, and for imported themes too: a color that
would fail is moved until it passes, so an imported theme can never make the interface unreadable. Text on an accent
button uses a color chosen for it. The terminal's own colors are the theme's: ANSI colors are up to the theme and the
program.

## Keyboard

Everything can be done from the keyboard: every action has a shortcut that can be changed in Preferences, the command
palette reaches the rest, and `Esc` closes the palette and Preferences. Application shortcuts never take a key a shell
needs (see [keyboard](keyboard.md)).

## How it is checked

- `tests/contrast.spec.ts` computes the ratios of every token on every surface for all built-in themes in both
  appearances, and for hostile theme input.
- `tests/a11y.spec.ts` runs [axe-core](https://github.com/dequelabs/axe-core) on the command palette, every section of
  Preferences, the status bar, the tab bar and the toolbar. The violations it found (a radio group made of list items, a
  tab list with buttons outside of it) were fixed, not silenced.
- `tests/motion.spec.ts` covers reduced motion.

## Not done yet

- A person using VoiceOver, NVDA or Orca has not walked through the application. The automated checks find missing names
  and contrast; they do not tell whether the experience is good. This is on the [validation checklist](validation-checklist.md).
- Whether focus returns to the right place after every dialog closes, and whether the command-block rail should be a live region, have not been reviewed with a screen reader.
- Zooming the whole window is the web view's: the font size setting (10 to 28) is the supported way to enlarge text.

Problems with accessibility are welcome as issues, with the system and the assistive technology you use.
