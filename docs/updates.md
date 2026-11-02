# Updates

TermZip never updates itself. It looks for a new version when you ask, tells you what is new, and installs it only when
you press a button.

## Checking and installing

**Preferences → General → Updates**

1. **Check for updates** asks GitHub for the release feed (`latest.json`). Nothing else is sent: no identifier, no
   version history. The request is made by the Rust core, never by the web view.
2. If there is a newer version, the dialog shows its number, its date and its notes as plain text.
3. **Install and restart** downloads the update file, verifies its signature, installs it and restarts TermZip.

An optional setting, *Look for an update at startup* (`check_updates_on_launch`, **off** by default), does step 1 once
when the app starts and shows a short message if there is an update. It never installs anything.

## What protects you

Every release's update files are signed with a private key that is held by the maintainer and by the release workflow. The
matching public key is built into TermZip (`plugins.updater.pubkey` in `tauri.conf.json`). Before it installs anything, the
updater checks the file against that key. A file that does not match is refused, with this message, and nothing on your
computer changes:

> The update did not pass its signature check, so it was not installed. Nothing on your computer was changed.

This protects the *update*, wherever it is downloaded from. It does not protect the first download: check the `SHA256SUMS`
of a release, or install through a package manager, which does it for you ([distribution](distribution.md)).

Tested end to end on a throwaway copy of the application: a tampered update file was refused and the installed program was
unchanged; the genuine file installed and the app restarted as the new version.

## If something goes wrong

- *No update found, but there is a newer release.* The feed is published with the release. A release that is still a draft
  has no feed; try again after it is published.
- *A network error.* Nothing is installed. Try again, or download the release by hand.
- *The signature failed.* Do not install by other means before you know why: open an issue, privately if you suspect
  tampering (see [SECURITY](../SECURITY.md)).
- Downgrades are not offered. To go back, install an older release by hand.

## For the maintainer

- `release.yml` builds the update files with `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (repository
  secrets), names the two macOS archives per architecture, and the `checksums` job writes `latest.json` with
  `tools/make-latest-json.py` from the release's files and their `.sig`. It refuses to write a feed with a platform missing.
- The key pair was made with `tauri signer generate`. Keep the private key and its password in a password manager.
  **If the key is lost**, existing installs cannot update to a release signed with a new key: they need a manual
  download. If it leaks, treat it like any leaked signing key: publish a release with a new key and say so.
- For local tests, a debug build reads `TERMZIP_UPDATE_ENDPOINT` (a URL of a `latest.json`). A release build ignores it.
