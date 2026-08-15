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
# Dereference symlinks so /tmp-backed slots remain visible to pool accounting.
dir_disk_bytes() {
  local d="$1"
  [ -d "$d" ] || { echo 0; return; }
  du -sbL "$d" 2>/dev/null | awk '{print $1}'
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
  find "$fd" -mindepth 2 -maxdepth 2 \( -type d -o -type l \) 2>/dev/null | sort
}

# Linked slots are valid build targets but their backing directories live
# outside POOL_ROOT. Account for and lock-check them, but never select them for
# automatic eviction: removing the link would strand the backing artifacts and
# falsely report reclaimed bytes. Their lifecycle remains an operator decision.
family_has_linked_slots() {
  local family="$1"
  local fd="$POOL_ROOT/family/$family"
  [ -d "$fd" ] || return 1
  find "$fd" -mindepth 2 -maxdepth 2 -type l -print -quit 2>/dev/null | grep -q .
}

# Resolve a symlink target even when one or more path components do not exist.
# `readlink -f` cannot do that, which is precisely the post-/tmp-loss case this
# doctor must diagnose.
resolve_link_target_lexical() {
  local link="$1" raw
  raw="$(readlink -- "$link" 2>/dev/null)" || return 1
  case "$raw" in
    /*) readlink -m -- "$raw" ;;
    *)  readlink -m -- "$(dirname "$link")/$raw" ;;
  esac
}

# A missing path is preparable when its nearest existing ancestor is a
# directory. Existing dangling symlinks and regular-file ancestors are not.
path_is_preparable() {
  local candidate="$1" ancestor="$1" parent
  while [ ! -e "$ancestor" ] && [ ! -L "$ancestor" ]; do
    parent="$(dirname "$ancestor")"
    [ "$parent" = "$ancestor" ] && return 1
    ancestor="$parent"
  done
  [ -d "$ancestor" ] && [ "$candidate" != "$ancestor" ]
}

# List every cargo-target symlink the pool owns or relies on. Pool slots have
# the fixed family/<family>/<workspace>/<profile> shape. In-tree target links
# are scanned in the primary checkout and managed worktrees; directories are
# intentionally excluded because legacy-targets already owns that class.
list_cargo_target_links() {
  local family_root="$POOL_ROOT/family"
  if [ -d "$family_root" ]; then
    find "$family_root" -mindepth 3 -maxdepth 3 -type l \
      -printf 'pool-slot\t%p\n' 2>/dev/null
  fi

  local roots=("$POOL_PARENT_REPO") root wt
  if [ -d "$POOL_WORKTREES_DIR" ]; then
    for wt in "$POOL_WORKTREES_DIR"/*; do
      [ -d "$wt" ] && roots+=("$wt")
    done
  fi
  for root in "${roots[@]}"; do
    [ -d "$root" ] || continue
    if [ "$root" = "$POOL_PARENT_REPO" ]; then
      find "$root" -maxdepth 6 -type l -name target \
        -not -path "*/node_modules/*" \
        -not -path "$POOL_ROOT/*" \
        -not -path "*/.git/*" \
        -not -path "$POOL_WORKTREES_DIR/*" \
        -not -path "*/elohim/holochain/dna/*/target" \
        -printf 'in-tree-target\t%p\n' 2>/dev/null
    else
      find "$root" -maxdepth 6 -type l -name target \
        -not -path "*/node_modules/*" \
        -not -path "$POOL_ROOT/*" \
        -not -path "*/.git/*" \
        -not -path "*/elohim/holochain/dna/*/target" \
        -printf 'in-tree-target\t%p\n' 2>/dev/null
    fi
  done | sort -u
}

# Emit unhealthy cargo-target links as TSV:
#   <class> <state> <link> <raw-target> <resolved-target>
# `dangling` means the target is absent but can be recreated under /tmp.
# `unpreparable` means automatic healing is unsafe or structurally impossible.
list_cargo_target_link_findings() {
  local kind link raw resolved state
  while IFS=$'\t' read -r kind link; do
    [ -L "$link" ] || continue
    [ -d "$link" ] && continue
    raw="$(readlink -- "$link" 2>/dev/null || true)"
    resolved="$(resolve_link_target_lexical "$link" 2>/dev/null || true)"
    state=unpreparable
    case "$resolved" in
      /tmp/*)
        path_is_preparable "$resolved" && state=dangling
        ;;
    esac
    printf '%s\t%s\t%s\t%s\t%s\n' "$kind" "$state" "$link" "$raw" "$resolved"
  done < <(list_cargo_target_links)
}

# Recreate the directory behind one dangling link. Healing is deliberately
# constrained to /tmp: never rewrite the link and never create elsewhere.
heal_cargo_target_link() {
  local link="$1" resolved
  [ -L "$link" ] && [ ! -e "$link" ] || return 0
  resolved="$(resolve_link_target_lexical "$link" 2>/dev/null)" || return 1
  case "$resolved" in
    /tmp/*) ;;
    *) return 1 ;;
  esac
  path_is_preparable "$resolved" || return 1
  mkdir -p -- "$resolved" || return 1
  [ -d "$link" ]
}

# Create the minimal source crate used by the opt-in fingerprint write probe.
# The build target itself lives below each slot and is removed after every
# attempt; the source crate lives in /tmp and is removed by the caller.
create_cargo_target_write_probe_crate() {
  local crate_dir="$1"
  mkdir -p -- "$crate_dir/src" || return 1
  printf '%s\n' \
    '[package]' \
    'name = "cargo-pool-doctor-probe"' \
    'version = "0.0.0"' \
    'edition = "2021"' \
    '' \
    '[lib]' \
    'path = "src/lib.rs"' > "$crate_dir/Cargo.toml"
  printf '%s\n' '#![allow(dead_code)]' 'pub fn probe() {}' > "$crate_dir/src/lib.rs"
}

# Emit one TSV result for a Cargo-semantic write probe:
#   <state> <slot> <detail>
# States: ok | fingerprint-enoent | probe-failed | unavailable.
# This never rewrites a slot. Only the reserved disposable subtarget created by
# this function is removed.
probe_cargo_target_slot() {
  local slot="$1" crate_dir="$2" probe_target output state detail cargo_bin
  case "$slot" in
    "$POOL_ROOT"/family/*) ;;
    *) printf 'unavailable\t%s\toutside pool root\n' "$slot"; return 0 ;;
  esac
  if [ ! -d "$slot" ]; then
    printf 'unavailable\t%s\tslot does not resolve to a directory\n' "$slot"
    return 0
  fi

  probe_target="$slot/.cargo-pool-doctor-probe"
  cargo_bin="${CARGO:-cargo}"
  rm -rf -- "$probe_target"
  if output="$(
    CARGO_TARGET_DIR="$probe_target" \
      CARGO_INCREMENTAL=0 \
      RUSTC_WRAPPER="" \
      RUSTFLAGS="" \
      "$cargo_bin" check --manifest-path "$crate_dir/Cargo.toml" --quiet 2>&1
  )"; then
    state=ok
    detail=""
  elif printf '%s\n' "$output" | grep -q '\.fingerprint/' \
    && printf '%s\n' "$output" | grep -qE 'No such file or directory|os error 2'; then
    state=fingerprint-enoent
    detail="$(printf '%s\n' "$output" | grep '\.fingerprint/' | head -1)"
  else
    state=probe-failed
    detail="$(printf '%s\n' "$output" | head -1)"
  fi
  rm -rf -- "$probe_target"
  detail="${detail//$'\t'/ }"
  printf '%s\t%s\t%s\n' "$state" "$slot" "$detail"
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
  local fam slot
  for fam in $(list_families); do
    while IFS= read -r slot; do
      case "$slot" in */dev|*/release) record_slot_watermark "$slot" ;; esac
    done < <(list_slots_for_family "$fam")
  done
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
# Pool policy (pool-policy.json) — the operator's standing eviction decisions,
# committed to the repo so enforcement is deterministic. See the policy file's
# _comment / _dispositions for semantics. All readers fail-soft to safe
# defaults when the file is missing (e.g. on an old branch).

POOL_POLICY_FILE="${CARGO_TARGET_POOL_POLICY:-$(cd "$(dirname "${BASH_SOURCE[0]}")" 2>/dev/null && pwd)/../pool-policy.json}"

# policy_get <jq-expr> <default> — read one value from the policy file.
policy_get() {
  local expr="$1" default="${2:-}" v=""
  if [ -f "$POOL_POLICY_FILE" ]; then
    v="$(jq -r "$expr // empty" "$POOL_POLICY_FILE" 2>/dev/null)"
  fi
  echo "${v:-$default}"
}

policy_pool_max_gb() { policy_get '.pool_max_gb' 150; }
policy_soft_pct()    { policy_get '.volume_soft_pct' 75; }
policy_hard_pct()    { policy_get '.volume_hard_pct' 85; }
policy_stale_days()  { policy_get '.stale_incremental_days' 3; }
policy_hash_keep()   { policy_get '.artifact_hash_keep' 1; }
policy_hash_min_mb() { policy_get '.artifact_min_mb' 64; }
policy_hash_min_age_hours() { policy_get '.artifact_min_age_hours' 6; }

# Disposition for a family, falling back to the "*" entry, then 'ttl'.
policy_family_disposition() {
  local fam="$1" v
  v="$(policy_get ".families[\"$fam\"].disposition" '')"
  [ -n "$v" ] && { echo "$v"; return; }
  policy_get '.families["*"].disposition' 'ttl'
}

policy_family_ttl_days() {
  local fam="$1" v
  v="$(policy_get ".families[\"$fam\"].ttl_days" '')"
  [ -n "$v" ] && { echo "$v"; return; }
  policy_get '.families["*"].ttl_days' 7
}

policy_family_max_gb() { policy_get ".families[\"$1\"].max_gb" 0; }

# ---------------------------------------------------------------------------
# Enforce guards — what enforce_pool must never touch.

# A slot is locked when a live cargo holds its advisory .cargo-lock. We test
# with a non-blocking flock attempt: if we CANNOT take the lock, cargo holds
# it. Never kill or starve a running build (see memory:
# feedback_workflow_long_cargo_orphan_lock — orphaned-looking locks resolve
# when the build finishes; the work lands on disk).
slot_locked() {
  local slot="${1:-}" prof lock
  [ -n "$slot" ] || return 1
  for prof in debug release; do
    lock="$slot/$prof/.cargo-lock"
    [ -f "$lock" ] || continue
    if ! flock -n "$lock" true 2>/dev/null; then
      return 0
    fi
  done
  return 1
}

# A family is busy when any of its slots is locked, or any live process has
# its CWD inside the family tree.
family_busy() {
  local fam="${1:-}" fd
  [ -n "$fam" ] || return 1
  fd="$POOL_ROOT/family/$fam"
  [ -d "$fd" ] || return 1
  local slot
  while IFS= read -r slot; do
    [ -n "$slot" ] || continue
    if slot_locked "$slot"; then return 0; fi
  done < <(list_slots_for_family "$fam")
  local pid_dir cwd
  for pid_dir in /proc/[0-9]*; do
    cwd="$(readlink "$pid_dir/cwd" 2>/dev/null)" || continue
    case "$cwd" in "$fd"|"$fd"/*) return 0 ;; esac
  done
  return 1
}

# Families that must never be touched by enforce: the main checkout's family
# plus the family of every worktree that has a live subagent process.
#
# FAIL-SAFE (review C2): under detached HEAD (rebase/bisect/sha-checkout)
# detect_family degrades to the directory basename ('elohim'), silently
# UN-protecting the real warm family. If the checkout is detached and no
# explicit family signal exists (env override or .family file), protect
# EVERY family — enforce becomes hygiene-only for that window.
protected_families() {
  local fams="" f wt branch
  branch="$(git -C "$POOL_PARENT_REPO" rev-parse --abbrev-ref HEAD 2>/dev/null || echo)"
  if { [ -z "$branch" ] || [ "$branch" = "HEAD" ]; } \
      && [ -z "${CARGO_TARGET_POOL_FAMILY:-}" ] \
      && [ ! -f "$POOL_PARENT_REPO/.family" ]; then
    list_families
    return
  fi
  f="$(detect_family "$POOL_PARENT_REPO" 2>/dev/null || true)"
  [ -n "$f" ] && fams="$fams $f"
  if [ -d "$POOL_WORKTREES_DIR" ]; then
    for wt in "$POOL_WORKTREES_DIR"/*; do
      [ -d "$wt" ] || continue
      if [ -n "$(find_active_subagent_pids "$wt" 2>/dev/null | head -1)" ]; then
        f="$(detect_family "$wt" 2>/dev/null || true)"
        [ -n "$f" ] && fams="$fams $f"
      fi
    done
  fi
  echo "$fams" | tr ' ' '\n' | sed '/^$/d' | sort -u
}

# All branches under <family>/* merged into dev (local AND origin-remote),
# and dev itself resolvable. Conservative on every edge (review D1/D2):
#   * any git invocation FAILURE → NOT merged (fail-closed, never treat an
#     enumeration error as emptiness)
#   * ZERO attributable <fam>/* branches is ambiguous — branches deleted
#     post-merge (cold) vs a hyphen-form branch (fam-foo) this glob can't
#     see (possibly hot). Disambiguate by tree activity: a fresh family is
#     NOT evictable; only idle-a-week-plus counts as merged-and-gone.
family_all_merged() {
  local fam="${1:-}"
  [ -n "$fam" ] || return 1
  git -C "$POOL_PARENT_REPO" rev-parse --verify dev >/dev/null 2>&1 || return 1
  local out_local out_remote
  out_local="$(git -C "$POOL_PARENT_REPO" branch --list "$fam/*" 2>/dev/null)" || return 1
  out_remote="$(git -C "$POOL_PARENT_REPO" branch -r --list "origin/$fam/*" 2>/dev/null)" || return 1
  if [ -z "$out_local" ] && [ -z "$out_remote" ]; then
    family_fresh_within_days "$fam" 7 && return 1
    return 0
  fi
  local unm
  unm="$(git -C "$POOL_PARENT_REPO" branch --list "$fam/*" --no-merged dev 2>/dev/null)" || return 1
  if printf '%s\n' "$unm" | sed -E 's/^[*+ ]+ ?//' | grep -q .; then
    return 1
  fi
  unm="$(git -C "$POOL_PARENT_REPO" branch -r --list "origin/$fam/*" --no-merged dev 2>/dev/null)" || return 1
  if printf '%s\n' "$unm" | grep -q .; then
    return 1
  fi
  return 0
}

# Any filesystem activity inside the family tree within $days?
# (-print -quit short-circuits on the first fresh file.)
family_fresh_within_days() {
  local fam="$1" days="$2" fd="$POOL_ROOT/family/$fam"
  [ -d "$fd" ] || return 1
  [ -n "$(find "$fd" -mtime "-$days" -print -quit 2>/dev/null)" ]
}

# Freshest activity timestamp (epoch) for a slot — .cargo-lock is touched on
# every build; fall back to marker files, then the dir itself.
slot_age_epoch() {
  local slot="$1" t=0 f m
  for f in "$slot/debug/.cargo-lock" "$slot/release/.cargo-lock" \
           "$slot/.last-build" "$slot/.peak-size"; do
    [ -e "$f" ] || continue
    m="$(stat -c %Y "$f" 2>/dev/null || echo 0)"
    [ "${m:-0}" -gt "$t" ] && t="$m"
  done
  if [ "$t" -eq 0 ]; then
    t="$(stat -c %Y "$slot" 2>/dev/null || echo 0)"
  fi
  echo "$t"
}

# ---------------------------------------------------------------------------
# Stale artifact-hash GC — THE dominant reclaim (measured 2026-06-04: 71% of
# a 321G pool was ~1GB DWARF-laden test/bin executables, most of them
# SUPERSEDED build hashes cargo never collects; e.g. one slot held 13
# distinct ~1GB elohim_storage-<hash> binaries).
#
# CORRECTNESS MODEL (adversarial-review hardened, 2026-06-04):
# Stripping the -<16hex> hash from a filename does NOT identify a unique
# cargo target — a [[bin]] and the package lib's unit-test executable can
# share a name (elohim-storage does), and same-named integration tests can
# exist across crates. "Newest by mtime" only proves supersession WITHIN one
# target. So:
#   * no-extension executables are grouped by stem + the first source path
#     from their sibling .d file (src/main.rs vs src/lib.rs vs tests/x.rs
#     distinguishes bin / lib-test / integration-test targets). Executables
#     with no .d sibling are NEVER touched (unclassifiable → conservative).
#   * files with st_nlink > 1 are NEVER deleted (cargo hardlink-uplifts the
#     current bin/lib to the profile root — nlink>1 means "current").
#   * files younger than $min_age_h hours are NEVER touched — closes the
#     race where a just-built test binary is deleted between build and
#     execution (the build lock is released before tests run, so
#     slot_locked/family_busy cannot see the execution phase).
#   * .rlib/.so dep artifacts keep the simple stem grouping (one lib target
#     per crate); a wrong pick there costs one dep recompile, not an exec
#     failure.
#
# Only files >= $min_mb are considered — bounds the find cost and the risk
# surface to the artifacts that actually matter.
#
# Output: <bytes>\t<path>
list_stale_artifact_hashes() {
  local root="${1:-$POOL_ROOT/family}" keep="${2:-1}" min_mb="${3:-64}" min_age_h="${4:-6}"
  [ -d "$root" ] || return 0
  local min_age_min=$((min_age_h * 60))
  find "$root" -type d -name deps 2>/dev/null \
    | while IFS= read -r deps; do
      find "$deps" -maxdepth 1 -type f -size "+${min_mb}M" -mmin "+${min_age_min}" \
        -printf '%T@\t%s\t%n\t%p\n' 2>/dev/null \
        | while IFS="$(printf '\t')" read -r mt sz nl p; do
            local n cls stem src dfile
            n="${p##*/}"
            cls=""
            case "$n" in
              *.rlib) cls=".rlib"; n="${n%.rlib}" ;;
              *.so)   cls=".so";   n="${n%.so}" ;;
              *.*)    continue ;;   # .d/.rmeta/etc — not our mass
            esac
            case "$n" in
              *-[0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f])
                stem="${n%-*}" ;;
              *) continue ;;        # no cargo hash suffix → skip
            esac
            src="-"
            if [ -z "$cls" ]; then
              # no-extension executable: disambiguate distinct targets via
              # the .d sibling's first source path; no .d → never touch.
              dfile="${p}.d"
              [ -f "$dfile" ] || continue
              src="$(sed -n '1s/^[^:]*: *//p' "$dfile" 2>/dev/null | awk '{print $1}')"
              [ -n "$src" ] || continue
            fi
            printf '%s\t%s\t%s\t%s\t%s\n' "$mt" "$sz" "$nl" "${stem}${cls}|${src}" "$p"
          done \
        | sort -t"$(printf '\t')" -k4,4 -k1,1gr \
        | awk -F'\t' -v keep="$keep" '{
            if ($4!=prev) { prev=$4; kept=0 }
            if ($3+0 > 1) { kept++; next }   # hardlink-uplifted = current → keep
            if (kept < keep) { kept++; next } # newest singles → keep
            print $2 "\t" $5
          }'
    done
}

stale_artifact_hash_bytes() {
  list_stale_artifact_hashes "${1:-$POOL_ROOT/family}" "${2:-1}" "${3:-64}" "${4:-6}" \
    | awk -F'\t' '{sum+=$1} END {print sum+0}'
}

# Prune stale artifact hashes under $root. yes=1 applies; else dry-run lines.
prune_stale_artifact_hashes() {
  local root="${1:-$POOL_ROOT/family}" keep="${2:-1}" min_mb="${3:-64}" min_age_h="${4:-6}" yes="${5:-0}"
  list_stale_artifact_hashes "$root" "$keep" "$min_mb" "$min_age_h" \
    | while IFS="$(printf '\t')" read -r bytes path; do
      if [ "$yes" = "1" ]; then
        rm -f "$path" 2>/dev/null && \
          printf 'removed\t%s\t%s\n' "$(human_bytes "$bytes")" "$path"
      else
        printf 'would-remove\t%s\t%s\n' "$(human_bytes "$bytes")" "$path"
      fi
    done
}

# ---------------------------------------------------------------------------
# enforce_pool — the deterministic pool reconciler (ccache semantics: bounded
# cache, automatic eviction, no human in the loop). Policy comes from
# pool-policy.json. Called by cargo-pool enforce, the SessionStart async hook
# (clean-caches.sh), the Stop hook (pool-postflight.sh), and .husky/pre-push
# under disk pressure.
#
# Unconditional hygiene (junk by definition — runs every time):
#   1. stale incremental hash dirs   (> stale_incremental_days)
#   2. stale artifact-hash copies    (keep newest K per stem; SKIPS protected
#      and busy families — the active family's newest hash may be mid-use)
#   3. legacy native targets outside the pool + stale node_modules
# Disposition pass (the operator's standing decisions):
#   4. evict-on-merge families whose <fam>/* branches are all merged;
#      ttl families idle past ttl_days; keep-warm families trimmed to max_gb
# Budget pass (useful-but-evictable warm caches, LRU):
#   5. while pool > pool_max_gb OR volume > volume_soft_pct: evict the
#      oldest unprotected unlocked slot (release profiles biased to go first)
#
# Guards at every destructive step: protected families (active session +
# live-PID worktrees) are never touched; flock'd slots are never touched;
# a pool-level flock makes concurrent enforce runs a no-op.
#
# enforce_pool <apply:0|1> <quiet:0|1>   (apply=0 → dry-run, prints plan)
enforce_pool() {
  local apply="${1:-0}" quiet="${2:-0}"
  local say
  if [ "$quiet" = "1" ]; then say() { :; }; else say() { echo "[enforce] $*"; }; fi

  pool_init

  # Concurrency: one enforce at a time, ever. fd 9 is explicitly closed on
  # every return path (review D5: a leaked fd into a sourcing hook process
  # would hold the advisory lock past the run, and a second in-process call
  # re-exec'ing 9>> would silently drop the first one's lock).
  exec 9>>"$POOL_ROOT/.enforce-lock" || { say "cannot open enforce lock"; return 0; }
  if ! flock -n 9 2>/dev/null; then
    say "another enforce is running — skipping"
    exec 9>&-
    return 0
  fi

  local protected
  protected="$(protected_families | tr '\n' ' ' | sed 's/ $//')"
  say "policy: pool<=$(policy_pool_max_gb)G vol<=$(policy_soft_pct)% (hard $(policy_hard_pct)%) | protected: ${protected:-none} | mode: $([ "$apply" = 1 ] && echo APPLY || echo dry-run)"

  local freed=0 fam

  # -- 1. stale incrementals ------------------------------------------------
  local days b1
  days="$(policy_stale_days)"
  b1="$(stale_incremental_bytes "$days")"
  if [ "${b1:-0}" -gt 0 ]; then
    say "1 stale-incrementals (>${days}d): $(human_bytes "$b1")"
    prune_stale_incrementals "$days" "$apply" | sed 's/^/[enforce]   /'
    [ "$apply" = 1 ] && { freed=$((freed + b1)); enforce_log stale-incrementals "pool" "$b1"; }
  else
    say "1 stale-incrementals: clean"
  fi

  # -- 2. stale artifact hashes (unprotected, non-busy families only) -------
  local keep min_mb min_age_h
  keep="$(policy_hash_keep)"; min_mb="$(policy_hash_min_mb)"; min_age_h="$(policy_hash_min_age_hours)"
  for fam in $(list_families); do
    case " $protected " in *" $fam "*) say "2 hash-gc $fam: skipped (protected)"; continue ;; esac
    if family_busy "$fam"; then say "2 hash-gc $fam: skipped (busy)"; continue; fi
    local b2
    b2="$(stale_artifact_hash_bytes "$POOL_ROOT/family/$fam" "$keep" "$min_mb" "$min_age_h")"
    if [ "${b2:-0}" -gt 0 ]; then
      say "2 hash-gc $fam: $(human_bytes "$b2") in superseded build hashes (>${min_age_h}h old, per-target grouped, hardlinked-current exempt)"
      prune_stale_artifact_hashes "$POOL_ROOT/family/$fam" "$keep" "$min_mb" "$min_age_h" "$apply" | sed 's/^/[enforce]   /'
      [ "$apply" = 1 ] && { freed=$((freed + b2)); enforce_log stale-hashes "$fam" "$b2"; }
    fi
  done

  # -- 3. legacy targets + stale node_modules -------------------------------
  local b3a b3b
  b3a="$(legacy_native_bytes)"
  if [ "${b3a:-0}" -gt 268435456 ]; then
    say "3 legacy-targets: $(human_bytes "$b3a")"
    clean_legacy_native_targets "$([ "$apply" = 1 ] && echo 0 || echo 1)" | sed 's/^/[enforce]   /'
    [ "$apply" = 1 ] && { freed=$((freed + b3a)); enforce_log legacy-targets "repo" "$b3a"; }
  fi
  b3b="$(node_modules_stale_bytes)"
  if [ "${b3b:-0}" -gt 268435456 ]; then
    say "3 stale-node_modules: $(human_bytes "$b3b")"
    clean_node_modules stale "$([ "$apply" = 1 ] && echo 0 || echo 1)" | sed 's/^/[enforce]   /'
    [ "$apply" = 1 ] && { freed=$((freed + b3b)); enforce_log stale-node-modules "repo" "$b3b"; }
  fi

  # -- 4. dispositions -------------------------------------------------------
  for fam in $(list_families); do
    case " $protected " in *" $fam "*) continue ;; esac
    if family_busy "$fam"; then say "4 $fam: skipped (busy)"; continue; fi
    if family_has_linked_slots "$fam"; then
      say "4 $fam: skipped (linked slot lifecycle is operator-owned)"
      continue
    fi
    local disp fd fb
    disp="$(policy_family_disposition "$fam")"
    fd="$POOL_ROOT/family/$fam"
    case "$disp" in
      evict-on-merge)
        if family_all_merged "$fam"; then
          fb="$(dir_disk_bytes "$fd")"
          say "4 $fam: evict-on-merge — all $fam/* branches merged ($(human_bytes "$fb"))"
          if [ "$apply" = 1 ]; then
            rm -rf "$fd" && { freed=$((freed + fb)); enforce_log evict-on-merge "$fam" "$fb"; }
          fi
        fi
        ;;
      ttl)
        local ttl
        ttl="$(policy_family_ttl_days "$fam")"
        if ! family_fresh_within_days "$fam" "$ttl"; then
          fb="$(dir_disk_bytes "$fd")"
          say "4 $fam: ttl — idle >${ttl}d ($(human_bytes "$fb"))"
          if [ "$apply" = 1 ]; then
            rm -rf "$fd" && { freed=$((freed + fb)); enforce_log ttl-evict "$fam" "$fb"; }
          fi
        fi
        ;;
      keep-warm)
        local cap
        cap="$(policy_family_max_gb "$fam")"
        if [ "${cap:-0}" -gt 0 ]; then
          trim_family_to_gb "$fam" "$cap" "$apply" "$quiet"
        fi
        ;;
      pin) : ;;
    esac
  done

  # -- 5. LRU budget trim ----------------------------------------------------
  # Evicts oldest unprotected slots until pool <= pool_max_gb AND volume <=
  # soft_pct. keep-warm/pin families are exempt (their size is governed by
  # their own max_gb trim in step 4). Dry-run simulates: removed slots go on
  # a skip-list and the volume %% is recomputed from simulated freed bytes.
  local pool_b max_b pct soft
  pool_b="$(pool_disk_bytes)"
  max_b=$(( $(policy_pool_max_gb) * 1073741824 ))
  soft="$(policy_soft_pct)"
  local vol_total_kb vol_used_kb sim_freed=0
  vol_total_kb="$(df -P "$POOL_PARENT_REPO" 2>/dev/null | awk 'NR==2 {print $2}')"
  vol_used_kb="$(df -P "$POOL_PARENT_REPO" 2>/dev/null | awk 'NR==2 {print $3}')"
  local lru_guard=0 lru_skip=""
  while :; do
    if [ "$apply" = 1 ] || [ -z "${vol_total_kb:-}" ] || [ "${vol_total_kb:-0}" -le 0 ]; then
      pct="$(disk_pct_used)"
    else
      # dry-run: simulate — subtract planned frees from the df snapshot
      pct=$(( (vol_used_kb - sim_freed / 1024) * 100 / vol_total_kb ))
    fi
    [ "${pool_b:-0}" -le "$max_b" ] && [ "${pct:-0}" -le "$soft" ] && break
    lru_guard=$((lru_guard + 1)); [ "$lru_guard" -gt 50 ] && break
    local victim
    victim="$(pick_lru_slot "$protected" "$lru_skip")"
    if [ -z "$victim" ]; then
      say "5 budget still exceeded (pool $(human_bytes "$pool_b"), vol ${pct:-?}%) but no evictable slots remain (keep-warm/pin/protected/locked exempt)"
      break
    fi
    local vb
    vb="$(dir_disk_bytes "$victim")"
    lru_skip="$lru_skip
$victim"
    # TOCTOU recheck (review D3): a cargo may have taken the slot between
    # pick and now (the du above can take seconds on a big slot).
    if slot_locked "$victim"; then
      say "5 lru-evict: $victim — became locked, skipping"
      continue
    fi
    say "5 lru-evict: $victim ($(human_bytes "$vb"))"
    if [ "$apply" = 1 ]; then
      rm -rf "$victim" && { freed=$((freed + vb)); pool_b=$((pool_b - vb)); enforce_log lru-evict "$victim" "$vb"; }
    else
      pool_b=$((pool_b - vb)); sim_freed=$((sim_freed + vb))
    fi
  done

  # Stamp pool size for cheap readers (cargo-disk-guard.py, status).
  if [ "$apply" = 1 ]; then
    pool_disk_bytes > "$POOL_ROOT/.pool-size-bytes" 2>/dev/null || true
  fi

  if [ "$apply" = 1 ]; then
    say "done — freed $(human_bytes "$freed"); volume now $(disk_human_summary) ($(disk_pct_used)%)"
  else
    say "done — dry-run plan above (apply with: cargo-pool enforce --yes); volume currently $(disk_human_summary) ($(disk_pct_used)%)"
  fi
  enforce_log summary "pool" "$freed" "apply=$apply"
  exec 9>&-
  return 0
}

# Trim a keep-warm family down to its max_gb: release slots first (oldest
# first), then oldest dev slots. Locked slots are skipped. The FRESHEST slot
# always survives — "keep-warm" means the primary warm cache is never
# evicted, even when it alone exceeds the cap (the cap then becomes
# best-effort; profile shrink is the durable fix for oversized slots).
trim_family_to_gb() {
  local fam="$1" cap_gb="$2" apply="${3:-0}" quiet="${4:-0}"
  local say
  if [ "$quiet" = "1" ]; then say() { :; }; else say() { echo "[enforce] $*"; }; fi
  local fd="$POOL_ROOT/family/$fam"
  [ -d "$fd" ] || return 0
  local cap_b=$((cap_gb * 1073741824))
  local size
  size="$(dir_disk_bytes "$fd")"
  [ "${size:-0}" -le "$cap_b" ] && return 0
  # Identify the freshest slot — the one keep-warm exists to protect.
  local freshest="" freshest_t=0 s t
  while IFS= read -r s; do
    [ -n "$s" ] || continue
    t="$(slot_age_epoch "$s")"
    if [ "${t:-0}" -gt "$freshest_t" ]; then freshest_t="$t"; freshest="$s"; fi
  done < <(list_slots_for_family "$fam")
  say "4 $fam: keep-warm over cap ($(human_bytes "$size") > ${cap_gb}G) — trimming (freshest slot kept: ${freshest:-none})"
  local slot
  while IFS= read -r slot; do
    [ -n "$slot" ] || continue
    [ "${size:-0}" -le "$cap_b" ] && break
    [ "$slot" = "$freshest" ] && continue
    [ -L "$slot" ] && continue
    if slot_locked "$slot"; then continue; fi
    local sb
    sb="$(dir_disk_bytes "$slot")"
    say "4 $fam: trim $slot ($(human_bytes "$sb"))"
    if [ "$apply" = 1 ]; then
      rm -rf "$slot" && { size=$((size - sb)); enforce_log keep-warm-trim "$slot" "$sb"; }
    else
      size=$((size - sb))
    fi
  done < <(
    # order: release profile slots oldest-first, then dev slots oldest-first
    for s in $(list_slots_for_family "$fam"); do
      printf '%s\t%s\t%s\n' "$(case "$s" in */release) echo 0 ;; *) echo 1 ;; esac)" "$(slot_age_epoch "$s")" "$s"
    done | sort -t"$(printf '\t')" -k1,1n -k2,2n | cut -f3
  )
  if [ "${size:-0}" -gt "$cap_b" ]; then
    say "4 $fam: still over cap after trim ($(human_bytes "$size") > ${cap_gb}G) — freshest slot preserved by keep-warm; shrink it via build profiles, not eviction"
  fi
}

# Oldest unprotected, unlocked slot across all families. Release-profile
# slots are biased to evict first (treated as 30d older). keep-warm and pin
# families are exempt — their size is governed by their own disposition.
# $2 (optional): newline-separated skip-list of already-chosen victims
# (dry-run simulation support).
pick_lru_slot() {
  local protected=" ${1:-} " skip="${2:-}"
  local best="" best_t=9999999999 fam slot t disp
  for fam in $(list_families); do
    case "$protected" in *" $fam "*) continue ;; esac
    disp="$(policy_family_disposition "$fam")"
    case "$disp" in keep-warm|pin) continue ;; esac
    # Same busy-guard as steps 2/4 (review D3): a momentarily-unlocked but
    # actively-used family must not lose slots to the budget pass.
    if family_busy "$fam"; then continue; fi
    while IFS= read -r slot; do
      [ -n "$slot" ] || continue
      case "$skip" in *"$slot"*) continue ;; esac
      [ -L "$slot" ] && continue
      if slot_locked "$slot"; then continue; fi
      t="$(slot_age_epoch "$slot")"
      case "$slot" in */release) t=$((t - 2592000)) ;; esac
      if [ "${t:-0}" -lt "$best_t" ]; then best_t="$t"; best="$slot"; fi
    done < <(list_slots_for_family "$fam")
  done
  echo "$best"
}

# Append one JSON enforcement record to pool.log.
enforce_log() {
  local action="$1" target="$2" bytes="${3:-0}" note="${4:-}"
  local line
  line="$(jq -nc \
    --arg ts "$(now_iso)" --arg event enforce \
    --arg action "$action" --arg target "$target" \
    --argjson bytes "${bytes:-0}" --arg note "$note" \
    '{ts:$ts,event:$event,action:$action,target:$target,bytes:$bytes} as $b
     | if ($note | length) > 0 then $b + {note:$note} else $b end' 2>/dev/null)" || return 0
  echo "$line" >> "$(pool_log_path)" 2>/dev/null || true
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
