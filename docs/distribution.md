# Building and distributing TermZip

## Local builds

```bash
cd apps/desktop
npm ci
npm run tauri build
```

Tauri writes platform bundles under `target/release/bundle/` at the repository root (the desktop shell is part of the
Cargo workspace), or `target/<triple>/release/bundle/` when you build with `--target`:

- macOS: `.app` and `.dmg` (Apple silicon and Intel targets are separate).
- Windows: `.msi` and NSIS `.exe`.
- Linux: `.deb`, `.rpm` and `.AppImage` when the host has the required tooling.

For a build without packaging (useful on constrained machines):

```bash
npm run tauri build -- --debug --no-bundle
```

## Release workflow

Pushing a `v*` tag starts `.github/workflows/release.yml`:

1. Fails early when the tag does not match the workspace, Tauri, and npm versions.
2. Creates a draft GitHub release.
3. Builds bundles for Apple silicon, Intel macOS, Linux and Windows.
4. Uploads the bundles to the draft release.
5. Generates a `SHA256SUMS` manifest (a re-run never hashes the previous manifest).

The draft is intentionally not published automatically. Review it, then publish it from GitHub. Signing and
notarization are not configured yet; when certificates are available, wire them through repository secrets before
publishing.

## Package managers

Homebrew, Scoop, Winget and Flathub distribution is planned for Phase 7. Do not publish package manifests until the
bundle names, checksums and installers have been verified on each platform.

## Versioning

The workspace version, `apps/desktop/package.json`, and `apps/desktop/src-tauri/tauri.conf.json` must match, and the
release workflow checks them against the tag. Use SemVer. Before 1.0, minor releases may include user-visible behavior changes; `CHANGELOG.md` records them.