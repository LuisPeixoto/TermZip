# Snippets

A snippet is a command you want to type again, with the values that change filled in each time. You keep them in
**Preferences → Snippets** and find them in the [command palette](keyboard.md#the-command-palette).

> **A snippet is typed, never run.** The text appears at the prompt, as if you had pasted it, and waits for you to read
> it, change it and press Enter. TermZip never presses Enter for you.

## Using one

Press **⇧⌘S** (Ctrl+Shift+S on Windows and Linux): the palette opens showing only your snippets. Type a few letters,
move with the arrows and press Enter. Snippets are also in the full palette (⇧⌘P), as **Snippet: name**.

- A snippet **without values** is typed at once.
- A snippet **with values** opens a small form with each value's field, already filled with its default. **Enter** in a
  field types the snippet, **Esc** goes back to the list. A value with no default cannot be left empty.

The text goes to the terminal that had the keyboard, and needs a running shell.

## Writing one

In **Preferences → Snippets**, choose **Add a snippet**:

| Field | What it is |
|---|---|
| Name | What the palette lists, up to 64 characters. |
| Command | One line, up to 4 KiB. Write `{{name}}` where a value goes, or `{{name:default}}` to start it filled in. |
| Description | Optional, a line about what it does. |
| Folder | Optional. The snippet is offered only while the terminal is in this folder or below (`~` is your home folder). |

While you type the command, the form shows the values it asks for, or the sentence that says why it cannot be saved.

```text
git checkout {{branch:main}}
git log --oneline --graph -n {{count:15}}
docker run --rm -it {{image}} {{command:sh}}
```

- A value name is lowercase letters, digits and `_`, not starting with a digit, up to 32 characters.
- The same name used twice is asked once (with the default of its first use).
- A snippet asks for at most 16 values, and a value is at most 1 KiB.
- To write two braces that are **not** a value (a Go template, a Helm chart), put a backslash before them:
  `docker inspect --format '\{{.State.Status}}' {{name}}`.
- Values are put in as typed. TermZip does not quote them for the shell, so what you type in a field is what the line
  contains; you see the line before it runs.

### In `config.toml`

```toml
[[snippets]]
id = "checkout"                      # derived from the name when missing
name = "Check out a branch"
command = "git checkout {{branch:main}}"
description = "Switch to a branch"   # optional
folder = "~/work/app"                # optional
```

## Why nothing in a snippet can run by itself

Whatever the command and the values contain, the text is cleaned before it is typed: **every control character is
removed** (a line break, a Tab, an escape sequence) and so are the characters that hide or reorder text. A value that
contains a newline cannot press Enter, and a value cannot start an escape sequence. The result is cut at 4 KiB.

This is the same cleaning that [plugins](plugins.md) go through, and it is done in Rust, where the snippet is rendered,
not in the page. ([Project tasks](project-tasks.md) are protected differently: their names can only be plain characters.)

## Limits

| What | Limit |
|---|---|
| Snippets | 128 |
| Name | 64 characters |
| Command | 4 KiB, one line, no control characters |
| Description | 200 characters |
| Values in one snippet | 16 |
| One value | 1 KiB |

Saving explains the first problem in a sentence. A `config.toml` edited by hand is cleaned silently when it is read: a
snippet that cannot be used (an empty command, a `{{` that never closes) is dropped.

## Troubleshooting

See [a snippet is missing from the palette](troubleshooting.md#a-snippet-is-missing-from-the-palette).
