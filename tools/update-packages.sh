#!/usr/bin/env bash
# Writes the package manifests of a release from its SHA256SUMS.
#
#   tools/update-packages.sh v1.0.0              render into packaging/_out and packaging/winget/manifests
#   tools/update-packages.sh v1.0.0 --push       also clone, update and push the Homebrew tap and the Scoop bucket
#
# The hashes come from the release's own SHA256SUMS (downloaded with gh), never from a file that was built here, so a
# package says exactly what the release says. For tests, SUMS_FILE names a local SHA256SUMS and nothing is downloaded.
# OUT_DIR (default packaging/_out) is where the cask and the bucket manifest are written.
set -euo pipefail

tag="${1:?usage: update-packages.sh vX.Y.Z [--push]}"
push="no"
[ "${2:-}" = "--push" ] && push="yes"
case "$tag" in v[0-9]*.[0-9]*.[0-9]*) ;; *) echo "the tag must look like v1.2.3" >&2; exit 2 ;; esac
version="${tag#v}"
repo="LuisPeixoto/TermZip"
root="$(cd "$(dirname "$0")/.." && pwd)"
out="${OUT_DIR:-$root/packaging/_out}"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

if [ -n "${SUMS_FILE:-}" ]; then
  sums="$SUMS_FILE"
else
  gh release download "$tag" --repo "$repo" --pattern SHA256SUMS --dir "$work"
  sums="$work/SHA256SUMS"
fi

hash_of() {
  local name="$1" found
  found="$(awk -v n="$name" '$2 == n { print $1 }' "$sums")"
  if ! [[ "$found" =~ ^[0-9a-f]{64}$ ]]; then
    echo "no SHA-256 for $name in the release's SHA256SUMS" >&2
    exit 1
  fi
  printf '%s' "$found"
}

dmg_arm="TermZip_${version}_aarch64.dmg"
dmg_intel="TermZip_${version}_x64.dmg"
exe="TermZip_${version}_x64-setup.exe"
sha_arm="$(hash_of "$dmg_arm")"
sha_intel="$(hash_of "$dmg_intel")"
sha_exe="$(hash_of "$exe")"
base="https://github.com/$repo/releases/download/$tag"
date="${RELEASE_DATE:-$(date -u +%Y-%m-%d)}"

mkdir -p "$out/homebrew/Casks" "$out/scoop/bucket"

# --- Homebrew: one cask for both Macs -----------------------------------------------------------------------------
cat > "$out/homebrew/Casks/termzip.rb" <<EOF
cask "termzip" do
  arch arm: "aarch64", intel: "x64"

  version "$version"
  sha256 arm:   "$sha_arm",
         intel: "$sha_intel"

  url "https://github.com/$repo/releases/download/v#{version}/TermZip_#{version}_#{arch}.dmg"
  name "TermZip"
  desc "Terminal with command blocks, split panes and a WebAssembly plugin sandbox"
  homepage "https://github.com/$repo"

  depends_on :macos

  app "TermZip.app"

  zap trash: [
    "~/Library/Application Support/termzip",
    "~/Library/Caches/dev.termzip.app",
    "~/Library/WebKit/dev.termzip.app",
  ]

  caveats <<~EOS
    TermZip is not signed with an Apple Developer ID or notarized yet, so macOS may refuse to open it the first time.
    Open it with a right-click (Control-click) on the app, then Open, or allow it in System Settings, Privacy and
    Security.
  EOS
end
EOF

# --- Scoop ---------------------------------------------------------------------------------------------------------
cat > "$out/scoop/bucket/termzip.json" <<EOF
{
    "version": "$version",
    "description": "Terminal with command blocks, split panes and a WebAssembly plugin sandbox",
    "homepage": "https://github.com/$repo",
    "license": "MIT",
    "notes": "TermZip is not signed with a Windows certificate yet, so SmartScreen may warn the first time it is opened.",
    "url": "$base/$exe",
    "hash": "$sha_exe",
    "installer": {
        "file": "$exe",
        "args": [
            "/S",
            "/D=\$dir"
        ]
    },
    "uninstaller": {
        "file": "uninstall.exe",
        "args": "/S"
    },
    "bin": "termzip.exe",
    "shortcuts": [
        [
            "termzip.exe",
            "TermZip"
        ]
    ],
    "checkver": {
        "github": "https://github.com/$repo"
    },
    "autoupdate": {
        "url": "https://github.com/$repo/releases/download/v\${version}/TermZip_\${version}_x64-setup.exe"
    }
}
EOF

# --- Winget (submitted by hand as a pull request to microsoft/winget-pkgs) -----------------------------------------
wg="$root/packaging/winget/manifests/l/LuisPeixoto/TermZip/$version"
mkdir -p "$wg"
cat > "$wg/LuisPeixoto.TermZip.yaml" <<EOF
# yaml-language-server: \$schema=https://aka.ms/winget-manifest.version.1.6.0.schema.json
PackageIdentifier: LuisPeixoto.TermZip
PackageVersion: $version
DefaultLocale: en-US
ManifestType: version
ManifestVersion: 1.6.0
EOF
cat > "$wg/LuisPeixoto.TermZip.installer.yaml" <<EOF
# yaml-language-server: \$schema=https://aka.ms/winget-manifest.installer.1.6.0.schema.json
PackageIdentifier: LuisPeixoto.TermZip
PackageVersion: $version
InstallerType: nsis
Scope: user
InstallerSwitches:
  Silent: /S
  SilentWithProgress: /S
Installers:
  - Architecture: x64
    InstallerUrl: $base/$exe
    InstallerSha256: $(printf '%s' "$sha_exe" | tr 'a-f' 'A-F')
ReleaseDate: $date
ManifestType: installer
ManifestVersion: 1.6.0
EOF
cat > "$wg/LuisPeixoto.TermZip.locale.en-US.yaml" <<EOF
# yaml-language-server: \$schema=https://aka.ms/winget-manifest.defaultLocale.1.6.0.schema.json
PackageIdentifier: LuisPeixoto.TermZip
PackageVersion: $version
PackageLocale: en-US
Publisher: Luis Peixoto
PublisherUrl: https://github.com/LuisPeixoto
PackageName: TermZip
PackageUrl: https://github.com/$repo
License: MIT
LicenseUrl: https://github.com/$repo/blob/$tag/LICENSE
ShortDescription: Terminal with command blocks, split panes and a WebAssembly plugin sandbox.
Tags:
  - terminal
  - shell
  - tauri
ReleaseNotesUrl: https://github.com/$repo/releases/tag/$tag
ManifestType: defaultLocale
ManifestVersion: 1.6.0
EOF

echo "rendered $tag into $out and $wg"

if [ "$push" = "yes" ]; then
  for pair in "homebrew-termzip:homebrew" "scoop-termzip:scoop"; do
    name="${pair%%:*}"
    dir="${pair##*:}"
    git clone --quiet "git@github.com:LuisPeixoto/$name.git" "$work/$name"
    cp -R "$out/$dir/." "$work/$name/"
    (
      cd "$work/$name"
      git add .
      if git diff --cached --quiet; then
        echo "$name: nothing changed"
      else
        git commit --quiet -m "termzip $version"
        git push --quiet origin HEAD
        echo "$name: pushed $version"
      fi
    )
  done
fi
