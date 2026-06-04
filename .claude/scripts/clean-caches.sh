#!/usr/bin/env bash
# clean-caches.sh — SessionStart async disk reconciler.
#
# History: this used to rm -rf hardcoded in-tree target/debug paths. Those
# paths are now 12K pool stubs (builds live in /projects/.cargo-target-pool),
# so it delegates to the policy-driven reconciler instead:
#   genesis/agentic/bin/cargo-pool enforce  (policy: genesis/agentic/pool-policy.json)
# The enforce ladder is guarded (active families, live PIDs, flock'd slots
# are never touched) and idempotent — safe to run on every session start.
# Output keeps the [clean-caches] prefix contract used by the settings.json
# hook grep.
set -uo pipefail

PROJECT_ROOT="${CLAUDE_PROJECT_DIR:-/projects/elohim}"
VOLUME="/projects"

if [ "${CARGO_TARGET_POOL_NO_ENFORCE:-0}" = "1" ]; then
  echo "[clean-caches] CARGO_TARGET_POOL_NO_ENFORCE=1 — skipping"
  exit 0
fi

PCT=$(df "$VOLUME" 2>/dev/null | awk 'NR==2 {gsub(/%/,""); print $5}')
echo "[clean-caches] disk at ${PCT:-?}% — running policy enforce"

if [ -x "$PROJECT_ROOT/genesis/agentic/bin/cargo-pool" ]; then
  bash "$PROJECT_ROOT/genesis/agentic/bin/cargo-pool" enforce --yes 2>&1 \
    | sed 's/^\[enforce\] /[clean-caches] /' || true
else
  echo "[clean-caches] cargo-pool not found — skipping enforce"
fi

# pnpm store prune only under real pressure (slow, rarely the offender —
# measured 2026-06-04: node_modules totalled 1.3GB vs a 321GB cargo pool).
if [ "${PCT:-0}" -ge 85 ] && command -v pnpm >/dev/null 2>&1; then
  echo "[clean-caches] pressure >=85% — pnpm store prune"
  pnpm store prune --force >/dev/null 2>&1 && echo "[clean-caches] pnpm store pruned" || true
fi

NEW_PCT=$(df "$VOLUME" 2>/dev/null | awk 'NR==2 {gsub(/%/,""); print $5}')
echo "[clean-caches] done — disk now at ${NEW_PCT:-?}%"
