# Compatibility matrix

This matrix records what has actually been exercised, not only what compiles. Update it when a platform is tested on
real hardware.

| Area | macOS | Windows | Linux |
|---|---|---|---|
| Rust workspace tests | ✅ CI + local | ✅ CI | ✅ CI |
| Tauri desktop compiles | ✅ local + CI | ✅ CI | ✅ CI |
| Svelte check, Vitest, Vite build | ✅ local + CI | ✅ CI | ✅ CI |
| PTY/ConPTY smoke test | ✅ CI | ✅ CI (`cmd.exe /C echo`) | ✅ CI |
| zsh integration (PTY test) | ✅ local + CI | n/a | ✅ CI |
| bash integration (PTY test) | ✅ local + CI | n/a | ✅ CI |
| Session stream tests (`cargo test -p termzip-desktop`) | ✅ local + CI | n/a (Unix shells) | ✅ CI |
| Manual interactive validation | ✅ typing, blocks, themes, shortcuts, restart, preferences | ⏳ pending | ⏳ pending |
| vim / top / tmux / SSH | ⏳ pending | ⏳ pending | ⏳ pending |
| Packaged installer | ✅ debug app build (no bundle) | ⏳ release workflow | ⏳ release workflow |

Legend: ✅ verified, ⏳ pending, n/a not applicable.

The release workflow builds signed/unsigned bundles depending on repository secrets. A successful CI build is not a
substitute for manually opening the packaged app on each platform.

## Reproduce the macOS checks

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo test -p termzip-desktop
cd apps/desktop
npm run check && npm run test && npm run build
npm run tauri build -- --debug --no-bundle
```