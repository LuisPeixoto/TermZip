# Search and filter

Two ways to find something you saw earlier: a **find box** over the terminal, and a **filter** on the blocks rail.

## Find in the terminal

Press ⌘F (Ctrl+Shift+F on Windows and Linux). A box appears at the top right of the pane.

- Matches are highlighted in the whole scrollback, with the current one in another color, and the box says where you
  are: `3 of 12`.
- **Enter** goes to the next match and **Shift+Enter** to the previous one.
- **Aa** matches case, **.\*** reads the text as a regular expression. A bad expression turns the box red instead of
  failing silently.
- **Escape** closes the box, clears the marks and gives the keyboard back to the shell. ⌘F again brings it back with
  the text selected.

Each pane has its own find box. Search covers what the pane still holds: the **scrollback** setting (1,000 to 200,000
lines) decides how far back that is. Beyond 1,000 highlighted matches the box shows `1000+ matches` and stops marking.

## Filter the blocks

Above the list of blocks there is a text box and an **All / OK / Failed** switch.

- The text is looked for in the command **and** in the output, ignoring case and ignoring colors: searching for `FAIL`
  finds a red **FAIL**, and `31m` finds nothing because that is only the color code.
- **OK** keeps blocks with exit code 0 and **Failed** any other code. A block with no exit code (an unfinished or
  uncaptured one) only shows under **All**.
- The count shows `3 of 12`. When nothing matches there is a link to clear the filter, and **Escape** in the box empties
  it before it leaves it.
- Each pane keeps its own filter, so switching tabs does not carry a filter into a list it does not fit.

## Limits

- The rail holds the last **200** blocks of a pane and at most **64 KiB** of output per block; the filter searches
  what the rail holds, not the whole terminal. Use find for the rest.
- The block filter is plain text, not a regular expression, and it does not mark the matched words inside the cards.
- Find does not search other panes or other tabs.
