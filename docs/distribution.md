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

| Channel | How to install | Where it lives |
| --- | --- | --- |
| Homebrew (macOS) | `brew install --cask luispeixoto/termzip/termzip` | [`LuisPeixoto/homebrew-termzip`](https://github.com/LuisPeixoto/homebrew-termzip), one cask for both Macs |
| Scoop (Windows) | `scoop bucket add termzip https://github.com/LuisPeixoto/scoop-termzip` then `scoop install termzip` | [`LuisPeixoto/scoop-termzip`](https://github.com/LuisPeixoto/scoop-termzip) |
| Winget (Windows) | `winget install LuisPeixoto.TermZip`, once the manifests are accepted | `packaging/winget/`, submitted by hand |

`tools/update-packages.sh vX.Y.Z` writes the three manifests from the release's own `SHA256SUMS` and, with `--push`,
pushes the tap and the bucket. Run it after a release is published. `tools/test-update-packages.sh` tests it without the
network and runs in CI.

What was verified: the cask passes `brew style` and `brew audit --online --strict` and installs and uninstalls with
`brew install --cask --appdir=<folder>`. The Scoop manifest is valid JSON and uses the NSIS installer silently, but it
has **not been run on a Windows computer** yet. The Winget manifests are not submitted: that is a pull request to
`microsoft/winget-pkgs`, which the maintainer opens by hand (copy `packaging/winget/manifests` there). Flathub is not
started.

### Signing credentials

Releases are unsigned until these repository secrets exist; `tools/prepare-signing.sh` turns signing on by itself when
they do:

- **macOS** (needs an Apple Developer Program membership): `APPLE_CERTIFICATE` (a Developer ID Application certificate
  exported as `.p12`, base64), `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, and for notarization `APPLE_ID`,
  `APPLE_PASSWORD` (an app-specific password) and `APPLE_TEAM_ID`.
- **Windows** (Azure Trusted Signing): `AZURE_TENANT_ID`, `AZURE_CLIENT_ID`, `AZURE_CLIENT_SECRET` (a service principal) and
  `TRUSTED_SIGNING_ENDPOINT`, `TRUSTED_SIGNING_ACCOUNT`, `TRUSTED_SIGNING_PROFILE`.
- **Updates** are signed separately, with the key already stored as `TAURI_SIGNING_PRIVATE_KEY` and
  `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (see [updates](updates.md)).

## Versioning

The workspace version, `apps/desktop/package.json`, and `apps/desktop/src-tauri/tauri.conf.json` must match, and the
release workflow checks them against the tag. Use SemVer. Before 1.0, minor releases may include user-visible behavior changes; `CHANGELOG.md` records them.