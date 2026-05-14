#!/usr/bin/env bash
# memory-balance.sh — deterministic snapshot of the project's memory footprint.
#
# Standing pattern: run at Wave 0 (baseline) and Wave 6 (end-state) of every
# memory-team ceremony. Each run persists a JSON + text snapshot and reports
# the delta against the most-recent prior snapshot.
#
# Headline diagnostic: Surface : Archive ratio.
#   Healthy: trending toward <100:1.
#   Smoking gun: ratio stays high or grows → distillation pipeline isn't running.
#
# Runaway signals (worth alarming on):
#   - MEMORY.md > 32KB (and growing across cycles)
#   - Stories never written / no canonical
#   - Working memory grows while archive stays flat (no graduation)
#   - Memorialize-archive entries without story_pointer (lost context)
#
# Usage:
#   ./genesis/scripts/memory-balance.sh                    # snapshot + diff
#   ./genesis/scripts/memory-balance.sh --no-save          # just print, don't persist
#   ./genesis/scripts/memory-balance.sh --json             # machine-readable JSON to stdout

set -u

cd "$(git rev-parse --show-toplevel 2>/dev/null || dirname "$0"/..)"

SAVE=1
JSON_ONLY=0
for arg in "$@"; do
  case "$arg" in
    --no-save) SAVE=0 ;;
    --json) JSON_ONLY=1 ;;
  esac
done

TS=$(date +%Y-%m-%d-%H%M)
SNAPDIR=".claude/memory-kit/balance-sheets"
mkdir -p "$SNAPDIR"
JSON_OUT="$SNAPDIR/$TS.json"
TXT_OUT="$SNAPDIR/$TS.txt"

# -------- gospel: canonical CLAUDE.md path-pattern --------
# Gospel = CLAUDE.md files that are always loaded as operating instructions.
# Exclude tooling/script directories and worktree-internal scaffold.
gospel_paths() {
  find . -name "CLAUDE.md" \
    -not -path '*/node_modules/*' \
    -not -path '*/.cargo-target-pool/*' \
    -not -path '*/.git/*' \
    -not -path '*/target/*' \
    -not -path './.claude/scripts/*' \
    -not -path '*/superpowers/*' \
    2>/dev/null
}

# -------- helpers --------
count_lines() {  # path
  if [ ! -d "$1" ]; then echo 0; return; fi
  find "$1" -name "*.md" 2>/dev/null | xargs wc -l 2>/dev/null | tail -1 | awk '{print $1+0}'
}
count_files() {  # path
  if [ ! -d "$1" ]; then echo 0; return; fi
  find "$1" -name "*.md" 2>/dev/null | wc -l
}
count_lang() {  # extension
  find . -path ./node_modules -prune -o -path ./target -prune -o -path '*/.cargo-target-pool' -prune \
    -o -name "*.$1" -print 2>/dev/null | xargs wc -l 2>/dev/null | tail -1 | awk '{print $1+0}'
}
count_stories_status() {  # status
  grep -l "^status: \"\?$1\"\?" genesis/data/stories/*.md 2>/dev/null | wc -l
}
count_archive_orphans() {  # archive entries with disposition:memorialize lacking story_pointer
  local orphans=0
  for f in $(find .claude/archive -name "*.md" 2>/dev/null); do
    if grep -q "^disposition: memorialize" "$f" 2>/dev/null && ! grep -q "^story_pointer:" "$f" 2>/dev/null; then
      orphans=$((orphans+1))
    fi
  done
  echo $orphans
}

# -------- thresholds (gospel-of-this-script: set deliberately, audit if changing) --------
THR_MEMORY_MD_WARN=22500   # 90% of 25KB
THR_MEMORY_MD_OVER=24576   # 24KB — Claude's reported truncation point
THR_GOSPEL_LINES_WARN=3500
THR_GOSPEL_LINES_OVER=5000
THR_SURFACE_ARCHIVE_OK=100
THR_SURFACE_ARCHIVE_WARN=500
THR_STORIES_CANONICAL_MIN=1

# -------- gather --------
RS=$(count_lang rs); TS_LINES=$(count_lang ts); HTML=$(count_lang html); SCSS=$(count_lang scss); PY=$(count_lang py)

GOSPEL_FILES=$(gospel_paths | wc -l)
GOSPEL_LINES=$(gospel_paths | xargs wc -l 2>/dev/null | tail -1 | awk '{print $1+0}')

SURFACE_PLANS=$(count_lines genesis/plans)
SURFACE_SP_PLANS=$(count_lines genesis/docs/superpowers/plans)
SURFACE_SP_SPECS=$(count_lines genesis/docs/superpowers/specs)
SURFACE_SHIFTS=$(count_lines .claude/shifts)
SURFACE_LINES=$((SURFACE_PLANS + SURFACE_SP_PLANS + SURFACE_SP_SPECS + SURFACE_SHIFTS))
SURFACE_FILES=$(($(count_files genesis/plans) + $(count_files genesis/docs/superpowers/plans) + $(count_files genesis/docs/superpowers/specs) + $(count_files .claude/shifts)))

WORKING_LINES=$(find .claude/memory -name "*.md" -not -name MEMORY.md 2>/dev/null | xargs wc -l 2>/dev/null | tail -1 | awk '{print $1+0}')
WORKING_FILES=$(find .claude/memory -name "*.md" -not -name MEMORY.md 2>/dev/null | wc -l)
MEMORY_MD_BYTES=$(stat -c %s .claude/memory/MEMORY.md 2>/dev/null || echo 0)

STORIES_LINES=$(count_lines genesis/data/stories)
STORIES_TOTAL=$(count_files genesis/data/stories)
STORIES_CANONICAL=$(count_stories_status canonical)
STORIES_DRAFT=$(count_stories_status draft)
STORIES_RETIRED=$(count_stories_status retired)
CHRONICLE_LINES=$(count_lines genesis/data/timeline/chronicle)
CHRONICLE_FILES=$(count_files genesis/data/timeline/chronicle)
ROADMAP_LINES=$(count_lines genesis/data/timeline/roadmap)
ROADMAP_FILES=$(count_files genesis/data/timeline/roadmap)
BACKLOG_LINES=$(count_lines genesis/data/timeline/backlog)
BACKLOG_FILES=$(count_files genesis/data/timeline/backlog)
EPIC_LINES=$(count_lines genesis/docs/content/elohim-protocol)
EPIC_FILES=$(count_files genesis/docs/content/elohim-protocol)

ARCHIVE_LINES=$(count_lines .claude/archive)
ARCHIVE_FILES=$(count_files .claude/archive)
ARCHIVE_ORPHANS=$(count_archive_orphans)

MEMPALACE_DRAWERS=$(mempalace status 2>/dev/null | grep -oP 'MemPalace Status — \K[0-9]+' || echo 0)

# -------- ratios --------
if [ "$ARCHIVE_LINES" -gt 0 ]; then
  SURFACE_ARCHIVE_RATIO=$(awk "BEGIN {printf \"%.0f\", $SURFACE_LINES / $ARCHIVE_LINES}")
else
  SURFACE_ARCHIVE_RATIO="inf"
fi
if [ "$STORIES_LINES" -gt 0 ]; then
  WORKING_STORIES_RATIO=$(awk "BEGIN {printf \"%.1f\", $WORKING_LINES / $STORIES_LINES}")
else
  WORKING_STORIES_RATIO="inf"
fi

# -------- health flags --------
flag() {  # value, warn_threshold, over_threshold, direction (over=bigger-worse, under=smaller-worse)
  local v=$1 warn=$2 over=$3 dir=${4:-over}
  if [ "$v" = "inf" ]; then echo "🔴"; return; fi
  if [ "$dir" = "over" ]; then
    if [ "$v" -ge "$over" ]; then echo "🔴"
    elif [ "$v" -ge "$warn" ]; then echo "⚠ "
    else echo "✓ "; fi
  else
    if [ "$v" -lt "$warn" ]; then echo "🔴"
    elif [ "$v" -lt "$over" ]; then echo "⚠ "
    else echo "✓ "; fi
  fi
}
FLAG_MEMORY_MD=$(flag "$MEMORY_MD_BYTES" "$THR_MEMORY_MD_WARN" "$THR_MEMORY_MD_OVER" over)
FLAG_GOSPEL=$(flag "$GOSPEL_LINES" "$THR_GOSPEL_LINES_WARN" "$THR_GOSPEL_LINES_OVER" over)
if [ "$SURFACE_ARCHIVE_RATIO" = "inf" ]; then FLAG_SA="🔴"
elif [ "$SURFACE_ARCHIVE_RATIO" -gt "$THR_SURFACE_ARCHIVE_WARN" ]; then FLAG_SA="🔴"
elif [ "$SURFACE_ARCHIVE_RATIO" -gt "$THR_SURFACE_ARCHIVE_OK" ]; then FLAG_SA="⚠ "
else FLAG_SA="✓ "; fi
if [ "$STORIES_CANONICAL" -ge "$THR_STORIES_CANONICAL_MIN" ]; then FLAG_STORIES="✓ "; else FLAG_STORIES="🔴"; fi
if [ "$ARCHIVE_ORPHANS" -eq 0 ]; then FLAG_ORPHANS="✓ "; else FLAG_ORPHANS="⚠ "; fi

# -------- emit JSON --------
write_json() {
  cat <<EOF
{
  "timestamp": "$TS",
  "code": {
    "rust": $RS, "typescript": $TS_LINES, "html": $HTML, "scss": $SCSS, "python": $PY
  },
  "gospel": {
    "lines": $GOSPEL_LINES, "files": $GOSPEL_FILES, "flag": "$FLAG_GOSPEL"
  },
  "surface": {
    "lines": $SURFACE_LINES, "files": $SURFACE_FILES,
    "plans": $SURFACE_PLANS, "sp_plans": $SURFACE_SP_PLANS,
    "sp_specs": $SURFACE_SP_SPECS, "shifts": $SURFACE_SHIFTS
  },
  "working_memory": {
    "lines": $WORKING_LINES, "files": $WORKING_FILES,
    "memory_md_bytes": $MEMORY_MD_BYTES, "memory_md_flag": "$FLAG_MEMORY_MD"
  },
  "canonical": {
    "stories": {
      "lines": $STORIES_LINES, "total": $STORIES_TOTAL,
      "canonical": $STORIES_CANONICAL, "draft": $STORIES_DRAFT, "retired": $STORIES_RETIRED,
      "flag": "$FLAG_STORIES"
    },
    "chronicle": { "lines": $CHRONICLE_LINES, "files": $CHRONICLE_FILES },
    "roadmap":   { "lines": $ROADMAP_LINES, "files": $ROADMAP_FILES },
    "backlog":   { "lines": $BACKLOG_LINES, "files": $BACKLOG_FILES },
    "epics":     { "lines": $EPIC_LINES, "files": $EPIC_FILES }
  },
  "archive": {
    "lines": $ARCHIVE_LINES, "files": $ARCHIVE_FILES,
    "orphans_missing_story_pointer": $ARCHIVE_ORPHANS,
    "orphans_flag": "$FLAG_ORPHANS"
  },
  "mempalace": { "drawers": $MEMPALACE_DRAWERS },
  "ratios": {
    "surface_to_archive": "$SURFACE_ARCHIVE_RATIO",
    "surface_to_archive_flag": "$FLAG_SA",
    "working_to_stories": "$WORKING_STORIES_RATIO"
  }
}
EOF
}

if [ "$JSON_ONLY" = "1" ]; then
  write_json
  exit 0
fi

# -------- find prior snapshot for diff --------
PRIOR=$(ls -t "$SNAPDIR"/*.json 2>/dev/null | grep -v "^$JSON_OUT\$" | head -1)
prior_val() { [ -z "$PRIOR" ] && echo "" || python3 -c "import json,sys; d=json.load(open('$PRIOR'))
for k in '$1'.split('.'):
  d = d.get(k, '') if isinstance(d, dict) else ''
print(d if d != '' else '-')" 2>/dev/null; }
delta() {  # current, prior_path
  local cur=$1 prior_path=$2
  local prior=$(prior_val "$prior_path")
  if [ -z "$prior" ] || [ "$prior" = "-" ]; then echo "(no prior)"
  else
    local d=$((cur - prior))
    if [ "$d" -gt 0 ]; then echo "(+$d)"
    elif [ "$d" -lt 0 ]; then echo "($d)"
    else echo "(=)"; fi
  fi
}

# -------- emit text --------
{
echo "================================================================"
echo " MEMORY BALANCE SHEET — $TS"
[ -n "$PRIOR" ] && echo " Prior snapshot: $(basename "$PRIOR" .json)" || echo " Prior snapshot: none (first run)"
echo "================================================================"
echo ""
echo "--- CODE (source — context for ratios below) ---"
printf "  %-44s %8s\n" "Rust"        "$RS"
printf "  %-44s %8s\n" "TypeScript"  "$TS_LINES"
printf "  %-44s %8s\n" "HTML"        "$HTML"
printf "  %-44s %8s\n" "SCSS"        "$SCSS"
printf "  %-44s %8s\n" "Python"      "$PY"
echo ""
echo "--- GOSPEL (always-loaded operating instructions) ---"
printf "  $FLAG_GOSPEL %-42s %8s lines / %4s files  %s\n" "CLAUDE.md (allowlist-filtered)" "$GOSPEL_LINES" "$GOSPEL_FILES" "$(delta "$GOSPEL_LINES" gospel.lines)"
echo ""
echo "--- SURFACE OF COMET (live work — distill before archive) ---"
printf "     %-42s %8s lines  %s\n" "genesis/plans"                  "$SURFACE_PLANS"   "$(delta "$SURFACE_PLANS" surface.plans)"
printf "     %-42s %8s lines  %s\n" "genesis/docs/superpowers/plans" "$SURFACE_SP_PLANS" "$(delta "$SURFACE_SP_PLANS" surface.sp_plans)"
printf "     %-42s %8s lines  %s\n" "genesis/docs/superpowers/specs" "$SURFACE_SP_SPECS" "$(delta "$SURFACE_SP_SPECS" surface.sp_specs)"
printf "     %-42s %8s lines  %s\n" ".claude/shifts"                 "$SURFACE_SHIFTS"   "$(delta "$SURFACE_SHIFTS" surface.shifts)"
printf "     %-42s %8s lines / %4s files\n" "TOTAL SURFACE" "$SURFACE_LINES" "$SURFACE_FILES"
echo ""
echo "--- WORKING MEMORY (crystallized lessons) ---"
printf "     %-42s %8s lines / %4s files  %s\n" "topic files" "$WORKING_LINES" "$WORKING_FILES" "$(delta "$WORKING_LINES" working_memory.lines)"
MEM_KB=$(awk "BEGIN {printf \"%.1fKB\", $MEMORY_MD_BYTES/1024}")
printf "  $FLAG_MEMORY_MD %-42s %8s  %s\n" "MEMORY.md size" "$MEM_KB" "$(delta "$MEMORY_MD_BYTES" working_memory.memory_md_bytes)"
echo ""
echo "--- CANONICAL (wisdom tier — preserved) ---"
printf "  $FLAG_STORIES %-42s canonical=%s draft=%s retired=%s\n" "stories (status breakdown)" "$STORIES_CANONICAL" "$STORIES_DRAFT" "$STORIES_RETIRED"
printf "     %-42s %8s lines / %4s files\n" "  stories total"         "$STORIES_LINES"   "$STORIES_TOTAL"
printf "     %-42s %8s lines / %4s files\n" "timeline/chronicle"      "$CHRONICLE_LINES" "$CHRONICLE_FILES"
printf "     %-42s %8s lines / %4s files\n" "timeline/roadmap"        "$ROADMAP_LINES"   "$ROADMAP_FILES"
printf "     %-42s %8s lines / %4s files\n" "timeline/backlog"        "$BACKLOG_LINES"   "$BACKLOG_FILES"
printf "     %-42s %8s lines / %4s files\n" "epics (elohim-protocol)" "$EPIC_LINES"      "$EPIC_FILES"
echo ""
echo "--- ARCHIVE (Isildur's-diary tier — distilled, retrievable via story-pointer) ---"
printf "     %-42s %8s lines / %4s files  %s\n" ".claude/archive/" "$ARCHIVE_LINES" "$ARCHIVE_FILES" "$(delta "$ARCHIVE_LINES" archive.lines)"
printf "  $FLAG_ORPHANS %-42s %8s entries  (memorialize without story_pointer)\n" "archive orphans" "$ARCHIVE_ORPHANS"
echo ""
echo "--- MEMPALACE (searchable view across tiers) ---"
printf "     %-42s %8s drawers  %s\n" "total drawers" "$MEMPALACE_DRAWERS" "$(delta "$MEMPALACE_DRAWERS" mempalace.drawers)"
echo ""
echo "--- DIAGNOSTIC RATIOS ---"
printf "  $FLAG_SA %-42s %12s : 1   (healthy: <%s:1)\n" "Surface : Archive" "$SURFACE_ARCHIVE_RATIO" "$THR_SURFACE_ARCHIVE_OK"
printf "     %-42s %12s : 1   (target: 2–3:1 long-term)\n" "Working memory : Stories"   "$WORKING_STORIES_RATIO"
echo ""
echo "================================================================"
echo " Legend:  ✓ healthy   ⚠  approaching threshold   🔴 over"
echo "================================================================"
} | tee "$TXT_OUT"

if [ "$SAVE" = "1" ]; then
  write_json > "$JSON_OUT"
  echo ""
  echo " Snapshot persisted:"
  echo "   $JSON_OUT"
  echo "   $TXT_OUT"
fi
