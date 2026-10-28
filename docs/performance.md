# Performance and limits

What TermZip does so that a burst of output, a long session or many panes stay responsive, and the bounds that
protect memory. These are design limits, not benchmark results: documented performance budgets on reference hardware
come with Phase 7.

## A flood of output

A command such as `yes` or `seq 1 2000000` can print faster than a terminal can draw. Without a brake, the output
piles up in the page's message queue while the screen falls further behind.

TermZip keeps a **flow-control** account for every pane:

1. The reader counts the bytes it sends to the terminal.
2. The terminal tells the backend (`ack_session`) what it has finished drawing, in batches of 64 KiB.
3. Past **1 MiB** of output the terminal has not drawn yet, the reader **waits**. It then stops emptying the bounded
   PTY channel, so the shell blocks on its own `write`, and it resumes below **256 KiB**.

So the screen stays close to what the program has done, Ctrl+C stops it, and the queue of output waiting to be drawn
is bounded instead of growing with the size of the output. If the page stops answering for **10 seconds** (a reload, a stuck
tab), the backend gives up on the account instead of freezing the shell for good, and destroying a pane releases a
waiting reader.

Checked in the running app with `seq 1 2000000`: the terminal drew the numbers live, Ctrl+C stopped the count at 667,254,
and the next command answered at once. The TermZip core process (not the web view, which was not measured) held about
90 MB afterwards with an idle processor.

## Command blocks

- A finished command is sent as **one** event, and a directory change as another. The list of blocks is no longer sent
  whole for every command, which used to be up to 200 blocks of 64 KiB each.
- The rail keeps the last **200** blocks of a pane and at most **64 KiB** of output per block, and it draws **only the
  cards in view** (plus a few more each side). Each card parses colored output, so this is what keeps a long session
  cheap; the scrollbar still has the size of the whole list.

## Bounds

| What | Bound |
|---|---|
| Output not yet drawn before the reader pauses | 1 MiB (resumes at 256 KiB) |
| Blocks per pane | 200 |
| Output per block | 64 KiB |
| Captured input per command | 128 KiB |
| Pending bytes in the marker parser | 64 KiB |
| Terminal scrollback | 1,000 to 200,000 lines (a preference) |
| Tabs, panes per tab, nested splits | 32, 16, 8 |
| Saved layout file | 64 KiB |
| Matches highlighted by find | 1,000 |

## Many panes

Every pane is its own shell, PTY and terminal. A pane that is not shown (another tab) keeps its buffer and keeps
receiving output, but it is never resized, and only the active pane's blocks are drawn.
