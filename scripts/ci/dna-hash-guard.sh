#!/usr/bin/env bash
# dna-hash-guard.sh — refuse a build whose packed DNA hash silently moved.
#
# WHY: commit 03f331f21 (2026-09-02) added an unconditional wasm-ld link arg
# believed hash-neutral in CI. It was not — the next DNA build (elohim-holochain
# #1416) produced INTEGRITY wasm with different bytes for all five DNAs, moving
# every DNA hash. On the fleet, storage's drift path read the new hash as a
# different hApp, uninstalled the running one, and tore the source chains. A
# moved DNA hash is a network event (peers on different hashes are different
# DHTs), not an ordinary code change — it must never reach CI silently.
#
# Compares each role's freshly-packed DNA hash (written during the "Build DNA"
# stage, one `role=hash` line per DNA) against the committed baseline
# (elohim/holochain/dna/dna-hashes.baseline). A mismatch fails the build unless
# the triggering commit is tagged `[dna:migrate]` — the same commit-tag
# convention the orchestrator Jenkinsfile uses for `[build:*]`. An intentional
# hash move updates dna-hashes.baseline AND carries the tag in the same commit;
# this guard enforces the tag, not that the baseline was actually bumped (an
# omitted baseline update just means the next build mismatches again).
#
# Usage: dna-hash-guard.sh <baseline-file> <actual-hashes-file>
set -euo pipefail

if [ "$#" -ne 2 ]; then
  echo "usage: $0 <baseline-file> <actual-hashes-file>" >&2
  exit 2
fi

baseline_file="$1"
actual_file="$2"

if [ ! -f "$baseline_file" ]; then
  echo "DNA-HASH-GUARD-ERROR: baseline file not found: $baseline_file" >&2
  exit 2
fi
if [ ! -f "$actual_file" ]; then
  echo "DNA-HASH-GUARD-ERROR: actual-hashes file not found: $actual_file (Build DNA stage should have written it)" >&2
  exit 2
fi

# In-container git ops need this or checkout's different UID makes git refuse
# the workspace as "dubious ownership" — defensive, Checkout already sets it.
git config --global --add safe.directory "*" 2>/dev/null || true

commit_msg="$(git log -1 --pretty=%B 2>/dev/null || true)"
migrate=0
if printf '%s' "$commit_msg" | grep -qiE '\[dna:migrate\]'; then
  migrate=1
fi

declare -A baseline
while IFS='=' read -r role hash; do
  [ -z "${role// }" ] && continue
  case "$role" in \#*) continue ;; esac
  baseline["$role"]="$hash"
done < "$baseline_file"

fail=0
while IFS='=' read -r role hash; do
  [ -z "${role// }" ] && continue
  case "$role" in \#*) continue ;; esac
  base="${baseline[$role]:-<none>}"
  if [ "$base" = "$hash" ]; then
    echo "DNA-HASH-CHECK ${role} baseline=${base} built=${hash} MATCH"
    continue
  fi

  if [ "$migrate" -eq 1 ]; then
    echo "DNA-HASH-CHECK ${role} baseline=${base} built=${hash} MISMATCH (allowed: commit tagged [dna:migrate])"
  else
    echo "INTEGRITY-HASH-MOVED ${role} baseline=${base} built=${hash} — a moved integrity hash orphans the fleet's DHT data; if intended, update dna-hashes.baseline in the same commit and tag it [dna:migrate]"
    fail=1
  fi
done < "$actual_file"

if [ "$fail" -ne 0 ]; then
  echo "DNA-HASH-GUARD: FAILED — one or more DNA hashes moved without [dna:migrate]. See INTEGRITY-HASH-MOVED lines above."
  exit 1
fi

echo "DNA-HASH-GUARD: all packed DNA hashes match the committed baseline (or the move is tagged [dna:migrate])."
