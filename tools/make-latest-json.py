#!/usr/bin/env python3
"""Writes the `latest.json` the updater reads, from the files of a release.

    tools/make-latest-json.py --version 1.0.0 --notes-file notes.md --assets assets \
        --base-url https://github.com/LuisPeixoto/TermZip/releases/download/v1.0.0 > latest.json

Each platform needs its update file and the `.sig` that goes with it (made by `tauri build` with the signing key).
The signature is copied as it is; whether it is right is decided by the app, against the public key built into it,
before it installs anything. A platform whose files are missing is an error, not silently left out: a release that
cannot update someone should not be published by accident.
"""
import argparse
import datetime
import json
import os
import sys

# platform key of the updater -> (what the update file is called, with {v} for the version)
PLATFORMS = {
    "darwin-aarch64": "TermZip_{v}_aarch64.app.tar.gz",
    "darwin-x86_64": "TermZip_{v}_x64.app.tar.gz",
    "linux-x86_64": "TermZip_{v}_amd64.AppImage",
    "windows-x86_64": "TermZip_{v}_x64-setup.exe",
}


def build(version, notes, assets, base_url, published=None):
    platforms = {}
    missing = []
    for key, pattern in PLATFORMS.items():
        name = pattern.format(v=version)
        file, signature = os.path.join(assets, name), os.path.join(assets, name + ".sig")
        if not (os.path.isfile(file) and os.path.isfile(signature)):
            missing.append(name)
            continue
        with open(signature, encoding="utf-8") as handle:
            platforms[key] = {"signature": handle.read().strip(), "url": f"{base_url.rstrip('/')}/{name}"}
    if missing:
        raise SystemExit("missing update files (each needs its .sig too): " + ", ".join(missing))
    return {
        "version": version,
        "notes": notes.strip(),
        "pub_date": published or datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "platforms": platforms,
    }


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--version", required=True)
    parser.add_argument("--notes-file", required=True)
    parser.add_argument("--assets", required=True)
    parser.add_argument("--base-url", required=True)
    parser.add_argument("--date", help="the publication date, as an RFC 3339 UTC time (default: now)")
    args = parser.parse_args(argv)
    with open(args.notes_file, encoding="utf-8") as handle:
        notes = handle.read()
    json.dump(build(args.version, notes, args.assets, args.base_url, args.date), sys.stdout, indent=2, ensure_ascii=False)
    print()


if __name__ == "__main__":
    main()
