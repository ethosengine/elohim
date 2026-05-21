#!/usr/bin/env bash
# count-pipeline-failures.sh — emit single integer: # of elohim-* pipelines
# whose lastCompletedBuild on dev resulted in FAILURE.
#
# Used as the measure command for agentic shifts driving the orchestrator
# pipeline graph toward UNSTABLE-or-better. Reads anonymous Jenkins API
# — passing JENKINS_USERNAME/JENKINS_TOKEN triggers the OIDC commenceLogin
# redirect on this server, so we deliberately do NOT send Basic auth.
#
# Exit code 0 with one integer on stdout; on Jenkins error we print 99
# (sentinel — measurement untrustworthy) and exit 0 so the loop doesn't
# crash mid-shift.

set -u
JENKINS_URL="${JENKINS_URL:-https://jenkins.ethosengine.com}"

# Auto-dispatched pipelines on dev (manual-only ones excluded).
# Source of truth: genesis/orchestrator/orchestrator-strategy.mjs PIPELINES.
PIPELINES=(
  "elohim-orchestrator"
  "elohim-holochain"
  "elohim-edge"
  "elohim"
  "elohim-genesis"
  "elohim-sophia"
  "elohim-epr"
  "elohim-storybook"
)

fails=0
unreachable=0
for p in "${PIPELINES[@]}"; do
  url="$JENKINS_URL/job/$p/job/dev/lastCompletedBuild/api/json?tree=result"
  # NO auth header — sending one triggers OIDC redirect; anonymous works.
  body=$(curl -sf -m 10 "$url" 2>/dev/null) || { unreachable=$((unreachable + 1)); continue; }
  result=$(printf '%s' "$body" | jq -r '.result // "UNKNOWN"' 2>/dev/null)
  case "$result" in
    FAILURE) fails=$((fails + 1)) ;;
  esac
done

# If more than half the pipelines were unreachable, the measurement is
# untrustworthy — return sentinel 99 so the loop knows to retry rather
# than mistake reachability failure for green pipelines.
total="${#PIPELINES[@]}"
if [ "$unreachable" -gt $((total / 2)) ]; then
  echo 99
else
  echo "$fails"
fi
