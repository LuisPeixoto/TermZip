#!/usr/bin/env bash
# Tests that need something real next to the code: they are `#[ignore]`d, so `cargo test` never runs them.
# Everything is throwaway: nothing here reads or writes your real ~/.ssh, and what it starts is stopped and
# deleted when the script ends, also when it fails.
#
#   tools/live-tests.sh          run all of them
#   tools/live-tests.sh ssh      only the SSH profile test
#
# macOS and Linux. It needs `sshd`, `ssh` and `ssh-keygen`.
set -euo pipefail

cd "$(dirname "$0")/.."
only="${1:-all}"
scratch="$(mktemp -d "${TMPDIR:-/tmp}/termzip-live.XXXXXX")"
sshd_pid=""

cleanup() {
  if [ -n "$sshd_pid" ]; then kill "$sshd_pid" 2>/dev/null || true; fi
  rm -rf "$scratch"
}
trap cleanup EXIT

run_ssh() {
  local sshd
  sshd="$(command -v sshd || true)"
  [ -n "$sshd" ] || sshd=/usr/sbin/sshd
  [ -x "$sshd" ] || { echo "skipped: no sshd on this computer"; return 0; }

  # A port nobody listens on, on this computer only.
  local port
  port="$(python3 -c 'import socket; s = socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1])')"

  ssh-keygen -q -t ed25519 -N "" -C termzip-live -f "$scratch/host_key"
  ssh-keygen -q -t ed25519 -N "" -C termzip-live -f "$scratch/user_key"
  cp "$scratch/user_key.pub" "$scratch/authorized_keys"
  chmod 600 "$scratch/authorized_keys" "$scratch/host_key" "$scratch/user_key"

  cat > "$scratch/sshd_config" <<CONFIG
Port $port
ListenAddress 127.0.0.1
HostKey $scratch/host_key
PidFile $scratch/sshd.pid
AuthorizedKeysFile $scratch/authorized_keys
PubkeyAuthentication yes
PasswordAuthentication no
KbdInteractiveAuthentication no
UsePAM no
StrictModes no
AllowUsers $(id -un)
CONFIG
  "$sshd" -D -e -f "$scratch/sshd_config" > "$scratch/sshd.log" 2>&1 &
  sshd_pid=$!
  for _ in $(seq 1 50); do
    if (echo > "/dev/tcp/127.0.0.1/$port") 2>/dev/null; then break; fi
    sleep 0.1
  done

  # The config `ssh` will read: a host alias, like the ones people keep in ~/.ssh/config. `ssh` takes its config
  # from the account's home folder and not from $HOME, so a wrapper on the PATH adds `-F` to this one.
  cat > "$scratch/ssh_config" <<CONFIG
Host livebox
  HostName 127.0.0.1
  Port $port
  User $(id -un)
  IdentityFile $scratch/user_key
  IdentitiesOnly yes
  UserKnownHostsFile $scratch/known_hosts
  StrictHostKeyChecking accept-new
CONFIG
  mkdir -p "$scratch/bin"
  printf '#!/bin/sh\nexec "%s" -F "%s" "$@"\n' "$(command -v ssh)" "$scratch/ssh_config" > "$scratch/bin/ssh"
  chmod +x "$scratch/bin/ssh"

  TERMZIP_LIVE_SSH_BIN="$scratch/bin" \
    cargo test -p termzip-desktop -- --ignored --nocapture an_ssh_profile_signs_in_to_a_real_sshd \
    || { echo "--- sshd log"; cat "$scratch/sshd.log"; return 1; }
}

case "$only" in
  all | ssh) run_ssh ;;
  *) echo "unknown test: $only" >&2; exit 2 ;;
esac
echo "live tests passed"
