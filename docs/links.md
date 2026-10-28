# Links

TermZip underlines URLs, hyperlinks and file paths in the terminal. **⌘+click** (Ctrl+click on Windows and Linux)
opens one. A plain click only selects, so text that contains a link can still be copied.

## What is recognized

| Kind | Examples |
|---|---|
| URLs | `https://example.com/docs`, `http://localhost:3000` |
| Hyperlinks (OSC 8) | what `ls --hyperlink` and many command-line tools print; http and https only |
| Paths | `/etc/hosts`, `~/notes.txt`, `./run.sh`, `../up/file`, `src/lib/layout.ts`, `C:\Users\me\a.txt` |
| Paths with a position | `src/lib/layout.ts:42:7`, `main.rs:12:3` (the way compilers print them) |

Ordinary text is left alone: `and/or`, `TCP/IP`, `1/2` and `10:45:12` are not paths, and a path glued to other text
(`https://host/a/b` is one URL, not `//host/a/b`) is not cut out of it.

## What opens, and what does not

Terminal output is untrusted: any program can print a link. The click goes to the backend, which decides before
anything reaches the operating system.

- **URLs** open in your default browser. Only `http`, `https` and `mailto` are accepted; `javascript:`, `data:`,
  `file:`, `vbscript:` and anything else is refused.
- **Files and folders** open in their default application, but only if they **exist**. A relative path is resolved
  against the folder of the pane the text was printed in, `~/` against your home folder, and `..` is resolved before
  the check. A `:line:column` suffix is understood and ignored: the file opens, it does not jump to the line.
- **Programs are never started by a click.** Scripts, installers and applications (`.command`, `.sh`, `.exe`, `.bat`,
  `.ps1`, `.msi`, `.app`, `.desktop`, `.AppImage`, `.dmg`, `.jar` and the like) only have the **folder that holds them**
  opened, so you can see them but the click cannot run them. A symlink to a program is treated as the program.
- A refusal shows as a message, for example `“/tmp/missing.txt” does not exist.`

## Limits

- A path that contains spaces is not recognized as one piece, and a path split over two terminal rows is not joined.
- A path printed by a program on another machine (SSH) refers to that machine, so it will not exist here.
- Opening a file does not jump to the line in an editor.
