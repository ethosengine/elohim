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

# Optional cold-slot GC. Off by default; opt-in via env to avoid
# surprising operators with disk activity at session-end.
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
