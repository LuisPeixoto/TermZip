# Plugins

A plugin is a small WebAssembly file that adds something to TermZip: a command in the palette, a label on a block, an
item in the status bar, or a link in the output. Plugins run in a sandbox that has no way out: **they cannot read your
files, use the network, start programs or even tell the time.** All a plugin can do is look at what TermZip hands it
and answer with what it wants shown. If you want to write one, see the [plugin API](plugin-api.md).

## What a plugin can do

| It can | You see it as | Needs |
|---|---|---|
| Add commands to the command palette | An entry named “Plugin name: Command title” (⇧⌘P, Ctrl+Shift+P) | nothing |
| Turn text in the output into links | An underline when you point at it; ⌘+click (Ctrl+click elsewhere) opens it like any other [link](links.md) | nothing |
| Show an item in the status bar | A short text after the size of the terminal | nothing |
| React to every finished command | A label on the block, a status text, or a message | `blocks` |
| …and see what the command printed | (the same, with more to go on) | `output` |
| Show you a message | A toast with the plugin's name | `notify` |
| Type text into the terminal when you run one of its commands | The text appears at the prompt, **never followed by Enter** | `type` |
| Open a web address when you run one of its commands | Your browser | `open` |

Labels, status items, commands and link patterns need no permission because they only show what the plugin returns.
Everything else is listed before you can turn a plugin on.

## Installing a plugin

Open **Preferences → Plugins** (⌘, Ctrl+,). There are two ways in.

- **From a file.** Press **Install from file…** and pick a `.wasm` file. TermZip reads it, checks it, and lists it as
  *From a file: not verified by anyone*. Nobody but you has looked at it, so install only files from people you trust.
- **From the catalog.** Open the **Catalog** tab, press **Load catalog**, and press **Install** on a plugin. It is
  listed as *From the catalog, checked against its signature*, for as long as the file is the one that was checked: if
  it changes on disk afterwards it is listed as not verified. See [the catalog](#the-catalog).

Installing never turns a plugin on. A plugin that was just installed is **Off**.

## Turning a plugin on

Use the switch next to the plugin. TermZip shows a confirmation that says what the plugin asks to do, one sentence per
permission:

- `blocks`: See each command you run: its text, exit code, duration and folder.
- `output`: Also see what each command printed.
- `type`: Type text into the terminal when you run one of its commands. It never presses Enter.
- `notify`: Show you short messages.
- `open`: Open a web address in your browser when you run one of its commands.

A plugin without permissions says so: it can only show labels and a status, add commands and make links. A file that
did not come from the catalog also carries a warning. Press **Turn on** to approve it, or **Cancel**.

Your approval belongs to the **contents** of the file, not to its name: TermZip keeps the SHA-256 of the file you
approved. If the file is replaced or edited afterwards, or you install an update, the plugin is **Off: the file changed
after you turned it on**, and you see its permissions again before it can run. What you approved is exactly what runs.

The switch turns it off again at any time. **Uninstall** (and then **Uninstall for good?**) deletes the file and what
you decided about it.

## What a running plugin sees and does

- A plugin with `blocks` hears about each command **after it finishes**, in every terminal: the command text, the exit
  code, how long it took, and the folder. With `output` it also gets what the command printed, at most 64 KiB, as the
  block keeps it. It never sees anything else on your computer.
- Its answer can label that block (a badge on the block's card, up to four per block, one per plugin), show a message,
  or change its status item. Nothing it says after a finished command can type or open anything.
- When you run one of its palette commands, its answer may also type text or open an address, if it has the permission.
  Typed text goes to the terminal you ran the command from, as a paste, with line breaks and control characters
  removed: it waits at the prompt for you to read, edit and press Enter. An address opens only if it is `http` or
  `https`, through the same checks as a click.
- Text a plugin shows is plain text, cut to its limit (24 characters for a label, 48 for a status item, 200 for a
  message). It is never interpreted as markup.

## The catalog

The catalog is a list of plugins published for TermZip at
`https://github.com/LuisPeixoto/termzip-plugins`. TermZip reads it **only when you press Load catalog or Install**,
never on its own, and not at all if you switch it off (below).

Before it trusts anything in the list, TermZip fetches the list **and its signature** and checks the signature against
a [minisign](https://jedisct1.github.io/minisign/) public key that is built into the app. A list that was changed on its
way, or signed with another key, is thrown away whole. A download is then checked against the size and the SHA-256 the
signed list promises, and the manifest inside the file must carry the same id, version and permissions the list showed
you. Everything uses HTTPS.

Each entry shows the name, version, description, size and the permissions it will ask for. The button says **Install**,
**Update to x.y.z**, or **Installed**. An update is a new file, so it needs your approval again.

To switch the catalog off, clear **Allow loading the plugin catalog** and press **Save** (`plugins_catalog = false` in
[`config.toml`](configuration.md)). With it off TermZip never contacts the catalog; installing from a file keeps
working.

## When something goes wrong

- A plugin that crashes, loops, or answers with something that is not a reply shows a message with the reason and is
  given up on after **three failures in a row**. It then reads *Stopped: it crashed three times in a row. Turn it off
  and on to try again.* A call that works resets the count.
- A plugin that is slow never slows the terminal: each plugin has its own queue of 32 events, and events beyond it are
  dropped for that plugin.
- A file that cannot be used (too big, not WebAssembly, a manifest with a mistake, a file name that does not match its
  id) is listed with a sentence that says why, and everything else keeps working. See
  [troubleshooting](troubleshooting.md#plugins).

## Limits

| What | Limit |
|---|---|
| A plugin file | 4 MiB |
| Installed plugins | 64 |
| Manifest | 16 KiB, 32 commands, 16 link patterns of up to 256 characters |
| Work per call | 50 million units of the interpreter's fuel; a plugin that runs out is stopped for that call |
| Memory | 16 MiB per plugin |
| A reply | 64 KiB, at most 32 actions |
| Output given to a plugin | 64 KiB per block |
| Queue per plugin | 32 events |
| Failures in a row before a plugin is stopped | 3 |
| Catalog | an index of up to 256 KiB listing up to 128 plugins; a download of up to 4 MiB; 30 seconds per request |

## Where plugins live

Installed plugins are in the `plugins/` folder next to `config.toml` (on macOS,
`~/Library/Application Support/termzip/plugins/`): one `<id>.wasm` per plugin and a `state.json` with what you turned
on and the hash you approved. A `state.json` that cannot be read approves nothing, so every plugin comes back **Off**.
`TERMZIP_HOME` moves the folder along with the rest of TermZip's data.

## Plugins to start from

[`examples/plugins`](../examples/plugins) has four, each a template for one kind of plugin: a notifier for long
commands, an exit-status marker with a failure count, issue links, and snippets that type common commands. The
[plugin API](plugin-api.md) explains how to write your own, in Rust or in anything that builds WebAssembly.
