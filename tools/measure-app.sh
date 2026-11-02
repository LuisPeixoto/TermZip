#!/usr/bin/env bash
# Measures the release app the same way every time: the time from the start of the process to the first prompt, a
# flood of a million lines typed into the first terminal, and the CPU and memory of the app and its web view.
#
#   tools/measure-app.sh                 build the release app, run it 5 times, print the medians
#   RUNS=3 BUILD=0 tools/measure-app.sh  reuse target/release/termzip, 3 runs
#   OUT=app.json tools/measure-app.sh    also write the medians as JSON (for `termzip-bench check`)
#
# It uses a throwaway data folder and an empty ZDOTDIR, so neither your settings nor your shell's startup files
# change the result, and it only ever stops the app it started. macOS and Linux (on Linux without a display, run it
# under `xvfb-run`). The window must be visible: a web view that is hidden behind other windows draws slowly.
set -euo pipefail

cd "$(dirname "$0")/.."
runs="${RUNS:-5}"
lines="${LINES_TO_PRINT:-1000000}"
binary="${TERMZIP_BIN:-target/release/termzip}"
idle_seconds="${IDLE_SECONDS:-20}"

if [ "${BUILD:-1}" = 1 ] || [ ! -x "$binary" ]; then
  (cd apps/desktop && npm ci && npm run tauri build -- --no-bundle)
fi
[ -x "$binary" ] || { echo "no app at $binary" >&2; exit 1; }

scratch="$(mktemp -d "${TMPDIR:-/tmp}/termzip-measure.XXXXXX")"
app_pid=""
cleanup() {
  if [ -n "$app_pid" ]; then kill "$app_pid" 2>/dev/null || true; fi
  rm -rf "$scratch"
}
trap cleanup EXIT

shell="$(command -v zsh || command -v bash)"
mkdir -p "$scratch/home" "$scratch/zdot"
printf 'shell = "%s"\nrestore_session = false\ngit_status = false\n' "$shell" > "$scratch/home/config.toml"

# Prints "<megabytes> <cpu seconds>" for the app, what it started, and the web view processes that appeared when it
# did (`$1` is the app, `$2` the file listing the web view processes that existed before).
sample() {
  python3 - "$1" "$2" <<'PY'
import subprocess, sys

app = int(sys.argv[1])
before = {int(x) for x in open(sys.argv[2]).read().split()}
rows = subprocess.run(["ps", "-axo", "pid=,ppid=,rss=,time=,command="], capture_output=True, text=True).stdout.splitlines()

def cpu(text):
    # [dd-][hh:]mm:ss[.cc]
    days = 0
    if "-" in text:
        d, text = text.split("-", 1); days = int(d)
    parts = [float(p) for p in text.split(":")]
    while len(parts) < 3: parts.insert(0, 0.0)
    return days * 86400 + parts[0] * 3600 + parts[1] * 60 + parts[2]

procs = {}
for row in rows:
    pieces = row.split(None, 4)
    if len(pieces) < 5: continue
    pid, ppid, rss, time, command = int(pieces[0]), int(pieces[1]), int(pieces[2]), pieces[3], pieces[4]
    procs[pid] = (ppid, rss, cpu(time), command)

ours = {app}
changed = True
while changed:
    changed = False
    for pid, (ppid, *_rest) in procs.items():
        if ppid in ours and pid not in ours:
            ours.add(pid); changed = True
web = [pid for pid, (_p, _r, _c, cmd) in procs.items() if "WebKit" in cmd or "WebContent" in cmd or "WebProcess" in cmd or "NetworkProcess" in cmd]
ours |= {pid for pid in web if pid not in before}
print(f"{sum(procs[p][1] for p in ours if p in procs) / 1024:.1f} {sum(procs[p][2] for p in ours if p in procs):.2f}")
PY
}

web_processes() {
  ps -axo pid=,command= | awk '/WebKit|WebContent|WebProcess|NetworkProcess/ && !/awk/ {print $1}'
}

front() {
  # A web view that is hidden draws slowly, so bring our window forward (macOS only, and only our process).
  if [ "$(uname)" = Darwin ]; then
    osascript -e "tell application \"System Events\" to set frontmost of (first process whose unix id is $1) to true" >/dev/null 2>&1 || true
  fi
}

for run in $(seq 1 "$runs"); do
  report="$scratch/run-$run.json"
  web_processes > "$scratch/before.txt"
  TERMZIP_HOME="$scratch/home" ZDOTDIR="$scratch/zdot" TERMZIP_BENCH="$report" TERMZIP_BENCH_LINES="$lines" \
    "$binary" > "$scratch/app.log" 2>&1 &
  app_pid=$!
  for _ in $(seq 1 100); do kill -0 "$app_pid" 2>/dev/null && break; sleep 0.1; done
  # The window has to be in front before the flood starts (about half a second after launch).
  for _ in $(seq 1 20); do front "$app_pid"; sleep 0.1; done
  for _ in $(seq 1 600); do [ -f "$report" ] && break; sleep 0.5; done
  [ -f "$report" ] || { echo "run $run: no report" >&2; cat "$scratch/app.log" >&2; exit 1; }

  read -r flood_mb _ < <(sample "$app_pid" "$scratch/before.txt")
  read -r _ cpu_start < <(sample "$app_pid" "$scratch/before.txt")
  sleep "$idle_seconds"
  read -r idle_mb cpu_end < <(sample "$app_pid" "$scratch/before.txt")
  python3 - "$report" "$flood_mb" "$idle_mb" "$cpu_start" "$cpu_end" "$idle_seconds" <<'PY'
import json, sys
path, flood_mb, idle_mb, cpu_start, cpu_end, seconds = sys.argv[1:]
report = json.load(open(path))
report["floodMemoryMb"] = float(flood_mb)
report["idleMemoryMb"] = float(idle_mb)
report["idleCpuPercent"] = (float(cpu_end) - float(cpu_start)) / float(seconds) * 100
json.dump(report, open(path, "w"), indent=2)
PY
  kill "$app_pid" 2>/dev/null || true
  wait "$app_pid" 2>/dev/null || true
  app_pid=""
  echo "run $run done" >&2
done

python3 - "$scratch" "$runs" "${OUT:-}" <<'PY'
import json, statistics, sys
scratch, runs, out = sys.argv[1], int(sys.argv[2]), sys.argv[3]
reports = [json.load(open(f"{scratch}/run-{n}.json")) for n in range(1, runs + 1)]
if any(r.get("failed") for r in reports):
    sys.exit("a run failed: the flood did not finish")
keys = ["launchToPromptMs", "floodLinesPerSecond", "idleCpuPercent", "idleMemoryMb", "floodMemoryMb"]
medians = {k: statistics.median(r[k] for r in reports) for k in keys}
spread = {k: (min(r[k] for r in reports), max(r[k] for r in reports)) for k in keys}
labels = {
    "launchToPromptMs": "launch to first prompt (ms)",
    "floodLinesPerSecond": f"flood, lines per second",
    "idleCpuPercent": "idle CPU (% of one core)",
    "idleMemoryMb": "memory after the flood, idle (MB)",
    "floodMemoryMb": "memory right after the flood (MB)",
}
first = reports[0]
print(f"\n{first['os']} {first['arch']}, TermZip {first['version']}, {first['lines']:,} lines, {runs} runs (median, min-max)")
for k in keys:
    print(f"  {labels[k]:<38} {medians[k]:>12,.1f}   {spread[k][0]:,.1f} - {spread[k][1]:,.1f}")
if out:
    json.dump({**medians, "os": first["os"], "arch": first["arch"], "version": first["version"], "runs": runs}, open(out, "w"), indent=2)
PY
