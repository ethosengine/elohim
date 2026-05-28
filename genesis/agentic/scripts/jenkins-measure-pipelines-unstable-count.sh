#!/usr/bin/env bash
# Counts how many orchestrator-triggered pipelines on `dev` have lastCompletedBuild
# result == SUCCESS or UNSTABLE. Returns single integer 0..N on stdout.
#
# Designed for /shift Objectives that target "all pipelines UNSTABLE-or-better"
# (no FAILURE across the dispatch surface). Anonymous reads — no auth required
# per the Jenkins anonymous-mode (OIDC) memory entry.
#
# Pipelines list mirrors the orchestrator's PIPELINES map in
# genesis/orchestrator/Jenkinsfile (excluding steward which is manual-only).
set -euo pipefail

JENKINS_URL="${JENKINS_URL:-https://jenkins.elohim.host}"
BRANCH="${BRANCH:-dev}"

PIPELINES=(
  "elohim-orchestrator"
  "elohim-app"
  "elohim-edge"
  "elohim-dna-lamad"
  "elohim-dna-mishpat"
  "elohim-genesis"
  "elohim-sophia"
)

count=0
for pipeline in "${PIPELINES[@]}"; do
  url="$JENKINS_URL/job/$pipeline/job/$BRANCH/lastCompletedBuild/api/json?tree=result"
  response=$(curl -sS --globoff --max-time 15 "$url" 2>/dev/null || echo '{}')
  result=$(printf '%s' "$response" | node -e 'let d; try { d = JSON.parse(require("fs").readFileSync(0,"utf8")); } catch { d = {}; } console.log(d.result || "");' 2>/dev/null || echo '')
  case "$result" in
    SUCCESS|UNSTABLE) count=$((count + 1)) ;;
    *) ;;
  esac
done

echo "$count"
