# Contributing to TermZip

Thank you for helping make TermZip better. Bug reports, terminal-compatibility findings, documentation, tests, and
focused features are welcome. Please follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## Before you start

- Check the [roadmap](docs/roadmap.md) and existing issues.
- Open an issue before starting a large feature or changing a cross-platform interface.
- Keep pull requests focused and explain user-facing trade-offs, especially terminal compatibility and security.

## Development setup

Install stable Rust with `rustfmt` and `clippy`, plus Node.js 20 or newer. Linux developers also need the WebKitGTK
development packages (`libwebkit2gtk-4.1-dev`, `libappindicator3-dev`, `librsvg2-dev`).

```bash
git clone https://github.com/LuisPeixoto/TermZip.git
cd TermZip/apps/desktop
npm install
npm run tauri dev
```

The app icon is generated from `assets/icon.png` with `npm run icon` (only needed when the icon changes).

Before opening a pull request, run:

```bash
cargo fmt --all --check
cargo clippy --workspace --exclude termzip-desktop --all-targets -- -D warnings
cargo clippy -p termzip-desktop --all-targets -- -D warnings
cargo test --workspace --exclude termzip-desktop
cd apps/desktop && npm run check && npm run test && npm run build
```

## Branches and commits

Create a short-lived branch from `main`. Use the prefixes `feat/…`, `fix/…`, `docs/…`, `perf/…`, `refactor/…`,
`test/…`, `ci/…`, and `chore/…`.

Commits follow [Conventional Commits](https://www.conventionalcommits.org):

```text
feat(terminal): add bracketed paste support
fix(pty): resize the child process with the window
docs: explain interactive terminal mode
```

Keep subjects concise and imperative. Explain why in the body when the reason is not obvious. Do not add co-author
trailers unless every named contributor explicitly requests them.

## Pull requests

- Explain the change and link its issue or roadmap phase.
- Include the commands and operating systems used to test it.
- Add tests for parser, settings, and process behavior.
- Update user documentation and `CHANGELOG.md` for user-visible changes.
- For visual changes, attach dark and light screenshots when available.
- Do not claim compatibility or performance that has not been measured.

## Terminal-specific expectations

- Keep PTY reads off the UI thread and bound cross-thread buffering.
- Treat shell input, environment variables, OSC, and clipboard sequences as untrusted data.
- Do not log terminal contents, commands, environment values, or child-process output.
- Keep marker parsing tolerant: split sequences across reads, BEL and ST terminators, unknown parameters.
- Integration scripts must never alter the user's prompt, aliases, or network access.
- Preserve direct terminal input for full-screen and interactive applications.
