#!/usr/bin/env bash
# Tests tools/update-packages.sh against the real SHA256SUMS of v0.5.0 (a fixture), without the network.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"; rm -rf "$root/packaging/winget/manifests/l/LuisPeixoto/TermZip/9.9.9"' EXIT

cat > "$work/SHA256SUMS" <<'SUMS'
fbe6a49bd4aace3abd1c7ed4a27c97713eb9c802f80454e5869d5af3ecd48894  TermZip-9.9.9-1.x86_64.rpm
01011e8c4b7c5c14687e83c9fe138ac735f5116874e3a201ce66dddd56283745  TermZip_9.9.9_aarch64.dmg
b5437aa806e9b46145cb49f13146f3210f11838b5f85b791cc60b98856016c47  TermZip_9.9.9_x64-setup.exe
4f38bf31c9218811ba6a011a5cdc35f0092c1dada78cb4aa0af93dc9938aa80c  TermZip_9.9.9_x64.dmg
SUMS

fail() { echo "FAIL: $1" >&2; exit 1; }
SUMS_FILE="$work/SHA256SUMS" OUT_DIR="$work/out" RELEASE_DATE=2026-01-02 bash "$root/tools/update-packages.sh" v9.9.9 >/dev/null

cask="$work/out/homebrew/Casks/termzip.rb"
grep -q 'version "9.9.9"' "$cask" || fail "cask version"
grep -q 'arm:   "01011e8c4b7c5c14687e83c9fe138ac735f5116874e3a201ce66dddd56283745"' "$cask" || fail "cask arm hash"
grep -q 'intel: "4f38bf31c9218811ba6a011a5cdc35f0092c1dada78cb4aa0af93dc9938aa80c"' "$cask" || fail "cask intel hash"
grep -q 'TermZip_#{version}_#{arch}.dmg' "$cask" || fail "cask url"
if command -v ruby >/dev/null; then ruby -c "$cask" >/dev/null || fail "cask is not valid ruby"; fi

scoop="$work/out/scoop/bucket/termzip.json"
python3 -m json.tool "$scoop" >/dev/null || fail "scoop manifest is not JSON"
python3 - "$scoop" <<'PY' || fail "scoop manifest content"
import json, sys
m = json.load(open(sys.argv[1]))
assert m["version"] == "9.9.9"
assert m["hash"] == "b5437aa806e9b46145cb49f13146f3210f11838b5f85b791cc60b98856016c47"
assert m["url"].endswith("/v9.9.9/TermZip_9.9.9_x64-setup.exe")
assert m["installer"]["args"][-1] == "/D=$dir"
assert "${version}" in m["autoupdate"]["url"]
PY

wg="$root/packaging/winget/manifests/l/LuisPeixoto/TermZip/9.9.9"
grep -q 'InstallerSha256: B5437AA806E9B46145CB49F13146F3210F11838B5F85B791CC60B98856016C47' "$wg/LuisPeixoto.TermZip.installer.yaml" || fail "winget hash"
grep -q 'ReleaseDate: 2026-01-02' "$wg/LuisPeixoto.TermZip.installer.yaml" || fail "winget date"
grep -q 'PackageVersion: 9.9.9' "$wg/LuisPeixoto.TermZip.yaml" || fail "winget version"

# A release without the file must stop the script, never write a half manifest.
head -n 2 "$work/SHA256SUMS" > "$work/short"
if SUMS_FILE="$work/short" OUT_DIR="$work/out2" bash "$root/tools/update-packages.sh" v9.9.9 >/dev/null 2>&1; then
  fail "a missing hash was accepted"
fi
if bash "$root/tools/update-packages.sh" 1.0 >/dev/null 2>&1; then fail "a bad tag was accepted"; fi
echo "update-packages: all cases pass"
