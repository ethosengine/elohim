#!/usr/bin/env bash
# pool-preflight.sh — SessionStart hook for the cargo target pool.
#
# 1. Initialize the pool root if absent.
# 2. Refresh stewardship off-path; print the last completed report with its age.
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

# Default: apply. The dry-run report has its own cache, never an applied count.
DRY="${CARGO_TARGET_POOL_PREFLIGHT_DRY:-0}"
CACHE_KEY="$(printf '%s\0' "$POOL_PARENT_REPO" "$POOL_WORKTREES_DIR" "$DRY" | sha256sum)"
CACHE="$POOL_ROOT/.preflight-${CACHE_KEY%% *}.txt"

refresh_report() {
  pool_init >/dev/null 2>&1 || return 1
  exec 9>"$POOL_ROOT/.preflight.lock" || return 1
  flock -n 9 || return 0
  local at now
  at="$(stat -c %Y "$CACHE" 2>/dev/null || echo 0)"
  now="$(date +%s)"
  if [ -s "$CACHE" ] && (( now >= at && now - at < 120 )); then return 0; fi

  # Capture stewardship output (one tab-separated record per worktree).
  STEWARD_OUTPUT="$(steward_all_worktrees "$DRY" 2>/dev/null)" || return 1

  # Tally outcomes for the context block.
  COUNT_REMOVED="$(echo "$STEWARD_OUTPUT" | awk -F'\t' '$2=="removed"' | wc -l | tr -d ' ')"
  COUNT_ORPHAN="$(echo "$STEWARD_OUTPUT" | awk -F'\t' '$2=="orphan_logged"' | wc -l | tr -d ' ')"
  COUNT_LEFT="$(echo "$STEWARD_OUTPUT" | awk -F'\t' '$2=="left"' | wc -l | tr -d ' ')"

  POOL_BYTES="$(pool_disk_bytes 2>/dev/null)" || return 1
  POOL_HUMAN="$(human_bytes "$POOL_BYTES")"
  OTHER_FAMILIES="$(list_families | tr '\n' ' ' | sed 's/ *$//')" || return 1

  # Legacy target/ banner — target/ dirs outside the pool that the operator
  # could reclaim. Only surface if non-zero native recoverable.
  LEGACY_BYTES="$(legacy_native_bytes 2>/dev/null)" || return 1
  LEGACY_BANNER=""
  if [ "${LEGACY_BYTES:-0}" -gt 268435456 ]; then  # >256MB worth noting
    LEGACY_BANNER="$(cat <<BANNER

!  Legacy target/ dirs outside pool (native, recoverable): $(human_bytes "$LEGACY_BYTES")
   Review: bash genesis/agentic/bin/cargo-pool legacy-targets
   Clean:  bash genesis/agentic/bin/cargo-pool legacy-targets --clean --yes
BANNER
)"
  fi

  # --------------------------------------------------------------------------
  # Stale-incremental banner — same surface form as legacy.
  STALE_BYTES="$(stale_incremental_bytes 3 2>/dev/null)" || return 1
  STALE_BANNER=""
  if [ "${STALE_BYTES:-0}" -gt 1073741824 ]; then  # >1GB worth offering
    STALE_BANNER="$(cat <<BANNER

!  Stale incremental hash dirs (>3d): $(human_bytes "$STALE_BYTES")
   Clean: bash genesis/agentic/bin/cargo-pool prune --stale-incrementals --yes
BANNER
)"
  fi

  # --------------------------------------------------------------------------
  # JS-cache banner — node_modules drift + old .angular caches. Combined into
  # one stanza because they're the same shape (different reclaim commands).
  NM_STALE_BYTES="$(node_modules_stale_bytes 2>/dev/null)" || return 1
  ANG_STALE_BYTES="$(angular_cache_stale_bytes 7 2>/dev/null)" || return 1
  JS_TOTAL=$((${NM_STALE_BYTES:-0} + ${ANG_STALE_BYTES:-0}))
  JS_BANNER=""
  if [ "$JS_TOTAL" -gt 2147483648 ]; then  # >2GB combined worth surfacing
    JS_LINES=""
    if [ "${NM_STALE_BYTES:-0}" -gt 268435456 ]; then  # >256MB
      JS_LINES="${JS_LINES}   Stale node_modules (lockfile drift): $(human_bytes "$NM_STALE_BYTES")
   Clean: bash genesis/agentic/bin/cargo-pool node-modules --clean --yes
"
    fi
    if [ "${ANG_STALE_BYTES:-0}" -gt 268435456 ]; then
      JS_LINES="${JS_LINES}   Old .angular caches (>7d): $(human_bytes "$ANG_STALE_BYTES")
   Clean: bash genesis/agentic/bin/cargo-pool angular-cache --clean --yes
"
    fi
    JS_BANNER="$(cat <<BANNER

!  JS-cache pressure: $(human_bytes "$JS_TOTAL") recoverable
$JS_LINES
BANNER
)"
  fi

  # --------------------------------------------------------------------------
  # Pre-dispatch budget banner — fires only when a worktree was created since
  # the previous preflight. Compares free disk against an estimate ×1.5 of the
  # cold cargo invocation in that worktree's slot. After computing, touch the
  # marker so next session sees only what's new.
  BUDGET_BANNER=""
  NEW_WTS="$(worktrees_newer_than_marker)" || return 1
  if [ -n "$NEW_WTS" ]; then
    BUDGET_LINES=""
    while IFS= read -r nwt; do
      [ -d "$nwt" ] || continue
      budget="$(disk_budget_for_worktree "$nwt")" || return 1
      est="$(echo "$budget" | cut -f1)"
      free="$(echo "$budget" | cut -f2)"
      verdict="$(echo "$budget" | cut -f3)"
      BUDGET_LINES="${BUDGET_LINES}  ${verdict}: $(basename "$nwt") (est ~${est}G, free ${free}G)
"
    done <<<"$NEW_WTS"
    if [ -n "$BUDGET_LINES" ]; then
      if echo "$BUDGET_LINES" | grep -q '^  short:'; then
        BUDGET_BANNER="$(cat <<BANNER

!! BUDGET SHORT for newly-created worktree(s) — first build risks disk fill:
$BUDGET_LINES   Clean before dispatching:
   bash genesis/agentic/bin/cargo-pool prune --stale-incrementals --yes
   bash genesis/agentic/bin/cargo-pool legacy-targets --clean --yes
BANNER
)"
      elif echo "$BUDGET_LINES" | grep -q '^  tight:'; then
        BUDGET_BANNER="$(cat <<BANNER

!  Budget tight for newly-created worktree(s):
$BUDGET_LINES   Consider cleanup before parallel waves.
BANNER
)"
      fi
    fi
  fi

  local removed="removed" logged="logged" mode=""
  if [ "$DRY" = "1" ]; then removed="would remove"; logged="would log"; mode=" (dry-run plan)"; fi
  local tmp="$CACHE.$$.tmp"
  trap 'rm -f -- "$CACHE.$$.tmp"' EXIT
  cat >"$tmp" <<EOF || return 1
Worktree stewardship${mode}: $COUNT_REMOVED $removed (merged-clean), $COUNT_ORPHAN $logged orphan/dirty, $COUNT_LEFT left untouched (active or unclassified).
• Pool size: $POOL_HUMAN
• Pool families: ${OTHER_FAMILIES:-none}${LEGACY_BANNER}${STALE_BANNER}${JS_BANNER}${BUDGET_BANNER}
EOF
  mv -f -- "$tmp" "$CACHE" || return 1
  touch "$(pool_last_preflight)" 2>/dev/null || true
}

if [ "${1:-}" = "--refresh" ]; then
  refresh_report
  exit 0
fi

# The reported pass exceeded 120 s (pool-postflight walks: 549–624 s). Here at
# load 17–28, du alone took 3.64 s and node_modules scanning took 8.56 s. All walks
# run detached under flock -n; the 15 s hook only reads a completed report.
CACHE_AT="$(stat -c %Y "$CACHE" 2>/dev/null || echo 0)"
NOW="$(date +%s)"
AGE=$((NOW - CACHE_AT))
STEWARD_REPORT="Worktree stewardship: refresh pending (no completed pass)."
if [ -s "$CACHE" ] && (( AGE >= 0 )); then
  STEWARD_REPORT="Worktree stewardship: last completed pass ${AGE}s ago.
$(cat "$CACHE")"
fi
if ! [ -s "$CACHE" ] || (( AGE < 0 || AGE >= 120 )); then
  if [ -s "$CACHE" ] && (( AGE >= 0 )); then
    STEWARD_REPORT="Worktree stewardship: refresh pending (last pass ${AGE}s ago).
$(cat "$CACHE")"
  fi
  setsid bash "$SCRIPT_DIR/pool-preflight.sh" --refresh </dev/null >/dev/null 2>&1 &
fi

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

PROJ_DISK="$(df -h "$POOL_PARENT_REPO" 2>/dev/null | awk 'NR==2 {print $5 " used (" $3 "/" $2 ")"}')"

# --------------------------------------------------------------------------
# Disk-pressure banner — substrate-agnostic. 80% warns, 90% screams.
# Same hook will serve future pnpm bloat signals (disk is disk).
DISK_PCT="$(disk_pct_used)"
DISK_STATUS="$(disk_status_for_pct "${DISK_PCT:-0}")"
DISK_SUMMARY="$(disk_human_summary)"
DISK_BANNER=""
case "$DISK_STATUS" in
  critical)
    DISK_BANNER="$(cat <<BANNER

!! CRITICAL DISK PRESSURE: $DISK_SUMMARY (${DISK_PCT}% used). RUN CLEANUP NOW:
 bash genesis/agentic/bin/cargo-pool enforce --yes
 (policy-driven ladder: genesis/agentic/pool-policy.json — the SessionStart
  async hook also runs this; heavy cargo is DENIED at the hard ceiling)
BANNER
)"
    ;;
  warn)
    DISK_BANNER="$(cat <<BANNER

!  Disk pressure: $DISK_SUMMARY (${DISK_PCT}% used). Reclaim per policy:
 bash genesis/agentic/bin/cargo-pool enforce --yes
BANNER
)"
    ;;
esac

# Compose the context block.
CTX="$(cat <<EOF
ELOHIM CARGO TARGET POOL:

• Pool root: $POOL_ROOT  (volume: $PROJ_DISK)
• Family for this worktree: $FAMILY  (branch: $BRANCH)
${DISK_BANNER}

$STEWARD_REPORT

For native cargo builds in this worktree, set CARGO_TARGET_DIR explicitly:
$(build_slot_lines "$FAMILY" dev)

For --release builds, swap the trailing 'dev' for 'release':
$(build_slot_lines "$FAMILY" release)

DNA / WASM workspaces (elohim/holochain/dna/*) — use plain cargo. Do NOT
redirect target/; hc dna pack canonicalizes ./target.

Operator commands:
  cargo-pool enforce [--yes]      # policy reconciler (pool-policy.json) — dry-run by default
  cargo-pool status               # families table + pressure indicators
  cargo-pool steward --dry-run    # preview stewardship without applying
  cargo-pool key                  # print slot path for current PWD
  cargo-pool estimate             # rough GB cost of next cargo in this worktree
  cargo-pool legacy-targets       # find target/ dirs outside the pool
  cargo-pool node-modules         # find stale node_modules (lockfile drift)
  cargo-pool angular-cache        # find old .angular caches
  cargo-pool prune --stale-incrementals --yes  # GC old incremental hash dirs
  cargo-pool prune family <name>  # nuke a family slot tree (interactive)
  cargo-pool orphans              # list orphan + uncommitted-dirty worktrees
  cargo-pool log -n 20            # tail the event log
EOF
)"

jq -n \
  --arg ctx "$CTX" \
  '{hookSpecificOutput: {hookEventName: "SessionStart", additionalContext: $ctx}}'
