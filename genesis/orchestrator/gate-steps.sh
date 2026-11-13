#!/usr/bin/env bash
# Sequential checks; argv arrays preserve quoting. No eval and no event store.
# Usage: gate-steps.sh fmt -- cargo fmt --check ::: lint -- node cargo-lint.mjs ...
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
  if "${command[@]}"; then
    result=0
  else
    result=$?
  fi
  if (( result )); then
    echo "gate-steps: $name failed with exit code $result" >&2
    (( first )) || first=$result
  fi
done
exit "$first"
