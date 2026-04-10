#!/usr/bin/env bash
# clean-caches.sh — triggered by SessionStart hook when /projects disk >= 90%
# Cleans Rust debug targets, Angular caches, and pnpm store orphans.
# Safe to run repeatedly; only acts when threshold is breached.

set -euo pipefail

VOLUME="/projects"
THRESHOLD=90
PROJECT_ROOT="/projects/elohim"

# Check current usage
USED_PCT=$(df "$VOLUME" | awk 'NR==2 {gsub(/%/,""); print $5}')

if [ "$USED_PCT" -lt "$THRESHOLD" ]; then
  echo "[clean-caches] disk at ${USED_PCT}% — below threshold, skipping"
  exit 0
fi

echo "[clean-caches] disk at ${USED_PCT}% >= ${THRESHOLD}% — cleaning..."

FREED=0

clean_dir() {
  local dir="$1"
  local label="$2"
  if [ -d "$dir" ]; then
    local size
    size=$(du -sm "$dir" 2>/dev/null | awk '{print $1}')
    rm -rf "$dir"
    echo "[clean-caches] removed $label (${size}MB)"
    FREED=$((FREED + size))
  fi
}

# --- Rust debug targets (largest offenders: 30GB + 14GB + 9.4GB) ---
clean_dir "$PROJECT_ROOT/elohim/elohim-storage/target/debug"   "elohim-storage/target/debug"
clean_dir "$PROJECT_ROOT/doorway/doorway-service/target/debug"  "doorway-service/target/debug"
clean_dir "$PROJECT_ROOT/steward/node/target/debug"             "steward/node/target/debug"

# --- Angular cache ---
clean_dir "$PROJECT_ROOT/app/elohim-app/.angular"               "elohim-app/.angular"
clean_dir "$PROJECT_ROOT/doorway/doorway-app/.angular"          "doorway-app/.angular"

# --- pnpm store: prune unreferenced packages ---
if command -v pnpm &>/dev/null; then
  echo "[clean-caches] running pnpm store prune..."
  pnpm store prune --force 2>/dev/null && echo "[clean-caches] pnpm store pruned" || true
fi

echo "[clean-caches] done — freed ~${FREED}MB total"

# Report new usage
NEW_PCT=$(df "$VOLUME" | awk 'NR==2 {gsub(/%/,""); print $5}')
echo "[clean-caches] disk now at ${NEW_PCT}%"
