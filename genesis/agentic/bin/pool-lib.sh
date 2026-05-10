#!/usr/bin/env bash
# Shared helpers for the cargo target pool's pre/post-flight hooks and
# operator CLI. Sourced, not executed.
#
# Design: genesis/docs/plans/cargo-target-pool-design.md
#
# Architecture (post user feedback): hooks-only. No wrapper, no per-cargo
# holder file. Cargo's intra-target advisory lock handles concurrency
# within a slot. The hooks handle worktree stewardship + emit the slot
# path to the agent so they know which CARGO_TARGET_DIR to set.

POOL_ROOT="${CARGO_TARGET_POOL_ROOT:-/projects/.cargo-target-pool}"
POOL_PARENT_REPO="${POOL_PARENT_REPO:-${CLAUDE_PROJECT_DIR:-/projects/elohim}}"
POOL_WORKTREES_DIR="${POOL_WORKTREES_DIR:-$POOL_PARENT_REPO/.claude/worktrees}"

pool_root() { echo "$POOL_ROOT"; }
pool_log_path() { echo "$POOL_ROOT/pool.log"; }
pool_orphan_log() { echo "$POOL_ROOT/orphan-worktrees.tsv"; }
pool_steward_log() { echo "$POOL_ROOT/steward.log"; }

pool_init() {
  mkdir -p "$POOL_ROOT/family"
  [ -f "$(pool_log_path)" ] || : > "$(pool_log_path)"
  [ -f "$(pool_orphan_log)" ] || : > "$(pool_orphan_log)"
  [ -f "$(pool_steward_log)" ] || : > "$(pool_steward_log)"
}

now_iso() { date -u +%Y-%m-%dT%H:%M:%SZ; }

# Walk up from $1 finding the dir that contains .git (file or dir).
find_worktree_root() {
  local d
  d="$(cd "${1:-$PWD}" 2>/dev/null && pwd -P)" || return 1
  while [ "$d" != "/" ] && [ -n "$d" ]; do
    [ -e "$d/.git" ] && { echo "$d"; return 0; }
    d="$(dirname "$d")"
  done
  return 1
}

# Family heuristic: env override → .family file → branch prefix → fallback.
detect_family() {
  local wt="$1"
  if [ -n "${CARGO_TARGET_POOL_FAMILY:-}" ]; then
    echo "$CARGO_TARGET_POOL_FAMILY"; return
  fi
  if [ -f "$wt/.family" ]; then
    local v; v="$(tr -d '[:space:]' < "$wt/.family")"
    [ -n "$v" ] && { echo "$v"; return; }
  fi
  local branch
  branch="$(git -C "$wt" rev-parse --abbrev-ref HEAD 2>/dev/null || echo)"
  if [ -z "$branch" ] || [ "$branch" = "HEAD" ]; then
    basename "$wt" | tr '[:upper:]' '[:lower:]' | tr -c 'a-z0-9-' '-' | sed 's/-*$//' || echo unknown
    return
  fi
  case "$branch" in
    feat/*|feature/*|fix/*|chore/*) branch="${branch#*/}" ;;
    worktree-*) branch="${branch#worktree-}" ;;
  esac
  echo "${branch%%[-/]*}" | tr '[:upper:]' '[:lower:]'
}

# Convert workspace-relative path "elohim/elohim-storage" → "elohim__elohim-storage"
flatten_path() {
  echo "$1" | tr '/' '_' | sed 's/__*/__/g' | sed 's/^_*//' | sed 's/_*$//'
}

# Slot path: $POOL_ROOT/family/<family>/<flat-ws-rel>/<profile>/
slot_path() {
  local family="$1" ws_rel="$2" profile="${3:-dev}"
  local flat; flat="$(flatten_path "$ws_rel")"
  echo "$POOL_ROOT/family/$family/$flat/$profile"
}

# Native cargo workspaces in this repo whose target/ should land in the pool.
# WASM/DNA workspaces are excluded by design (hc dna pack canonicalizes
# ./target — moving it breaks the build).
NATIVE_WORKSPACES=(
  "elohim/elohim-storage"
  "doorway/doorway-service"
  "steward/node"
  "elohim/holochain/tests/sweettest"
  "crates"
)

# Classify a worktree's branch: cleaned | cleaned-dirty | orphan | active |
#                                unknown | broken | missing
classify_worktree() {
  local wt="$1" branch="$2"
  if [ -z "$wt" ] || ! [ -d "$wt" ]; then echo missing; return; fi
  if ! git -C "$wt" rev-parse HEAD >/dev/null 2>&1; then echo broken; return; fi
  if [ -z "$branch" ]; then echo unknown; return; fi
  if ! [ -d "$POOL_PARENT_REPO" ]; then echo unknown; return; fi

  # Merged into dev?
  local merged
  if merged="$(git -C "$POOL_PARENT_REPO" branch --merged dev 2>/dev/null)"; then
    # `git branch --merged` uses `* ` for current, `+ ` for checked-out-elsewhere,
    # `  ` for others. Strip them all.
    if echo "$merged" | sed -E 's/^[*+ ]+ ?//' | grep -Fxq "$branch"; then
      local dirty
      dirty="$(git -C "$wt" status --porcelain 2>/dev/null | head -1)"
      if [ -n "$dirty" ]; then echo cleaned-dirty; else echo cleaned; fi
      return
    fi
  fi

  # Exists on origin?
  if git -C "$POOL_PARENT_REPO" ls-remote --heads origin "$branch" 2>/dev/null \
      | grep -q "refs/heads/$branch"; then
    echo active
  else
    # Branch is NOT merged AND we don't see it on origin. We cannot
    # distinguish "deleted upstream" (true orphan) from "never pushed,
    # still being worked on locally" without external signal. Conservative
    # default: classify as unknown and leave the worktree alone.
    echo unknown
  fi
}

# Steward one worktree: classify, act, log.
# Output format on stdout: tab-separated record per worktree acted on.
#   <classification> <action> <worktree_path> <branch>
# Action is one of: removed | orphan_logged | left
# In dry_run mode, no filesystem mutations — just print the planned action.
steward_worktree() {
  local wt="$1" dry_run="${2:-0}"
  local branch
  branch="$(git -C "$wt" rev-parse --abbrev-ref HEAD 2>/dev/null || echo)"
  local cls action
  cls="$(classify_worktree "$wt" "$branch")"

  case "$cls" in
    cleaned)        action="removed" ;;
    cleaned-dirty)  action="orphan_logged" ;;
    active|unknown|broken|missing) action="left" ;;
    *)              action="left" ;;
  esac

  if [ "$dry_run" != "1" ] && [ "$action" = "removed" ]; then
    git -C "$POOL_PARENT_REPO" worktree remove --force "$wt" >/dev/null 2>&1 || action="left"
  fi
  if [ "$dry_run" != "1" ] && [ "$action" = "orphan_logged" ]; then
    printf '%s\t%s\t%s\t%s\n' "$(now_iso)" "$wt" "$branch" "$cls" >> "$(pool_orphan_log)"
  fi
  if [ "$dry_run" != "1" ]; then
    log_steward_event "$cls" "$action" "$wt" "$branch"
  fi

  printf '%s\t%s\t%s\t%s\n' "$cls" "$action" "$wt" "$branch"
}

# Walk every worktree in $POOL_WORKTREES_DIR and steward.
steward_all_worktrees() {
  local dry_run="${1:-0}"
  [ -d "$POOL_WORKTREES_DIR" ] || return 0
  local wt
  for wt in "$POOL_WORKTREES_DIR"/*; do
    [ -d "$wt" ] || continue
    steward_worktree "$wt" "$dry_run"
  done
}

# Append one JSON line to the steward event log.
log_steward_event() {
  local cls="$1" action="$2" wt="$3" branch="$4"
  local line
  line="$(jq -nc \
    --arg ts "$(now_iso)" --arg event steward_clean \
    --arg cls "$cls" --arg action "$action" \
    --arg wt "$wt" --arg branch "$branch" \
    '{ts:$ts,event:$event,classification:$cls,action:$action,worktree_path:$wt,branch:$branch}')"
  echo "$line" >> "$(pool_steward_log)" 2>/dev/null || true
}

# Compute disk usage of a directory in bytes (du -sb), or 0 if missing.
dir_disk_bytes() {
  local d="$1"
  [ -d "$d" ] || { echo 0; return; }
  du -sb "$d" 2>/dev/null | awk '{print $1}'
}

# Total pool disk usage in bytes.
pool_disk_bytes() {
  dir_disk_bytes "$POOL_ROOT"
}

# Bytes → human-readable.
human_bytes() {
  local b="${1:-0}"
  if [ "$b" -ge 1073741824 ]; then
    awk -v b="$b" 'BEGIN{printf "%.1fG", b/1073741824}'
  elif [ "$b" -ge 1048576 ]; then
    awk -v b="$b" 'BEGIN{printf "%.1fM", b/1048576}'
  elif [ "$b" -ge 1024 ]; then
    awk -v b="$b" 'BEGIN{printf "%.1fK", b/1024}'
  else
    echo "${b}B"
  fi
}

# List active families (subdirs of $POOL_ROOT/family).
list_families() {
  local fr="$POOL_ROOT/family"
  [ -d "$fr" ] || return 0
  find "$fr" -mindepth 1 -maxdepth 1 -type d -printf '%f\n' 2>/dev/null | sort
}

# For one family, list slot paths (3rd level under family/<family>/<ws>/<profile>).
list_slots_for_family() {
  local family="$1"
  local fd="$POOL_ROOT/family/$family"
  [ -d "$fd" ] || return 0
  find "$fd" -mindepth 2 -maxdepth 2 -type d 2>/dev/null | sort
}
