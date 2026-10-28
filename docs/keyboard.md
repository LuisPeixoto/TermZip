# Keyboard shortcuts and the command palette

## The shortcuts

The shortcuts never take a key your shell expects: on macOS they use **⌘**, which no shell uses, and on Windows and
Linux **Ctrl+Shift**, so plain Ctrl+R (reverse search), Ctrl+B and the other Ctrl keys always reach the shell.

| Action | macOS | Windows and Linux |
|---|---|---|
| Command palette | ⇧⌘P | Ctrl+Shift+P |
| Preferences | ⌘, | Ctrl+, |
| Show or hide the command blocks | ⌘B | Ctrl+Shift+B |
| New tab | ⌘T | Ctrl+Shift+T |
| Close the pane | ⌘W | Ctrl+Shift+W |
| Split to the right | ⌘D | Ctrl+Shift+D |
| Split downward | ⇧⌘D | Ctrl+Shift+E |
| Next tab | ⇧⌘] | Ctrl+Tab |
| Previous tab | ⇧⌘[ | Ctrl+Shift+Tab |
| Focus the pane on the left / right / above / below | ⌥⌘ + arrow | Ctrl+Alt + arrow |
| Go to tab 1 to 9 | ⌘1 … ⌘9 | Ctrl+Shift+1 … 9 |
| Find in the terminal | ⌘F | Ctrl+Shift+F |
| Restart the shell | ⌘R | Ctrl+Shift+R |

Copy, paste and select all in the terminal are **⌘C / ⌘V / ⌘A** on macOS and **Ctrl+Shift+C / V / A** elsewhere; they are
the terminal's, and cannot be given to another action. Ctrl+C still interrupts the running command.

Letters follow the character you type, so a shortcut works on Dvorak, AZERTY and other layouts. Digits, brackets and
punctuation follow the physical key, because Shift changes what they type.

## Changing a shortcut

**Preferences → Keyboard** lists every action with its shortcut.

1. Press the shortcut next to an action (or Tab to it and press Enter): it says **Press keys…**.
2. Press the new keys. **Esc** cancels, **Backspace** (or Delete) removes the shortcut.
3. Press **Save**. **Reset** puts one action back, **Reset all** puts them all back, and the search box filters the
   list.

While you record, no shortcut acts and the keys do not reach the terminal, so you can record any chord.

- **A chord needs Ctrl, Alt or ⌘** (function keys may stand alone). A bare letter, or Shift with a letter, would take
  over typing and is refused.
- **Copy, paste and select all** are refused (see above).
- **A chord a shell uses is accepted with a warning**: Ctrl with a letter (readline: Ctrl+A, Ctrl+K, Ctrl+R…) and Alt
  or Option with a letter (word movement in shells; characters on many macOS keyboards). TermZip would receive those keys
  and the shell no longer would.
- **A chord another action has is taken from it**: the other action is left without a shortcut and a note says which.
  Resetting an action takes its default back the same way. A key never does two things.
- An action may have no shortcut; it is still in the command palette, and its tooltips show no hint.

Only what differs from the defaults is saved, so a default that changes in a later version still reaches everyone who
never touched it.

### In `config.toml`

```toml
[keybindings]
splitRight = "Shift+Cmd+E"
find = ""                   # no shortcut
restart = "Ctrl+Alt+R"
```

A chord is written `Ctrl+Alt+Shift+Cmd+Key` (modifiers in that order; `Option`, `Command` and `Meta` are accepted when
reading, and the file is rewritten in the canonical order). **Cmd** is the ⌘ key, and the Windows key elsewhere.

| Key | Written as |
|---|---|
| Letters and digits | `A` … `Z`, `0` … `9` |
| Function keys | `F1` … `F24` |
| Arrows | `Left`, `Right`, `Up`, `Down` |
| Others | `Tab`, `Enter`, `Space`, `Backspace`, `Delete`, `Insert`, `Home`, `End`, `PageUp`, `PageDown` |
| Punctuation | `Comma`, `Period`, `Slash`, `Semicolon`, `Quote`, `BracketLeft`, `BracketRight`, `Backslash`, `Minus`, `Equal`, `Backquote` |

The action names are `commandPalette`, `preferences`, `blocks`, `newTab`, `closePane`, `splitRight`, `splitDown`,
`nextTab`, `prevTab`, `focusLeft`, `focusRight`, `focusUp`, `focusDown`, `tab1` … `tab9`, `find` and `restart`.

A hand-edited file is read as untrusted input: an entry that is not a well-formed chord (`"D"`, `"Shift+D"`, `"Cmd+,"`)
or names an action that does not exist is ignored, at most 64 entries are read, and when two actions are given the same
chord the first one in the list above keeps it. Saving from Preferences explains a mistake in a sentence instead.

## The command palette

**⇧⌘P** (Ctrl+Shift+P) opens a list of everything TermZip can do. Type a few letters (“splitd” finds *Split downward*,
“thdr” finds *Theme: Dracula*), move with the **arrow keys**, press **Enter** to run, **Esc** to close. The matched
letters are highlighted, and each command shows its current shortcut.

It lists:

- every action above, with the shortcut **you** set;
- **Go to tab N: name** for each open tab (up to nine);
- **New tab: profile** for each profile and the built-in one;
- **Theme: name** for every theme, marking the one in use (it switches to that theme's appearance and saves it);
- the switch between dark and light, and (where the system can blur) the window blur.

With nothing typed, the commands you used last come first. The search matches letters in order, and rewards letters
that start a word or follow each other, so a few letters are enough. The palette is a list box with
`aria-activedescendant`, so screen readers announce the highlighted command.

## Limits

- One shortcut per action.
- Shortcuts are not per profile, and there are no multi-step (chord sequence) shortcuts.
- The command palette does not list the commands of the shell, only TermZip's.
- The recently used commands are remembered while the window is open, not between launches.
