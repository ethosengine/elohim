#!/usr/bin/env bash
# Sequential checks; argv arrays preserve quoting. No eval and no event store.
# Usage: gate-steps.sh fmt -- cargo fmt --check ::: lint -- node cargo-lint.mjs ...
# When GATE_STEP_TIMES names a file, each step appends one line to it: name, exit status, start
# and end in epoch seconds, tab-separated. gate-runner turns those lines into one gate-step reading
# row per step. Telemetry only: a write that fails is ignored and never changes the exit status.
set -uo pipefail
steps=("$@")
# Validate the entire plan before running any command.
while (( $# )); do
  [[ $# -ge 3 && "$1" =~ ^[a-zA-Z0-9_-]+$ && "$2" == -- && "$3" != ::: ]] || {
    echo 'gate-steps: expected name -- command [args...] ::: name -- command [args...]' >&2
    exit 2
  }
  shift 2
  while (( $# )) && [[ "$1" != ::: ]]; do shift; done
  if (( $# )); then
    shift
    (( $# )) || { echo 'gate-steps: trailing separator' >&2; exit 2; }
  fi
done
(( ${#steps[@]} )) || { echo 'gate-steps: no steps' >&2; exit 2; }
set -- "${steps[@]}"
first=0
while (( $# )); do
  name="$1"
  shift 2
  command=()
  while (( $# )) && [[ "$1" != ::: ]]; do command+=("$1"); shift; done
  (( $# )) && shift
  echo "gate-steps: running $name"
  step_started=${EPOCHREALTIME:-$(date +%s.%N)}
  if "${command[@]}"; then
    result=0
  else
    result=$?
  fi
  if [[ -n "${GATE_STEP_TIMES:-}" ]]; then
    step_ended=${EPOCHREALTIME:-$(date +%s.%N)}
    { printf '%s\t%s\t%s\t%s\n' "$name" "$result" "${step_started/,/.}" "${step_ended/,/.}" >> "$GATE_STEP_TIMES"; } 2>/dev/null || true
  fi
  if (( result )); then
    echo "gate-steps: $name failed with exit code $result" >&2
    (( first )) || first=$result
  fi
done
exit "$first"
