#!/usr/bin/env bash
# address-profile-lint.sh refuses a planted sha256- blob address and passes the
# allowlisted OCI `sha256:` line. Runs on a throwaway non-git tree.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
T="$(mktemp -d)"; trap 'rm -rf "$T"' EXIT
mkdir -p "$T/scripts/ci" "$T/genesis/seeder/src"
H64="$(printf 'a%.0s' $(seq 64))"
echo "image=harbor/x@sha256:$H64" > "$T/scripts/ci/build-storage-image.sh"
echo "const ok = 'bafkr4i';" > "$T/genesis/seeder/src/clean.ts"
ADDRESS_LINT_ROOT="$T" bash "$HERE/address-profile-lint.sh" >/dev/null || { echo "FAIL clean tree refused"; exit 1; }
echo "ok   clean tree + OCI digest passes"
echo "const a = 'sha256-$H64';" > "$T/genesis/seeder/src/bad.ts"
if ADDRESS_LINT_ROOT="$T" bash "$HERE/address-profile-lint.sh" >/dev/null; then echo "FAIL planted sha256- passed"; exit 1; fi
echo "ok   planted sha256- address refused"
echo "ALL PASS"
