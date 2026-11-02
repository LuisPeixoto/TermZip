# Performance and limits

What TermZip does so that a burst of output, a long session or many panes stay responsive, and the bounds that
protect memory, followed by the [budgets](#budgets) the project holds itself to.

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

A flood does not wait for one read at a time any more: the reader merges what is already queued, up to 64 KiB and never
past the flow-control room, before it sends. Measured in a visible window, that took `seq 1 1000000` from about 79,000
to about 200,000 lines per second and the memory after it from 1,048 to 611 MB. Larger batches (256 KiB) gave no more.

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

## Budgets

`docs/performance-budgets.toml` holds one limit per metric. They are what TermZip should meet on the **reference
hardware**: an Intel x86_64 Mac with 32 GB, macOS 26, a release build and a visible window. Medians of five runs there:

| Metric | Budget | Measured |
| --- | --- | --- |
| OSC marker parser | at least 1,000 MB/s | 1,300 MB/s |
| Parser and block tracker | at least 600 MB/s | 738 MB/s |
| Importing one theme file | at most 200 µs | 143 µs |
| Reading the task files of a project | at most 10 µs | 6.7 µs |
| Reading `git status` output | at most 1 µs | 0.3 µs |
| Rendering one snippet | at most 2 µs | 1.1 µs |
| A real PTY to its reader (Unix) | at least 10 MB/s | 13.7 MB/s |
| Launch to the first prompt | at most 700 ms | 578 ms |
| Flood of `seq` in the window | at least 180,000 lines/s | 200,000 lines/s |
| Idle CPU after the prompt | at most 5 % | 2.5 % |
| Memory after the prompt | at most 700 MB | 611 MB |

Memory counts the app and its web view together.

**How they are checked.** `termzip-bench run` measures the headless parts on any system, and `tools/measure-app.sh`
measures the app (five runs, medians, on macOS and Linux). The `Smoke` workflow runs both on a macOS, a Linux and a
Windows runner for every change to `main`, and fails only when a measurement is far past its budget: a shared runner is
slower and noisier than the reference computer, so the headless numbers get a slack of 4 times and the app numbers on the
macOS runner a slack of 20 (it starts the app cold: 6 to 8 s to the first prompt). The check is there to catch a
regression of an order of magnitude, not a slow afternoon.

**Other systems, as the runners saw them** (one run each, so only the order of magnitude means anything):

| | Launch to prompt | Flood |
| --- | --- | --- |
| macOS, Apple silicon runner | 5.8 s | 138,000 lines/s |
| Linux runner, under Xvfb | 33 s | 212,000 lines/s |
| Windows runner, PowerShell | 9.2 s | 4,600 lines/s |

Windows is the slowest by far in the flood: the console layer (ConPTY) and PowerShell's own output are what a 100,000 line
flood waits for. It completes and the blocks are right, but it is a known limit that no budget covers yet.
