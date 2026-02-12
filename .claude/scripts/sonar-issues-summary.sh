#!/bin/bash
# Fetch SonarQube issues and produce a compact summary for agent consumption.
#
# Usage:
#   ./sonar-issues-summary.sh                    # All issues, compact summary
#   ./sonar-issues-summary.sh --severity HIGH    # Filter by severity
#   ./sonar-issues-summary.sh --full             # Full details (for scripts)
#   ./sonar-issues-summary.sh --raw              # Save raw JSON only
#
# Output: Writes to .claude/sonar-issues-summary.json (compact)
#         and .claude/sonar-issues-raw.json (full API response)
#
# This script exists because SonarQube MCP responses are huge (~50KB per page)
# and destroy agent context windows. Run this script via Bash, then read the
# compact summary file.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
OUTPUT_DIR="$PROJECT_DIR/.claude"
RAW_FILE="$OUTPUT_DIR/sonar-issues-raw.json"
SUMMARY_FILE="$OUTPUT_DIR/sonar-issues-summary.json"

# SonarQube config
SONAR_URL="${SONAR_HOST_URL:-https://sonarqube.ethosengine.com}"
# Project keys: elohim-app-alpha, doorway-alpha, sophia-alpha
SONAR_PROJECT="${SONAR_PROJECT_KEY:-elohim-app-alpha}"
SONAR_TOKEN="${SONAR_TOKEN:-${SONARQUBE_TOKEN:-}}"

# Parse args
SEVERITY_FILTER=""
MODE="summary"  # summary | full | raw
while [[ $# -gt 0 ]]; do
  case $1 in
    --severity) SEVERITY_FILTER="$2"; shift 2 ;;
    --full) MODE="full"; shift ;;
    --raw) MODE="raw"; shift ;;
    --project) SONAR_PROJECT="$2"; shift 2 ;;
    *) echo "Unknown arg: $1"; exit 1 ;;
  esac
done

if [ -z "$SONAR_TOKEN" ]; then
  echo "Error: SONAR_TOKEN or SONARQUBE_TOKEN env var required"
  echo "Set it with: export SONARQUBE_TOKEN=your-token"
  exit 1
fi

# Fetch all pages
echo "Fetching SonarQube issues for $SONAR_PROJECT..." >&2

ALL_ISSUES="[]"
PAGE=1
TOTAL=0

while true; do
  URL="$SONAR_URL/api/issues/search?projects=$SONAR_PROJECT&ps=500&p=$PAGE&resolved=false"
  if [ -n "$SEVERITY_FILTER" ]; then
    URL="$URL&impactSeverities=$SEVERITY_FILTER"
  fi

  RESPONSE=$(curl -s -u "$SONAR_TOKEN:" "$URL")

  # Check for error
  if echo "$RESPONSE" | jq -e '.errors' >/dev/null 2>&1; then
    echo "Error from SonarQube API:" >&2
    echo "$RESPONSE" | jq '.errors' >&2
    exit 1
  fi

  TOTAL=$(echo "$RESPONSE" | jq '.paging.total')
  PAGE_ISSUES=$(echo "$RESPONSE" | jq '.issues')
  PAGE_COUNT=$(echo "$PAGE_ISSUES" | jq 'length')

  ALL_ISSUES=$(echo "$ALL_ISSUES" "$PAGE_ISSUES" | jq -s 'add')

  FETCHED=$(echo "$ALL_ISSUES" | jq 'length')
  echo "  Page $PAGE: $PAGE_COUNT issues (total fetched: $FETCHED / $TOTAL)" >&2

  if [ "$FETCHED" -ge "$TOTAL" ]; then
    break
  fi

  PAGE=$((PAGE + 1))

  # Safety: max 10 pages (5000 issues)
  if [ "$PAGE" -gt 10 ]; then
    echo "  Warning: Stopping at 5000 issues (10 pages)" >&2
    break
  fi
done

# Save raw
echo "$ALL_ISSUES" > "$RAW_FILE"
echo "Raw issues saved to $RAW_FILE ($FETCHED issues)" >&2

if [ "$MODE" = "raw" ]; then
  echo "$RAW_FILE"
  exit 0
fi

# Build compact summary
jq '
  # Group by rule
  group_by(.rule) |
  map({
    rule: .[0].rule,
    severity: .[0].impacts[0].severity,
    category: .[0].impacts[0].softwareQuality,
    count: length,
    files: [.[] | {
      file: (.component | split(":")[1] // .component),
      line: .line,
      message: (.message | if length > 120 then .[:120] + "..." else . end)
    }]
  }) |
  sort_by(-.count)
' "$RAW_FILE" > "$SUMMARY_FILE"

# Print human-readable summary
echo "" >&2
echo "=== SonarQube Issues Summary ===" >&2
echo "Project: $SONAR_PROJECT" >&2
echo "Total issues: $FETCHED" >&2
echo "" >&2

jq -r '
  "By severity:",
  (group_by(.severity) | map("  \(.[0].severity): \(map(.count) | add)") | .[]),
  "",
  "By rule (top 20):",
  (.[:20][] | "  \(.count)\t\(.severity)\t\(.rule)")
' "$SUMMARY_FILE" >&2

echo "" >&2
echo "Summary saved to $SUMMARY_FILE" >&2
echo "$SUMMARY_FILE"
