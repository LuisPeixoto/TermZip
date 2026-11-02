#!/usr/bin/env bash
# Starts the release app in bench mode on this computer, waits for its report and takes a screenshot of the screen it
# ran on. It is what the smoke workflow runs on macOS and Linux (on Linux under `xvfb-run`), and it can be run by
# hand after `npm run tauri build -- --no-bundle`:
#
#   tools/smoke.sh                 report and screenshot in ./smoke-out
#   OUT_DIR=/tmp/x LINES_TO_PRINT=100000 tools/smoke.sh
#
# Everything is throwaway (a new data folder, an empty ZDOTDIR) and only the app it started is stopped.
set -euo pipefail

cd "$(dirname "$0")/.."
binary="${TERMZIP_BIN:-target/release/termzip}"
out="${OUT_DIR:-smoke-out}"
lines="${LINES_TO_PRINT:-200000}"
[ -x "$binary" ] || { echo "no app at $binary: build it first (npm run tauri build -- --no-bundle)" >&2; exit 1; }

mkdir -p "$out"
scratch="$(mktemp -d "${TMPDIR:-/tmp}/termzip-smoke.XXXXXX")"
app_pid=""
cleanup() {
  if [ -n "$app_pid" ]; then kill "$app_pid" 2>/dev/null || true; fi
  rm -rf "$scratch"
}
trap cleanup EXIT

shell="$(command -v zsh || command -v bash)"
mkdir -p "$scratch/home" "$scratch/zdot"
printf 'shell = "%s"\nrestore_session = false\ngit_status = false\n' "$shell" > "$scratch/home/config.toml"
rm -f "$out/app.json"

TERMZIP_HOME="$scratch/home" ZDOTDIR="$scratch/zdot" TERMZIP_BENCH="$out/app.json" TERMZIP_BENCH_LINES="$lines" \
  "$binary" > "$out/app.log" 2>&1 &
app_pid=$!

for _ in $(seq 1 360); do
  [ -f "$out/app.json" ] && break
  kill -0 "$app_pid" 2>/dev/null || { echo "the app ended before reporting" >&2; cat "$out/app.log" >&2; exit 1; }
  sleep 0.5
done
[ -f "$out/app.json" ] || { echo "no report after 3 minutes" >&2; cat "$out/app.log" >&2; exit 1; }

# What the screen shows after the flood. The runner is a throwaway virtual machine, so the whole screen is fine here.
sleep 1
case "$(uname)" in
  Darwin) screencapture -x "$out/screen.png" || true ;;
  *) import -window root "$out/screen.png" 2>/dev/null || xwd -root -silent 2>/dev/null | convert xwd:- "$out/screen.png" || true ;;
esac

python3 - "$out/app.json" <<'PY'
import json, sys
report = json.load(open(sys.argv[1]))
marks = report["marksMs"]
print(f"{report['os']} {report['arch']}: prompt after {report['launchToPromptMs']:.0f} ms, "
      f"{report['lines']:,} lines at {report.get('floodLinesPerSecond') or 0:,.0f} per second")
missing = [m for m in ("mounted", "painted", "prompt", "flood_start", "done") if m not in marks]
if report["failed"] or missing:
    sys.exit(f"the run did not complete (failed={report['failed']}, missing={missing})")
PY
