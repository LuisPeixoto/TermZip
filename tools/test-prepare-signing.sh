#!/usr/bin/env bash
# Tests tools/prepare-signing.sh with each combination of secrets. It needs no secrets and signs nothing.
set -uo pipefail
cd "$(dirname "$0")/.."

failures=0
run() {
  # run <system> <expected text in the output> <expected lines in the env file, or "none"> [NAME=value ...]
  local system="$1" want_output="$2" want_env="$3"; shift 3
  local env_file output
  env_file="$(mktemp)"
  output="$(env -i PATH="$PATH" RUNNER_OS="$system" GITHUB_ENV="$env_file" "$@" bash tools/prepare-signing.sh 2>&1)"
  local status=$?
  local ok=1
  [ "$status" = 0 ] || ok=0
  echo "$output" | grep -qF -- "$want_output" || ok=0
  if [ "$want_env" = none ]; then
    [ ! -s "$env_file" ] || ok=0
  else
    for name in $want_env; do grep -q "^$name=" "$env_file" || ok=0; done
  fi
  # A value must never be printed.
  echo "$output" | grep -q "SECRET-VALUE" && ok=0
  if [ "$ok" = 1 ]; then echo "ok:     $system: $want_output"; else echo "FAILED: $system: $want_output"; echo "$output" | sed 's/^/        /'; cat "$env_file" | sed 's/=.*/=…/; s/^/        env: /'; failures=$((failures + 1)); fi
  rm -f "$env_file"
}

run macOS "no signing certificate" none
run macOS "the app will be signed" "APPLE_CERTIFICATE APPLE_CERTIFICATE_PASSWORD APPLE_SIGNING_IDENTITY" \
  APPLE_CERTIFICATE=SECRET-VALUE APPLE_CERTIFICATE_PASSWORD=SECRET-VALUE APPLE_SIGNING_IDENTITY="Developer ID Application: X"
run macOS "signed but not notarized" "APPLE_CERTIFICATE" \
  APPLE_CERTIFICATE=SECRET-VALUE APPLE_CERTIFICATE_PASSWORD=SECRET-VALUE APPLE_SIGNING_IDENTITY=id APPLE_ID=a@example.com
run macOS "the app will be notarized" "APPLE_CERTIFICATE APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID" \
  APPLE_CERTIFICATE=SECRET-VALUE APPLE_CERTIFICATE_PASSWORD=SECRET-VALUE APPLE_SIGNING_IDENTITY=id APPLE_ID=a@example.com APPLE_PASSWORD=SECRET-VALUE APPLE_TEAM_ID=T
# An empty value is a missing one: this is what an undefined secret looks like to a step.
run macOS "no signing certificate" none APPLE_CERTIFICATE= APPLE_CERTIFICATE_PASSWORD= APPLE_SIGNING_IDENTITY=
run Windows "no signing credentials" none
run Windows "no signing credentials" none AZURE_CLIENT_ID=x AZURE_CLIENT_SECRET=SECRET-VALUE
run Windows "signed with Azure Trusted Signing" "AZURE_TENANT_ID AZURE_CLIENT_ID AZURE_CLIENT_SECRET TERMZIP_TAURI_CONFIG TERMZIP_SIGN_WINDOWS" \
  AZURE_TENANT_ID=t AZURE_CLIENT_ID=c AZURE_CLIENT_SECRET=SECRET-VALUE TRUSTED_SIGNING_ENDPOINT=https://eus.codesigning.azure.net TRUSTED_SIGNING_ACCOUNT=acc TRUSTED_SIGNING_PROFILE=prof
run Linux "nothing to sign" none APPLE_CERTIFICATE=SECRET-VALUE

# The Windows config must be valid JSON for Tauri's --config.
env_file="$(mktemp)"
env -i PATH="$PATH" RUNNER_OS=Windows GITHUB_ENV="$env_file" AZURE_TENANT_ID=t AZURE_CLIENT_ID=c AZURE_CLIENT_SECRET=s \
  TRUSTED_SIGNING_ENDPOINT=https://eus.codesigning.azure.net TRUSTED_SIGNING_ACCOUNT=acc TRUSTED_SIGNING_PROFILE=prof bash tools/prepare-signing.sh >/dev/null
if sed -n 's/^TERMZIP_TAURI_CONFIG=//p' "$env_file" | python3 -c 'import json,sys; c=json.load(sys.stdin)["bundle"]["windows"]["signCommand"]; assert "%1" in c and "SECRET" not in c'; then
  echo "ok:     the Windows config is valid JSON and has no secret"
else
  echo "FAILED: the Windows config"; failures=$((failures + 1))
fi
rm -f "$env_file"

[ "$failures" = 0 ] && echo "all passed" || { echo "$failures failed"; exit 1; }
