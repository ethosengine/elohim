#!/usr/bin/env bash
# pool-preflight.sh — SessionStart hook for the cargo target pool.
#
# 1. Initialize the pool root if absent.
# 2. Refresh stewardship off-path; print the last completed report with its age.
# 3. Detect family for $CLAUDE_PROJECT_DIR's worktree.
# 4. Emit a context block of at most four lines when the disk is healthy:
#      line 1  pool root, the percent used of the VOLUME holding the repo
#              (not of the pool root alone), and the family;
#      line 2  (only when non-zero) the worktree stewardship line with the
#              age of the pass: something was removed or an orphan/dirty
#              worktree was logged;
#      line 3  (only when a worktree created since the last preflight has a
#              "short" disk estimate) one budget line naming the worktree,
#              the estimate, the free space and `cargo-pool estimate`;
#      last    one pointer to `cargo-pool key` (slot for the current
#              directory) and the full reference (`cargo-pool status`,
#              `cargo-pool --help`).
#    At warn the disk-pressure banner adds 2 lines (max 6 in all) and at
#    critical 4 lines (max 8 in all). An agent that runs cargo without a
#    CARGO_TARGET_DIR is told the exact slot at that moment by
#    .claude/hooks/cargo-disk-guard.py, so no slot table is printed here.
#    The stewardship and budget facts are computed in the detached refresh
#    and cached in one file; the hook only reads it.
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

  # Worktrees created since the last preflight whose estimate is "short": the
  # slow estimate runs here, detached, never in the hook. The marker is touched
  # below, so each worktree is judged once.
  local budget_rows="" nwt budget est free verdict
  while IFS= read -r nwt; do
    [ -n "$nwt" ] && [ -d "$nwt" ] || continue
    budget="$(disk_budget_for_worktree "$nwt" 2>/dev/null)" || continue
    est="$(echo "$budget" | cut -f1)"
    free="$(echo "$budget" | cut -f2)"
    verdict="$(echo "$budget" | cut -f3)"
    [ "$verdict" = "short" ] || continue
    budget_rows="${budget_rows}budget	$(basename "$nwt")	${est}	${free}
"
  done < <(worktrees_newer_than_marker 2>/dev/null)

  local mode="live" tmp="$CACHE.$$.tmp"
  if [ "$DRY" = "1" ]; then mode="dry"; fi
  trap 'rm -f -- "$CACHE.$$.tmp"' EXIT
  # Tab-separated records: steward2 <removed> <orphan> <mode>, then budget rows.
  # An older cache line has no steward2 prefix and is never printed.
  {
    printf 'steward2\t%s\t%s\t%s\n' "${COUNT_REMOVED:-0}" "${COUNT_ORPHAN:-0}" "$mode"
    printf '%s' "$budget_rows"
  } >"$tmp" || return 1
  mv -f -- "$tmp" "$CACHE" || return 1
  touch "$(pool_last_preflight)" 2>/dev/null || true
}

if [ "${1:-}" = "--refresh" ]; then
  refresh_report
  exit 0
fi

# The reported pass exceeded 120 s (pool-postflight walks: 549-624 s). All walks
# run detached under flock -n; the 15 s hook only reads a completed report.
CACHE_AT="$(stat -c %Y "$CACHE" 2>/dev/null || echo 0)"
NOW="$(date +%s)"
AGE=$((NOW - CACHE_AT))
STEWARD_REPORT=""
BUDGET_REPORT=""
if [ -s "$CACHE" ] && (( AGE >= 0 )); then
  while IFS=$'\t' read -r kind c1 c2 c3; do
    case "$kind" in
      steward2)
        if [[ "$c1" =~ ^[0-9]+$ && "$c2" =~ ^[0-9]+$ ]] && (( c1 > 0 || c2 > 0 )); then
          if [ "$c3" = "dry" ]; then
            STEWARD_REPORT="Worktree stewardship (dry-run plan, pass ${AGE}s ago): $c1 would remove (merged-clean), $c2 would log orphan/dirty."
          else
            STEWARD_REPORT="Worktree stewardship (pass ${AGE}s ago): $c1 removed (merged-clean), $c2 logged orphan/dirty."
          fi
        fi
        ;;
      budget)
        BUDGET_REPORT="Budget SHORT for new worktree $c1: first build estimate ~${c2}G, free ${c3}G; run \`cargo-pool estimate\` before building."
        ;;
    esac
  done <"$CACHE"
fi
if ! [ -s "$CACHE" ] || (( AGE < 0 || AGE >= 120 )); then
  setsid bash "$SCRIPT_DIR/pool-preflight.sh" --refresh </dev/null >/dev/null 2>&1 &
fi

WT_ROOT="$(find_worktree_root "$POOL_PARENT_REPO" 2>/dev/null || echo "$POOL_PARENT_REPO")"
FAMILY="$(detect_family "$WT_ROOT")"
[ -z "$FAMILY" ] && FAMILY="unknown"

# Disk-pressure banner: 80% warns, 90% screams; silent below that.
DISK_PCT="$(disk_pct_used)"
DISK_STATUS="$(disk_status_for_pct "${DISK_PCT:-0}")"
DISK_SUMMARY="$(disk_human_summary)"
DISK_BANNER=""
case "$DISK_STATUS" in
  critical)
    DISK_BANNER="!! CRITICAL DISK PRESSURE: $DISK_SUMMARY (${DISK_PCT}% used). RUN CLEANUP NOW:
 bash genesis/agentic/bin/cargo-pool enforce --yes
 (policy-driven ladder: genesis/agentic/pool-policy.json — the SessionStart
  async hook also runs this; heavy cargo is DENIED at the hard ceiling)"
    ;;
  warn)
    DISK_BANNER="!  Disk pressure: $DISK_SUMMARY (${DISK_PCT}% used). Reclaim per policy:
 bash genesis/agentic/bin/cargo-pool enforce --yes"
    ;;
esac

CTX="Cargo pool: $POOL_ROOT; volume ${DISK_PCT:-?}% used; family $FAMILY."
[ -n "$DISK_BANNER" ] && CTX="$CTX
$DISK_BANNER"
[ -n "$STEWARD_REPORT" ] && CTX="$CTX
$STEWARD_REPORT"
[ -n "$BUDGET_REPORT" ] && CTX="$CTX
$BUDGET_REPORT"
CTX="$CTX
Native cargo needs CARGO_TARGET_DIR: \`cargo-pool key\` prints this directory's slot; \`cargo-pool status\` and \`cargo-pool --help\` (genesis/agentic/bin/cargo-pool) hold the full reference."

jq -n \
  --arg ctx "$CTX" \
  '{hookSpecificOutput: {hookEventName: "SessionStart", additionalContext: $ctx}}'
