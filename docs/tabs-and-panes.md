# Tabs and split panes

One window can hold several shells. Each **tab** has its own layout of **panes**, and every pane is a real shell in its
own PTY with its own command blocks.

## Using them

These are the defaults; every shortcut can be changed in **Preferences → Keyboard** (see [keyboard](keyboard.md)), and
the command palette (⇧⌘P) lists every action and tab.

| Action | macOS | Windows and Linux |
|---|---|---|
| New tab | ⌘T, or the **+** button | Ctrl+Shift+T |
| Close the active pane (the last pane closes the tab) | ⌘W | Ctrl+Shift+W |
| Split to the right | ⌘D | Ctrl+Shift+D |
| Split downward | ⇧⌘D | Ctrl+Shift+E |
| Move between panes | ⌥⌘ + arrow | Ctrl+Alt + arrow |
| Next / previous tab | ⇧⌘] / ⇧⌘[ | Ctrl+Tab / Ctrl+Shift+Tab |
| Go to tab 1 to 9 | ⌘1 … ⌘9 | Ctrl+Shift+1 … 9 |

- **Tabs.** Click a tab to switch, the **×** to close it, double-click to rename it (an empty name goes back to
  following the shell), and drag it to reorder. A tab shows its name, else the title the shell sets (`vim notes.md`),
  else the name of its folder. An amber dot means the shell in it has exited.
- **A new tab or split starts where you are.** A plain new tab and a split begin in the folder of the pane you were in,
  with the same profile. Choose a profile from the caret next to **+** to start in that profile's own folder instead
  (see [profiles and restore](profiles-and-restore.md)).
- **Panes.** Drag the line between two panes to resize them (each keeps at least a tenth of the space). The pane with
  the keyboard has a thin accent ring, and the status bar and the blocks rail always show that pane. Click a pane or
  use the arrow shortcut to move the keyboard. Closing a pane gives its space to its neighbor and the keyboard to the
  pane before it.
- **Closing the last tab** opens a fresh one, so the window is never empty. The red window button closes the window.

## How it works inside

- Every tab stays mounted, so its terminal keeps its buffer and its shell keeps running while you look at another tab.
  A terminal that is not shown never resizes its PTY.
- A layout is a small tree (`layout.ts`: split, close, resize, neighbors by geometry) and the panes are drawn from the
  rectangles of that tree as a flat list keyed by id. A pane is therefore never moved inside the page when you split
  another one, which is what keeps its shell alive: moving a terminal in the page would recreate it.
- Application shortcuts are handled before the shell sees the key, and they never use plain Ctrl keys on Windows and
  Linux (see [troubleshooting](troubleshooting.md#the-shortcuts-do-nothing-on-windows-or-linux)).
- On macOS the default menu's **Close Window** item is removed, because a menu accelerator is handled by the system
  before the page sees the key and ⌘W would close the whole window instead of a pane.

## Limits

- At most **32 tabs**, **16 panes per tab** and **8 levels** of splits. Past a limit the shortcut shows a message and
  nothing changes. The limits are also what a saved layout may hold.
- Closing a tab or a pane ends its shell without asking, even if a command is running in it.
- Panes cannot be moved to another tab or dragged to a new position, and there is no pane zoom yet.
- The tab strip shrinks tabs to a minimum width; it does not scroll yet, so with a very large number of tabs the last
  ones run out of room.
