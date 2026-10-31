#!/usr/bin/env bash
# pool-postflight.sh — Stop hook for the cargo target pool.
#
# Re-runs worktree stewardship to catch any worktrees that were merged
# during this session (e.g., the agent finished a branch and merged it
# to dev, leaving its own worktree as a stale entry). Idempotent;
# always safe to run.
#
# Hook input on stdin: { hook_event_name: "Stop", ... }
# No additionalContext output — the session is ending.
#
# Failures must not block. Exit 0 always.
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/pool-lib.sh"

# Drain stdin.
cat >/dev/null 2>&1 || true

if ! command -v jq >/dev/null 2>&1; then exit 0; fi
if ! [ -d "$POOL_PARENT_REPO/.git" ] && ! [ -f "$POOL_PARENT_REPO/.git" ]; then
  exit 0
fi

pool_init

DRY="${CARGO_TARGET_POOL_POSTFLIGHT_DRY:-0}"
steward_all_worktrees "$DRY" >/dev/null 2>&1 || true

# Sample slot watermarks — HWM rises monotonically across sessions; feeds
# `cargo-pool estimate` so it can ground predictions in observed peaks
# rather than hardcoded 10G/3G.
#
# Policy enforcement at session end — the deterministic eviction pass
# (pool-policy.json: stale incrementals/hashes, dispositions, LRU budget).
# Guarded inside enforce_pool: active families, live PIDs, and flock'd
# slots are never touched, so a concurrent session's build is safe.
# Escape hatch: CARGO_TARGET_POOL_NO_ENFORCE=1.
#
# Both walk the whole pool: 549–624 s per run measured 2026-10-30 at load 40–54,
# against this Stop hook's 15 s timeout, so the harness cancelled the hook (17
# times in one session) and killed enforcement mid-pass on every turn. They now
# run DETACHED and single-flight: a second Stop while one pass is running skips.
if [ "$DRY" != "1" ]; then
  setsid bash -c '
    exec 9>"$1" || exit 0
    flock -n 9 || exit 0
    # shellcheck disable=SC1091
    source "$2/pool-lib.sh"
    pool_init
    record_all_slot_watermarks >/dev/null 2>&1 || true
    if [ "${CARGO_TARGET_POOL_NO_ENFORCE:-0}" != "1" ]; then
      enforce_pool 1 1 >/dev/null 2>&1 || true
    fi
  ' _ "$POOL_ROOT/.postflight.lock" "$SCRIPT_DIR" </dev/null >/dev/null 2>&1 &
fi

# Legacy opt-in cold-slot GC (superseded by enforce; kept for operators who
# pinned a custom GC window via env).
if [ -n "${CARGO_TARGET_POOL_POSTFLIGHT_GC_DAYS:-}" ]; then
  GC_DAYS="$CARGO_TARGET_POOL_POSTFLIGHT_GC_DAYS"
  # Find slot dirs (4 levels deep: family/<f>/<ws>/<profile>) whose
  # .last-build is older than GC_DAYS, and remove them.
  if [ -d "$POOL_ROOT/family" ]; then
    while IFS= read -r -d '' lb; do
      slot="$(dirname "$lb")"
      if [ "$DRY" != "1" ]; then
        rm -rf "$slot"
      fi
    done < <(find "$POOL_ROOT/family" -mindepth 4 -maxdepth 4 -type f \
      -name '.last-build' -mtime "+$GC_DAYS" -print0 2>/dev/null)
  fi
fi

exit 0
