#!/bin/bash
# hc-mesh-happ-freshness.test.sh — self-test for hc-mesh.sh's happ_bundle_freshness
# (WASM -> DNA -> HAPP cascade). Sources hc-mesh.sh the same way
# hc-mesh-prologue.sh does (dispatch guard: BASH_SOURCE != $0 skips the CLI
# switch), then drives the function against a scratch git repo with
# `touch -d`-controlled mtimes and a stubbed `hc` so no real Holochain binary
# or wasm build is ever invoked.
#
# Root cause this guards: genesis/a2o/reports/recovery/serving-edge-20260920/
# FINDING-election-ordering-503.md — a repack guard that compared only *.dna
# vs elohim.happ mtime let a rebuilt coordinator wasm with no repacked .dna
# install a week-old coordinator on a MESH_RESET household, silently.
#
# Usage: bash hc-mesh-happ-freshness.test.sh
set -u

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=safe.directory GIT_CONFIG_VALUE_0='*'

# shellcheck source=hc-mesh.sh
source "$SCRIPT_DIR/hc-mesh.sh"

FAILS=0
pass() { echo "PASS: $1"; }
fail() { echo "FAIL: $1"; FAILS=$((FAILS + 1)); }

TESTROOT="$(mktemp -d)"
cleanup() { rm -rf "$TESTROOT"; }
trap cleanup EXIT

# --- scratch DNA fixture: one fake "testdna" row standing in for the real
# five-row HAPP_DNA_TABLE, plus its own throwaway git repo so
# _happ_newest_tracked_mtime has real tracked-source semantics to read.
mkdir -p "$TESTROOT/elohim/holochain/dna/testdna/zomes/testzome/src"
mkdir -p "$TESTROOT/elohim/holochain/target/wasm32-unknown-unknown/release"
mkdir -p "$TESTROOT/elohim/holochain/dna/elohim/workdir"
(
  cd "$TESTROOT"
  git init -q
  git config user.email test@example.com
  git config user.name test
  echo 'fn main() {}' > elohim/holochain/dna/testdna/zomes/testzome/src/lib.rs
  git add -A
  git commit -q -m init
)

# Override the globals happ_bundle_freshness reads, pointing everything at
# the scratch fixture instead of the real repo.
REPO_ROOT="$TESTROOT"
HAPP_WORKDIR="$TESTROOT/elohim/holochain/dna/elohim/workdir"
HAPP_WASM_RELEASE_DIR="$TESTROOT/elohim/holochain/target/wasm32-unknown-unknown/release"
HAPP_DNA_TABLE=(
  "testdna|elohim/holochain/dna/testdna/zomes|test.wasm|elohim/holochain/dna/testdna|elohim/holochain/dna/elohim/workdir/testdna.dna"
)
HAPP_PATH="$HAPP_WORKDIR/elohim.happ"
HAPP_PATH_EXPLICIT=0
unset MESH_ALLOW_STALE_HAPP

# Stub `hc` — the launcher must never build wasm, and this test must never
# shell out to a real Holochain toolchain either. Simulates `hc dna pack . -o
# <path>` / `hc app pack . -o <path>` by just touching the named output.
hc() {
  local out=""
  while [ $# -gt 0 ]; do
    case "$1" in
      -o) out="$2"; shift 2 ;;
      *) shift ;;
    esac
  done
  [ -n "$out" ] && : > "$out"
}

SRC="$TESTROOT/elohim/holochain/dna/testdna/zomes/testzome/src/lib.rs"
WASM="$HAPP_WASM_RELEASE_DIR/test.wasm"
DNA="$TESTROOT/elohim/holochain/dna/elohim/workdir/testdna.dna"
HAPP="$HAPP_PATH"

echo "=== fresh -> no action ==="
touch -d "2026-01-01T00:00:00" "$SRC"
touch -d "2026-01-01T00:00:10" "$WASM"
touch -d "2026-01-01T00:00:20" "$DNA"
touch -d "2026-01-01T00:00:30" "$HAPP"
out="$(happ_bundle_freshness apply)"; rc=$?
if [ "$rc" -eq 0 ] && ! grep -qiE 'repack|REFUSED|WARN' <<<"$out"; then
  pass "fresh bundle takes no action (rc=0, no repack/refuse/warn lines)"
else
  fail "fresh bundle should be a no-op: rc=$rc"$'\n'"$out"
fi

echo "=== wasm newer than dna -> repack cascade ==="
touch -d "2026-01-01T00:00:00" "$SRC"
touch -d "2026-01-01T00:00:50" "$WASM"
touch -d "2026-01-01T00:00:10" "$DNA"
touch -d "2026-01-01T00:00:20" "$HAPP"
out="$(happ_bundle_freshness report)"; rc=$?
if [ "$rc" -eq 0 ] \
  && grep -q "would repack: elohim/holochain/dna/elohim/workdir/testdna.dna" <<<"$out" \
  && grep -q "would repack: elohim.happ" <<<"$out" \
  && [ "$(stat -c %Y "$DNA")" = "$(date -d 2026-01-01T00:00:10 +%s)" ]; then
  pass "report mode names the wasm->dna->happ cascade without touching disk"
else
  fail "report mode cascade wrong: rc=$rc"$'\n'"$out"
fi
# reset mtimes (report must not have mutated, but be explicit) and apply for real
touch -d "2026-01-01T00:00:10" "$DNA"
touch -d "2026-01-01T00:00:20" "$HAPP"
out="$(happ_bundle_freshness apply)"; rc=$?
if [ "$rc" -eq 0 ] \
  && grep -q "^repacking elohim/holochain/dna/elohim/workdir/testdna.dna" <<<"$out" \
  && grep -q "^repacking elohim.happ" <<<"$out" \
  && [ "$(stat -c %Y "$DNA")" -gt "$(date -d 2026-01-01T00:00:50 +%s)" ] \
  && [ "$(stat -c %Y "$HAPP")" -ge "$(stat -c %Y "$DNA")" ]; then
  pass "apply mode actually repacks the .dna then the .happ, in order"
else
  fail "apply mode cascade wrong: rc=$rc"$'\n'"$out"
fi

echo "=== source newer than wasm -> REFUSED with rebuild command ==="
touch -d "2026-01-01T00:00:00" "$WASM"
touch -d "2026-01-01T00:01:00" "$SRC"
touch -d "2026-01-01T00:00:10" "$DNA"
touch -d "2026-01-01T00:00:20" "$HAPP"
unset MESH_ALLOW_STALE_HAPP
out="$(happ_bundle_freshness apply)"; rc=$?
if [ "$rc" -eq 1 ] \
  && grep -q "^REFUSED hApp bundle: testdna coordinator wasm (test.wasm) is STALE" <<<"$out" \
  && grep -q 'cd elohim/holochain/dna/testdna && just build' <<<"$out" \
  && grep -q 'MESH_ALLOW_STALE_HAPP=1' <<<"$out"; then
  pass "stale wasm vs tracked source REFUSES and names the rebuild command"
else
  fail "stale-wasm refuse wrong: rc=$rc"$'\n'"$out"
fi

echo "=== MESH_ALLOW_STALE_HAPP=1 -> proceeds with a warning ==="
touch -d "2026-01-01T00:00:00" "$WASM"
touch -d "2026-01-01T00:01:00" "$SRC"
touch -d "2026-01-01T00:00:10" "$DNA"
touch -d "2026-01-01T00:00:20" "$HAPP"
export MESH_ALLOW_STALE_HAPP=1
out="$(happ_bundle_freshness apply)"; rc=$?
unset MESH_ALLOW_STALE_HAPP
if [ "$rc" -eq 0 ] \
  && grep -q "^WARN hApp bundle: testdna coordinator wasm (test.wasm) is STALE" <<<"$out" \
  && grep -q 'proceeding with the stale coordinator on purpose' <<<"$out"; then
  pass "MESH_ALLOW_STALE_HAPP=1 downgrades the refuse to a warning and proceeds (rc=0)"
else
  fail "override wrong: rc=$rc"$'\n'"$out"
fi

echo "=== explicit MESH_HAPP_PATH -> never repacked, reported ==="
EXPLICIT="$TESTROOT/explicit-elohim.happ"
touch -d "2026-01-01T00:00:00" "$EXPLICIT"
touch -d "2026-01-01T00:00:50" "$WASM"   # newer than the explicit bundle
touch -d "2026-01-01T00:00:10" "$DNA"
BEFORE_DNA_TS="$(stat -c %Y "$DNA")"
HAPP_PATH="$EXPLICIT"
HAPP_PATH_EXPLICIT=1
out="$(happ_bundle_freshness apply)"; rc=$?
if [ "$rc" -eq 0 ] \
  && grep -q "^hApp bundle: $EXPLICIT (explicit MESH_HAPP_PATH — never repacked or refused)" <<<"$out" \
  && grep -q "is older than the default workdir's newest built coordinator wasm" <<<"$out" \
  && [ "$(stat -c %Y "$EXPLICIT")" = "$(date -d 2026-01-01T00:00:00 +%s)" ] \
  && [ "$(stat -c %Y "$DNA")" = "$BEFORE_DNA_TS" ]; then
  pass "explicit MESH_HAPP_PATH is reported, warned-if-older, and never touched"
else
  fail "explicit path handling wrong: rc=$rc"$'\n'"$out"
fi
HAPP_PATH_EXPLICIT=0

echo
if [ "$FAILS" -eq 0 ]; then
  echo "hc-mesh-happ-freshness.test.sh: ALL PASS"
  exit 0
else
  echo "hc-mesh-happ-freshness.test.sh: $FAILS FAILURE(S)"
  exit 1
fi
