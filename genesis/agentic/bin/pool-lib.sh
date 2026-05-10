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
#
# Active-subagent override: any worktree with a live process whose CWD is
# inside the worktree path is left untouched regardless of git state. This
# closes the race where a just-dispatched agent (zero commits ahead, no dirty
# files yet) was misclassified as `cleaned` and removed mid-launch.
steward_worktree() {
  local wt="$1" dry_run="${2:-0}"
  local branch
  branch="$(git -C "$wt" rev-parse --abbrev-ref HEAD 2>/dev/null || echo)"

  # Activity check first — overrides everything else.
  local active_pids
  active_pids="$(find_active_subagent_pids "$wt" 2>/dev/null | head -3 | tr '\n' ',' | sed 's/,$//')"
  if [ -n "$active_pids" ]; then
    local ctx; ctx="pids=$active_pids"
    if [ "$dry_run" != "1" ]; then
      log_steward_event "active-subagent" "left" "$wt" "$branch" "$ctx"
    fi
    printf '%s\t%s\t%s\t%s\n' "active-subagent" "left" "$wt" "$branch"
    return
  fi

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
    # Dedupe: only append if (wt, branch, cls) isn't already logged. The same
    # merged-dirty worktrees re-trigger on every preflight; without this the
    # log bloats by 3 lines/preflight × ~50 preflights/day. Operator-readable
    # signal beats append-only purity here.
    if ! awk -F'\t' -v wt="$wt" -v br="$branch" -v cls="$cls" \
        '$2==wt && $3==br && $4==cls {exit 0} END {exit 1}' \
        "$(pool_orphan_log)" 2>/dev/null; then
      printf '%s\t%s\t%s\t%s\n' "$(now_iso)" "$wt" "$branch" "$cls" >> "$(pool_orphan_log)"
    fi
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

# Append one JSON line to the steward event log. Optional 5th arg is a free-form
# context string folded into the record under "context".
log_steward_event() {
  local cls="$1" action="$2" wt="$3" branch="$4" ctx="${5:-}"
  local line
  line="$(jq -nc \
    --arg ts "$(now_iso)" --arg event steward_clean \
    --arg cls "$cls" --arg action "$action" \
    --arg wt "$wt" --arg branch "$branch" --arg ctx "$ctx" \
    '{ts:$ts,event:$event,classification:$cls,action:$action,worktree_path:$wt,branch:$branch} as $b
     | if ($ctx | length) > 0 then $b + {context:$ctx} else $b end')"
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

# ---------------------------------------------------------------------------
# Disk-pressure probe — substrate-agnostic. Reports on the volume containing
# $POOL_PARENT_REPO. Used by preflight for warn/critical banners. Also used by
# operator's `cargo-pool status`. The same probe will serve a future pnpm
# bloat hook — disk pressure is one signal regardless of source.

# Echo the integer percent-used for the volume containing $1 (default
# POOL_PARENT_REPO). Empty string if df fails.
disk_pct_used() {
  df -P "${1:-$POOL_PARENT_REPO}" 2>/dev/null \
    | awk 'NR==2 {gsub("%","",$5); print $5}'
}

# Echo human-readable "<used> used / <avail> free / <size> total".
disk_human_summary() {
  df -h "${1:-$POOL_PARENT_REPO}" 2>/dev/null \
    | awk 'NR==2 {printf "%s used / %s free / %s total", $3, $4, $2}'
}

# Echo a status keyword (ok | warn | critical) given a percent-used integer.
disk_status_for_pct() {
  local pct="${1:-0}"
  [ -z "$pct" ] && { echo unknown; return; }
  if [ "$pct" -ge 90 ]; then echo critical
  elif [ "$pct" -ge 80 ]; then echo warn
  else echo ok
  fi
}

# ---------------------------------------------------------------------------
# Legacy target-dir sweep — finds target/ dirs that should be in the pool but
# aren't, plus target/ dirs the pool intentionally exempts (WASM/DNA/typesrc)
# so the operator can see the full picture.
#
# Classifies each target/ dir as:
#   native     — under one of NATIVE_WORKSPACES; safe to remove (pool rebuilds)
#   wasm       — under elohim/holochain/dna/* or sibling of dna.yaml/happ.yaml;
#                MUST keep (hc dna pack canonicalizes ./target)
#   typesrc    — under elohim/sdk/domains/*/types/*; tiny TS codegen artifacts;
#                leave alone (rebuilt cheaply, not on the disk-pressure path)
#   unknown    — anything else; report, don't auto-clean
#
# Output format on stdout (tab-separated): <classification> <bytes> <path>
list_legacy_targets() {
  local roots=()
  roots+=("$POOL_PARENT_REPO")
  if [ -d "$POOL_WORKTREES_DIR" ]; then
    local wt
    for wt in "$POOL_WORKTREES_DIR"/*; do
      [ -d "$wt" ] && roots+=("$wt")
    done
  fi
  local root
  for root in "${roots[@]}"; do
    find "$root" -maxdepth 6 -type d -name target \
      -not -path "*/node_modules/*" \
      -not -path "$POOL_ROOT/*" \
      -not -path "*/.git/*" \
      -print 2>/dev/null
  done | sort -u | while IFS= read -r t; do
    local cls bytes
    cls="$(classify_legacy_target "$t")"
    bytes="$(dir_disk_bytes "$t")"
    printf '%s\t%s\t%s\n' "$cls" "$bytes" "$t"
  done
}

# Classify one target/ dir. See list_legacy_targets for categories.
classify_legacy_target() {
  local t="$1"
  local parent; parent="$(dirname "$t")"
  case "$parent" in
    */elohim/holochain/dna|*/elohim/holochain/dna/*) echo wasm; return ;;
    */elohim/sdk/domains/*/types|*/elohim/sdk/domains/*/types/*) echo typesrc; return ;;
  esac
  # cdylib parent? → WASM workspace.
  if [ -f "$parent/Cargo.toml" ] && grep -qE 'crate-type *= *\[[^]]*"cdylib"' "$parent/Cargo.toml" 2>/dev/null; then
    echo wasm; return
  fi
  # dna.yaml/happ.yaml adjacent? → WASM.
  if [ -f "$parent/dna.yaml" ] || [ -f "$parent/happ.yaml" ]; then
    echo wasm; return
  fi
  # Native workspace match? — parent path ends with one of NATIVE_WORKSPACES.
  local ws
  for ws in "${NATIVE_WORKSPACES[@]}"; do
    case "$parent" in *"/$ws"|*"/$ws/") echo native; return ;; esac
  done
  echo unknown
}

# Sum bytes of all "native" legacy targets — i.e. recoverable disk if cleaned.
legacy_native_bytes() {
  list_legacy_targets 2>/dev/null \
    | awk -F'\t' '$1=="native" {sum+=$2} END {print sum+0}'
}

# Remove all `native` legacy target/ dirs (the safe-to-clean class). Echo each
# removal on stdout. Used by `cargo-pool legacy-targets --clean`.
clean_legacy_native_targets() {
  local dry="${1:-0}"
  list_legacy_targets 2>/dev/null \
    | awk -F'\t' '$1=="native" {print $0}' \
    | while IFS=$'\t' read -r cls bytes path; do
      if [ "$dry" = "1" ]; then
        printf 'would-remove\t%s\t%s\n' "$(human_bytes "$bytes")" "$path"
      else
        rm -rf "$path" 2>/dev/null && \
          printf 'removed\t%s\t%s\n' "$(human_bytes "$bytes")" "$path"
      fi
    done
}

# ---------------------------------------------------------------------------
# Active-subagent detection — scan /proc/*/cwd for any process whose CWD is
# inside $1 (the worktree path). Echo one PID per line. Empty if none.
#
# Closes the steward-vs-just-dispatched-agent race observed 2026-05-10: a
# wave-2 worktree was classified `cleaned` 60s after creation because it had
# zero commits ahead and no dirty files yet — but an agent's CWD was inside.
find_active_subagent_pids() {
  local wt="$1"
  [ -d "$wt" ] || return 0
  local resolved
  resolved="$(cd "$wt" 2>/dev/null && pwd -P)" || return 0
  local pid_dir pid cwd
  for pid_dir in /proc/[0-9]*; do
    pid="${pid_dir##*/}"
    cwd="$(readlink "$pid_dir/cwd" 2>/dev/null)" || continue
    case "$cwd" in
      "$resolved"|"$resolved"/*) echo "$pid" ;;
    esac
  done
}

# Echo the first arg of /proc/<pid>/cmdline (the executable name), truncated.
proc_cmd_snippet() {
  local pid="$1"
  local raw
  raw="$(tr '\0' ' ' </proc/"$pid"/cmdline 2>/dev/null | head -c 80)"
  echo "${raw:-?}"
}

# ---------------------------------------------------------------------------
# Stale-incremental pruner — Cargo doesn't GC its incremental/ subdirs when
# branches get merged; the per-crate hash dir persists indefinitely with the
# fingerprints of a now-deleted branch's source diff. Slot growth across Wave
# 1→5 was 16G→81G; most was stale incrementals.
#
# Layout under a slot:
#   $slot/{debug,release}/incremental/<crate>-<hash>/
# We prune the per-crate hash dirs whose mtime is older than $days.
# Naive — branch-aware variant deferred.

# Echo tab-separated <bytes> <path> for each stale incremental hash dir.
list_stale_incrementals() {
  local days="${1:-3}"
  [ -d "$POOL_ROOT/family" ] || return 0
  find "$POOL_ROOT/family" -mindepth 6 -maxdepth 6 -type d \
    -path '*/incremental/*' -mtime "+$days" -print 2>/dev/null \
    | while IFS= read -r d; do
      printf '%s\t%s\n' "$(dir_disk_bytes "$d")" "$d"
    done
}

# Total bytes recoverable by pruning stale incrementals older than $days.
stale_incremental_bytes() {
  list_stale_incrementals "${1:-3}" | awk -F'\t' '{sum+=$1} END {print sum+0}'
}

# Prune stale incremental hash dirs older than $days. Dry-run by default;
# pass yes=1 to apply. Echos one line per entry.
prune_stale_incrementals() {
  local days="${1:-3}" yes="${2:-0}"
  list_stale_incrementals "$days" | while IFS=$'\t' read -r bytes path; do
    if [ "$yes" = "1" ]; then
      rm -rf "$path" 2>/dev/null && \
        printf 'removed\t%s\t%s\n' "$(human_bytes "$bytes")" "$path"
    else
      printf 'would-remove\t%s\t%s\n' "$(human_bytes "$bytes")" "$path"
    fi
  done
}

# ---------------------------------------------------------------------------
# Slot watermarks — high-water mark of observed slot size, in bytes. Written
# by pool-postflight on each session Stop, monotonically rises. The estimator
# prefers HWM × safety factor when available; falls back to hardcoded 10G/3G
# when a slot has never been observed.
#
# Why HWM (not average / last): the cost we care about is "will the next
# build fit," which is bounded by peak. The HWM grows as builds get bigger
# (deps added, new workspaces), and survives across `cargo clean` because we
# only ever overwrite with max.

watermark_file() { echo "$1/.peak-size"; }

# Read HWM for a slot path, in bytes. Echo 0 if missing.
read_slot_watermark() {
  local f; f="$(watermark_file "$1")"
  [ -f "$f" ] && cat "$f" 2>/dev/null || echo 0
}

# Record current du into HWM, keeping the maximum. Quiet on no-op.
record_slot_watermark() {
  local slot="$1"
  [ -d "$slot" ] || return 0
  local cur; cur="$(dir_disk_bytes "$slot")"
  [ "${cur:-0}" -eq 0 ] && return 0
  local f; f="$(watermark_file "$slot")"
  local prev; prev="$(read_slot_watermark "$slot")"
  if [ "$cur" -gt "${prev:-0}" ]; then
    echo "$cur" > "$f"
  fi
}

# Walk every slot under POOL_ROOT/family/*/*/{dev,release} and update HWMs.
# Called by pool-postflight.sh on Stop.
record_all_slot_watermarks() {
  [ -d "$POOL_ROOT/family" ] || return 0
  local slot
  while IFS= read -r slot; do
    record_slot_watermark "$slot"
  done < <(find "$POOL_ROOT/family" -mindepth 3 -maxdepth 3 -type d \
    \( -name dev -o -name release \) 2>/dev/null)
}

# Estimate cost in GB for next cargo invocation in a worktree's slot(s).
# Strategy:
#   1. If a slot already exists AND has a HWM file → ceil(HWM × 1.2) / 1G.
#   2. Else if slot exists (warm, no HWM yet) → 3G (hardcoded).
#   3. Else (cold slot) → look for sibling family's HWM for the same workspace
#      as a cross-family reference (most cargo workspaces have similar cost
#      across families). Use that × 1.2 if found.
#   4. Else hardcoded 10G cold.
# Output: single integer GB.
estimate_slot_cost_gb() {
  local wt="$1"
  local family; family="$(detect_family "$wt")"
  local total=0 ws slot bytes
  for ws in "${NATIVE_WORKSPACES[@]}"; do
    [ -d "$wt/$ws" ] || continue
    slot="$(slot_path "$family" "$ws" dev)"
    bytes=0
    if [ -d "$slot" ]; then
      bytes="$(read_slot_watermark "$slot")"
      if [ "${bytes:-0}" -eq 0 ]; then bytes=$((3 * 1073741824)); fi
    else
      # Cold slot: try sibling families' HWM for the same workspace.
      local sib_hwm; sib_hwm="$(cross_family_watermark "$ws")"
      if [ "${sib_hwm:-0}" -gt 0 ]; then
        bytes="$sib_hwm"
      else
        bytes=$((10 * 1073741824))
      fi
    fi
    # Apply +20% safety margin and convert to GB ceil.
    local gb=$(( (bytes * 12 / 10 + 1073741823) / 1073741824 ))
    total=$((total + gb))
  done
  echo "$total"
}

# Echo the maximum HWM across all families for a given workspace_rel_path
# (e.g. "elohim/elohim-storage"). Used to seed cold-slot estimates from
# observed peak in any other family. Empty/0 if no observation exists.
cross_family_watermark() {
  local ws="$1"
  local flat; flat="$(flatten_path "$ws")"
  local fr="$POOL_ROOT/family"
  [ -d "$fr" ] || { echo 0; return; }
  local best=0 wm
  local fam
  for fam in $(list_families); do
    local slot="$fr/$fam/$flat/dev"
    [ -d "$slot" ] || continue
    wm="$(read_slot_watermark "$slot")"
    if [ "${wm:-0}" -gt "$best" ]; then best="$wm"; fi
  done
  echo "$best"
}

# Disk budget for a worktree — checks free space against estimate × 1.5.
# Echos "<estimate_gb>\t<free_gb>\t<verdict>" where verdict is ok|tight|short.
disk_budget_for_worktree() {
  local wt="$1"
  local est; est="$(estimate_slot_cost_gb "$wt")"
  local free_kb
  free_kb="$(df -P "$POOL_PARENT_REPO" 2>/dev/null | awk 'NR==2 {print $4}')"
  local free_gb=$((${free_kb:-0} / 1024 / 1024))
  local verdict
  local needed=$((est * 3 / 2))
  if [ "$free_gb" -ge "$needed" ]; then
    verdict=ok
  elif [ "$free_gb" -ge "$est" ]; then
    verdict=tight
  else
    verdict=short
  fi
  printf '%s\t%s\t%s\n' "$est" "$free_gb" "$verdict"
}

# Marker file used to detect "worktrees created since last preflight."
pool_last_preflight() { echo "$POOL_ROOT/.last-preflight"; }

# Echo paths of worktrees newer than the last-preflight marker. Empty on
# first run (marker doesn't exist yet — touch is the caller's job).
worktrees_newer_than_marker() {
  local marker; marker="$(pool_last_preflight)"
  [ -d "$POOL_WORKTREES_DIR" ] || return 0
  [ -e "$marker" ] || return 0
  find "$POOL_WORKTREES_DIR" -mindepth 1 -maxdepth 1 -type d \
    -newer "$marker" 2>/dev/null
}

# ---------------------------------------------------------------------------
# JS / Angular cache hygiene — same shape as legacy-targets and stale
# incrementals, applied to node_modules and .angular caches that bloat the
# same /projects volume cargo does. Active-subagent override applies: if a
# process CWD lives inside the containing worktree, the cache is left alone.

# Walk up from $1 to find the nearest dir containing .git (worktree root).
# Echoes the resolved path, or empty if none.
containing_worktree() {
  local d
  d="$(cd "$(dirname "${1}")" 2>/dev/null && pwd -P)" || return 0
  while [ "$d" != "/" ] && [ -n "$d" ]; do
    [ -e "$d/.git" ] && { echo "$d"; return 0; }
    d="$(dirname "$d")"
  done
}

# Is this directory inside an active-subagent's worktree?
in_active_worktree() {
  local target="$1"
  local wt; wt="$(containing_worktree "$target")"
  [ -z "$wt" ] && return 1
  local pids; pids="$(find_active_subagent_pids "$wt" 2>/dev/null | head -1)"
  [ -n "$pids" ]
}

# Tighter scope than in_active_worktree — does any process have CWD in the
# IMMEDIATE parent directory of the target (the "project" scope, not the
# whole repo). Used for node_modules / .angular hygiene: the worktree-wide
# check is too broad in a monorepo, where Claude/IDE/shells routinely sit
# at the repo root and would falsely mark every sub-project node_modules
# as active.
in_active_project() {
  local target="$1"
  local project
  project="$(cd "$(dirname "$target")" 2>/dev/null && pwd -P)" || return 1
  local pid_dir pid cwd
  for pid_dir in /proc/[0-9]*; do
    pid="${pid_dir##*/}"
    cwd="$(readlink "$pid_dir/cwd" 2>/dev/null)" || continue
    case "$cwd" in
      "$project"|"$project"/*) return 0 ;;
    esac
  done
  return 1
}

# Classify one node_modules dir by freshness against its nearest lockfile.
# Echoes: stale | fresh | active | unknown
classify_node_modules() {
  local nm="$1"
  if in_active_project "$nm"; then echo active; return; fi
  local parent; parent="$(dirname "$nm")"
  local lockfile=""
  # Find the nearest lockfile by walking up from $parent.
  local d="$parent"
  while [ "$d" != "/" ] && [ -n "$d" ]; do
    for cand in "$d/pnpm-lock.yaml" "$d/package-lock.json" "$d/yarn.lock"; do
      if [ -f "$cand" ]; then lockfile="$cand"; break 2; fi
    done
    d="$(dirname "$d")"
  done
  if [ -z "$lockfile" ]; then echo unknown; return; fi
  # Stale if lockfile is newer than .modules.yaml (pnpm's done-marker) or the
  # dir itself when no .modules.yaml exists.
  local marker="$nm/.modules.yaml"
  if [ -f "$marker" ]; then
    if [ "$lockfile" -nt "$marker" ]; then echo stale; else echo fresh; fi
  else
    if [ "$lockfile" -nt "$nm" ]; then echo stale; else echo fresh; fi
  fi
}

# List all node_modules outside pool, sophia submodule, and nested under
# other node_modules. Output: <classification>\t<bytes>\t<path>
list_node_modules() {
  local roots=()
  roots+=("$POOL_PARENT_REPO")
  if [ -d "$POOL_WORKTREES_DIR" ]; then
    local wt
    for wt in "$POOL_WORKTREES_DIR"/*; do
      [ -d "$wt" ] && roots+=("$wt")
    done
  fi
  local root
  for root in "${roots[@]}"; do
    find "$root" -maxdepth 6 -type d -name node_modules \
      -not -path "*/node_modules/*" \
      -not -path "*/sophia/*" \
      -not -path "$POOL_ROOT/*" \
      -not -path "*/.git/*" \
      -print 2>/dev/null
  done | sort -u | while IFS= read -r nm; do
    local cls bytes
    cls="$(classify_node_modules "$nm")"
    bytes="$(dir_disk_bytes "$nm")"
    printf '%s\t%s\t%s\n' "$cls" "$bytes" "$nm"
  done
}

# Total bytes recoverable by cleaning stale node_modules (lockfile drift).
node_modules_stale_bytes() {
  list_node_modules 2>/dev/null \
    | awk -F'\t' '$1=="stale" {sum+=$2} END {print sum+0}'
}

# Total bytes across ALL non-active node_modules. Used for "nuke" estimate.
node_modules_nuke_bytes() {
  list_node_modules 2>/dev/null \
    | awk -F'\t' '$1!="active" {sum+=$2} END {print sum+0}'
}

# Clean node_modules. mode=stale (default) → only `stale` class.
#                    mode=all              → everything except `active`.
clean_node_modules() {
  local mode="${1:-stale}" dry="${2:-0}"
  list_node_modules 2>/dev/null \
    | awk -F'\t' -v mode="$mode" '
        mode=="all"   && $1!="active"        {print; next}
        mode=="stale" && $1=="stale"         {print}
      ' \
    | while IFS=$'\t' read -r cls bytes path; do
      if [ "$dry" = "1" ]; then
        printf 'would-remove\t%s\t%s\t%s\n' "$cls" "$(human_bytes "$bytes")" "$path"
      else
        rm -rf "$path" 2>/dev/null && \
          printf 'removed\t%s\t%s\t%s\n' "$cls" "$(human_bytes "$bytes")" "$path"
      fi
    done
}

# .angular cache hygiene — pure speed cache, always safe to delete; Angular
# CLI rebuilds on next start/build. Default age threshold: 7 days.

# List .angular caches with bytes + age-in-days. Output:
#   <age_days>\t<bytes>\t<path>  or  active\t<bytes>\t<path>
list_angular_caches() {
  local roots=()
  roots+=("$POOL_PARENT_REPO")
  if [ -d "$POOL_WORKTREES_DIR" ]; then
    local wt
    for wt in "$POOL_WORKTREES_DIR"/*; do
      [ -d "$wt" ] && roots+=("$wt")
    done
  fi
  local root
  for root in "${roots[@]}"; do
    find "$root" -maxdepth 6 -type d -name .angular \
      -not -path "*/node_modules/*" \
      -not -path "*/sophia/*" \
      -not -path "$POOL_ROOT/*" \
      -not -path "*/.git/*" \
      -print 2>/dev/null
  done | sort -u | while IFS= read -r ac; do
    local bytes age_days mtime now
    bytes="$(dir_disk_bytes "$ac")"
    if in_active_project "$ac"; then
      printf 'active\t%s\t%s\n' "$bytes" "$ac"
      continue
    fi
    mtime="$(stat -c %Y "$ac" 2>/dev/null || echo 0)"
    now="$(date +%s)"
    age_days=$(( (now - mtime) / 86400 ))
    printf '%s\t%s\t%s\n' "$age_days" "$bytes" "$ac"
  done
}

# Total bytes from .angular caches older than $days that are not active.
angular_cache_stale_bytes() {
  local days="${1:-7}"
  list_angular_caches 2>/dev/null \
    | awk -F'\t' -v d="$days" '$1!="active" && ($1+0) >= d {sum+=$2} END {print sum+0}'
}

# All non-active .angular bytes (full nuke estimate).
angular_cache_total_bytes() {
  list_angular_caches 2>/dev/null \
    | awk -F'\t' '$1!="active" {sum+=$2} END {print sum+0}'
}

clean_angular_caches() {
  local days="${1:-7}" dry="${2:-0}"
  list_angular_caches 2>/dev/null \
    | awk -F'\t' -v d="$days" '$1!="active" && ($1+0) >= d' \
    | while IFS=$'\t' read -r age_days bytes path; do
      if [ "$dry" = "1" ]; then
        printf 'would-remove\t%sd\t%s\t%s\n' "$age_days" "$(human_bytes "$bytes")" "$path"
      else
        rm -rf "$path" 2>/dev/null && \
          printf 'removed\t%sd\t%s\t%s\n' "$age_days" "$(human_bytes "$bytes")" "$path"
      fi
    done
}

# ---------------------------------------------------------------------------
# Uncommitted-orphan scan — worktrees with dirty status AND no active
# subagent. These are crash-recovery candidates: fmt drift from a dead
# agent, half-finished work after a workspace restart, etc. Don't auto-
# clean; surface for operator triage via `cargo-pool orphans`.
#
# Output (tab-separated): <worktree> <branch> <dirty_count> <first_paths>
list_uncommitted_orphans() {
  [ -d "$POOL_WORKTREES_DIR" ] || return 0
  local wt
  for wt in "$POOL_WORKTREES_DIR"/*; do
    [ -d "$wt" ] || continue
    git -C "$wt" rev-parse HEAD >/dev/null 2>&1 || continue
    local dirty; dirty="$(git -C "$wt" status --porcelain 2>/dev/null)"
    [ -n "$dirty" ] || continue
    # Skip if any subagent process has CWD inside this worktree.
    local pids; pids="$(find_active_subagent_pids "$wt" 2>/dev/null | head -1)"
    [ -n "$pids" ] && continue
    local branch count first
    branch="$(git -C "$wt" rev-parse --abbrev-ref HEAD 2>/dev/null || echo unknown)"
    count="$(printf '%s\n' "$dirty" | wc -l | tr -d ' ')"
    first="$(printf '%s\n' "$dirty" | head -5 | awk '{print $NF}' | tr '\n' '|' | sed 's/|$//')"
    printf '%s\t%s\t%s\t%s\n' "$wt" "$branch" "$count" "$first"
  done
}
