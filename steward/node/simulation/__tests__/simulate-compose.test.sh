#!/usr/bin/env bash
# CLI compatibility only: no Docker daemon, containers, or network calls.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
script="$here/../simulate.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
mkdir "$tmp/bin"
ln -s "$(command -v dirname)" "$tmp/bin/dirname"
cat > "$tmp/docker" <<'MOCK'
#!/bin/bash
if [[ "${1:-}" == compose && "${2:-}" == version ]]; then
    exit "${PLUGIN_STATUS:-0}"
fi
printf 'docker' >> "$CALLS"
printf ' <%s>' "$@" >> "$CALLS"
printf '\n' >> "$CALLS"
exit "${COMMAND_STATUS:-0}"
MOCK
cat > "$tmp/docker-compose" <<'MOCK'
#!/bin/bash
printf 'docker-compose' >> "$CALLS"
printf ' <%s>' "$@" >> "$CALLS"
printf '\n' >> "$CALLS"
exit "${COMMAND_STATUS:-0}"
MOCK
chmod +x "$tmp/docker" "$tmp/docker-compose"
export CALLS="$tmp/calls"
run() {
    : > "$CALLS"
    set +e
    PATH="$tmp/bin" /bin/bash "$script" "$@" > "$tmp/output" 2>&1
    status=$?
    set -e
}
ln -s "$tmp/docker" "$tmp/bin/docker"
ln -s "$tmp/docker-compose" "$tmp/bin/docker-compose"
run logs 'node with spaces'
[[ "$status" == 0 ]]
grep -Fx 'docker <compose> <logs> <-f> <node with spaces>' "$CALLS"
[[ $(wc -l < "$CALLS") == 1 ]]
export PLUGIN_STATUS=1
run stop
[[ "$status" == 0 ]]
grep -Fx 'docker-compose <down>' "$CALLS"
unset PLUGIN_STATUS
export COMMAND_STATUS=42
run test
[[ "$status" == 42 ]]
grep -Fx 'docker <compose> <up> <-d>' "$CALLS"
[[ $(wc -l < "$CALLS") == 1 ]]
run start --latency
[[ "$status" == 42 ]]
grep -Fx 'docker <compose> <--profile> <latency> <up> <-d>' "$CALLS"
export PLUGIN_STATUS=1 COMMAND_STATUS=43
run clean
[[ "$status" == 43 ]]
grep -Fx 'docker-compose <down> <-v> <--rmi> <local> <--remove-orphans>' "$CALLS"
unset COMMAND_STATUS
rm "$tmp/bin/docker-compose"
unset PLUGIN_STATUS
run stop
[[ "$status" == 0 ]]
grep -Fx 'docker <compose> <down>' "$CALLS"
export PLUGIN_STATUS=1
run stop
[[ "$status" == 127 ]]
grep -F 'Docker Compose is required' "$tmp/output"
[[ ! -s "$CALLS" ]]
rm "$tmp/bin/docker"
run help
[[ "$status" == 0 ]]
run stop
[[ "$status" == 127 ]]
grep -F 'Docker Compose is required' "$tmp/output"
[[ ! -s "$CALLS" ]]
printf 'PASS: 9 Compose CLI selection, quoting, refusal, and failure cases\n'
