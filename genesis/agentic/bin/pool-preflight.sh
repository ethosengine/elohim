#!/usr/bin/env bash
# pool-preflight.sh — SessionStart hook for the cargo target pool.
#
# 1. Initialize the pool root if absent.
# 2. Run worktree stewardship (clean merged worktrees, log orphans).
# 3. Detect family for $CLAUDE_PROJECT_DIR's worktree.
# 4. Emit ELOHIM CARGO TARGET POOL context block telling the agent
#    which CARGO_TARGET_DIR to set for native builds in this worktree.
#
# Hook input on stdin: { hook_event_name: "SessionStart", ... }
# Hook output on stdout (JSON):
#   { "hookSpecificOutput": { "hookEventName": "SessionStart",
#                             "additionalContext": "<context block>" } }
#
# Failures must not block the session — exit 0 even on error, optionally
# log to stderr (which Claude shows as a hook warning).
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/pool-lib.sh"

# Drain stdin (the hook input is JSON we don't need; ignore it).
cat >/dev/null 2>&1 || true

# Defensive: if jq is missing, do nothing rather than fail the session.
if ! command -v jq >/dev/null 2>&1; then
  exit 0
fi

# Skip if not in a workspace that looks like the elohim repo.
if ! [ -d "$POOL_PARENT_REPO/.git" ] && ! [ -f "$POOL_PARENT_REPO/.git" ]; then
  exit 0
fi

pool_init

# Run stewardship. Default: apply (not dry-run). Operator can disable
# the apply via CARGO_TARGET_POOL_PREFLIGHT_DRY=1 in shell or devfile.
DRY="${CARGO_TARGET_POOL_PREFLIGHT_DRY:-0}"

# Capture stewardship output (one tab-separated record per worktree).
STEWARD_OUTPUT="$(steward_all_worktrees "$DRY" 2>/dev/null || true)"

# Tally outcomes for the context block.
COUNT_REMOVED="$(echo "$STEWARD_OUTPUT" | awk -F'\t' '$2=="removed"' | wc -l | tr -d ' ')"
COUNT_ORPHAN="$(echo "$STEWARD_OUTPUT" | awk -F'\t' '$2=="orphan_logged"' | wc -l | tr -d ' ')"
COUNT_LEFT="$(echo "$STEWARD_OUTPUT" | awk -F'\t' '$2=="left"' | wc -l | tr -d ' ')"

# Family + slot map for the current project.
WT_ROOT="$(find_worktree_root "$POOL_PARENT_REPO" 2>/dev/null || echo "$POOL_PARENT_REPO")"
FAMILY="$(detect_family "$WT_ROOT")"
[ -z "$FAMILY" ] && FAMILY="unknown"
BRANCH="$(git -C "$WT_ROOT" rev-parse --abbrev-ref HEAD 2>/dev/null || echo unknown)"

build_slot_lines() {
  local family="$1" profile="${2:-dev}"
  local ws slot
  for ws in "${NATIVE_WORKSPACES[@]}"; do
    if [ -d "$WT_ROOT/$ws" ]; then
      slot="$(slot_path "$family" "$ws" "$profile")"
      printf '  - %-44s → %s\n' "$ws" "$slot"
    fi
  done
}

# Pool disk pressure.
POOL_BYTES="$(pool_disk_bytes 2>/dev/null || echo 0)"
POOL_HUMAN="$(human_bytes "$POOL_BYTES")"
PROJ_DISK="$(df -h "$POOL_PARENT_REPO" 2>/dev/null | awk 'NR==2 {print $5 " used (" $3 "/" $2 ")"}')"

# Other active families (siblings).
OTHER_FAMILIES="$(list_families | grep -vFx "$FAMILY" | tr '\n' ' ' | sed 's/ *$//')"

# Compose the context block.
CTX="$(cat <<EOF
ELOHIM CARGO TARGET POOL (preflight ran):

• Pool root: $POOL_ROOT  (current size: $POOL_HUMAN; volume: $PROJ_DISK)
• Family for this worktree: $FAMILY  (branch: $BRANCH)
• Other active families: ${OTHER_FAMILIES:-none}

For native cargo builds in this worktree, set CARGO_TARGET_DIR explicitly:
$(build_slot_lines "$FAMILY" dev)

For --release builds, swap the trailing 'dev' for 'release':
$(build_slot_lines "$FAMILY" release)

DNA / WASM workspaces (elohim/holochain/dna/*) — use plain cargo. Do NOT
redirect target/; hc dna pack canonicalizes ./target.

Worktree stewardship: $COUNT_REMOVED removed (merged-clean), $COUNT_ORPHAN logged orphan/dirty, $COUNT_LEFT left untouched (active or unclassified).

Operator commands:
  cargo-pool status               # families table
  cargo-pool steward --dry-run    # preview stewardship without applying
  cargo-pool key                  # print slot path for current PWD
  cargo-pool prune family <name>  # nuke a family slot tree (interactive)
  cargo-pool log -n 20            # tail the event log
EOF
)"

jq -n \
  --arg ctx "$CTX" \
  '{hookSpecificOutput: {hookEventName: "SessionStart", additionalContext: $ctx}}'
