#!/usr/bin/env bash
# Measure helper for /shift Objective `drive-genesis-e2e-verification-quality`.
#
# Returns a single integer summarising the failure surface visible in the
# latest completed elohim-genesis/dev build's sprint-report.md.
#
# Decision tree (lower is better):
#
#   1000  — sprint-report.md is missing or returns empty body (artifact pipeline broken)
#   500   — report exists but reports `scenarios = 0` (e2e stage runs but executes
#           nothing — common cascade symptom of seed/discovery failure)
#   N     — `failed` count from the report's Summary table (working verification,
#           N failing scenarios)
#   0     — report exists, scenarios > 0, failed = 0 (clean)
#
# This covers integration-iteration mode's full state space:
#   • report-pipeline broken → 1000 penalty drives candidate A
#   • no-scenarios-running → 500 penalty drives candidate A' (run-the-tests)
#   • scenarios run with failures → drive each failing class as a candidate
#
# Reads the sprint-report via anonymous Overall.Read (the same access path
# ci-observer uses). Requires `JENKINS_URL` env var.
set -euo pipefail

: "${JENKINS_URL:?JENKINS_URL must be set}"

URL="$JENKINS_URL/job/elohim-genesis/job/dev/lastCompletedBuild/artifact/genesis/a2o/reports/sprint-report.md"

report=$(curl -sSf "$URL" 2>/dev/null || true)

if [ -z "$report" ]; then
  echo 1000
  exit 0
fi

# Parse the Summary table. Expected format:
#   | scenarios | passed | failed | skipped | pending |
#   |---|---|---|---|---|
#   | N | N | N | N | N |
#
# We extract the data row (first table row after the header separator) and
# pull `scenarios` (col 1) and `failed` (col 3).
data_row=$(printf '%s\n' "$report" | awk '
  /^\|---/ { found_sep = 1; next }
  found_sep && /^\| *[0-9]+ *\|/ { print; exit }
')

if [ -z "$data_row" ]; then
  # Couldn't find the data row — unknown format, treat as broken
  echo 999
  exit 0
fi

scenarios=$(printf '%s\n' "$data_row" | awk -F'|' '{ gsub(/ /,"",$2); print $2 }')
failed=$(printf '%s\n' "$data_row" | awk -F'|' '{ gsub(/ /,"",$4); print $4 }')

scenarios="${scenarios:-0}"
failed="${failed:-0}"

if [ "$scenarios" = "0" ]; then
  echo 500
  exit 0
fi

echo "$failed"
