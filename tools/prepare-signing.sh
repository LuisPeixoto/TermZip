#!/usr/bin/env bash
# Decides, from which secrets exist, whether the release build signs the app, and says so. It writes what the build
# needs to the file named by $GITHUB_ENV (a later step reads it), so a secret that is missing never reaches Tauri as an
# empty variable (which it would take as "sign with nothing" and fail), and a release without certificates is still a
# release, with a warning that says it is unsigned.
#
# What it reads (all optional):
#   macOS:    APPLE_CERTIFICATE (a base64 .p12), APPLE_CERTIFICATE_PASSWORD, APPLE_SIGNING_IDENTITY, and, to notarize,
#             APPLE_ID, APPLE_PASSWORD (an app-specific password) and APPLE_TEAM_ID
#   Windows:  AZURE_TENANT_ID, AZURE_CLIENT_ID, AZURE_CLIENT_SECRET (a service principal) and TRUSTED_SIGNING_ENDPOINT,
#             TRUSTED_SIGNING_ACCOUNT, TRUSTED_SIGNING_PROFILE (Azure Trusted Signing)
#
# It prints one line per decision, for the log, and never prints a value.
set -euo pipefail

target="${GITHUB_ENV:-/dev/null}"
system="${RUNNER_OS:-$(uname)}"

export_variable() {
  # Only what has a value; the name is never echoed with its value.
  printf '%s=%s\n' "$1" "${!1}" >> "$target"
}

all_set() {
  for name in "$@"; do
    [ -n "${!name:-}" ] || return 1
  done
}

case "$system" in
  macOS | Darwin)
    if all_set APPLE_CERTIFICATE APPLE_CERTIFICATE_PASSWORD APPLE_SIGNING_IDENTITY; then
      for name in APPLE_CERTIFICATE APPLE_CERTIFICATE_PASSWORD APPLE_SIGNING_IDENTITY; do export_variable "$name"; done
      echo "macOS: the app will be signed."
      if all_set APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID; then
        for name in APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID; do export_variable "$name"; done
        echo "macOS: the app will be notarized."
      else
        echo "::warning::macOS: signed but not notarized (APPLE_ID, APPLE_PASSWORD and APPLE_TEAM_ID are not all set). Gatekeeper will still warn."
      fi
    else
      echo "::warning::macOS: no signing certificate, so the app is not signed or notarized. Gatekeeper will warn on first open."
    fi
    ;;
  Windows | Windows_NT | MINGW* | MSYS*)
    if all_set AZURE_TENANT_ID AZURE_CLIENT_ID AZURE_CLIENT_SECRET TRUSTED_SIGNING_ENDPOINT TRUSTED_SIGNING_ACCOUNT TRUSTED_SIGNING_PROFILE; then
      for name in AZURE_TENANT_ID AZURE_CLIENT_ID AZURE_CLIENT_SECRET; do export_variable "$name"; done
      # Tauri runs this command on each file it wants signed (%1). The endpoint, account and profile are not secrets.
      command="trusted-signing-cli -e ${TRUSTED_SIGNING_ENDPOINT} -a ${TRUSTED_SIGNING_ACCOUNT} -c ${TRUSTED_SIGNING_PROFILE} -d TermZip %1"
      printf 'TERMZIP_TAURI_CONFIG={"bundle":{"windows":{"signCommand":"%s"}}}\n' "$command" >> "$target"
      echo "TERMZIP_SIGN_WINDOWS=yes" >> "$target"
      echo "Windows: the installers will be signed with Azure Trusted Signing."
    else
      echo "::warning::Windows: no signing credentials, so the installers are not signed. SmartScreen will warn."
    fi
    ;;
  *)
    echo "Linux: nothing to sign (the packages are not signed; the checksums are published)."
    ;;
esac
