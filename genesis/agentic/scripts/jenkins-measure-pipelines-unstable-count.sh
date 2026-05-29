#!/usr/bin/env bash
# Counts how many orchestrator-triggered pipelines on `dev` have lastCompletedBuild
# result == SUCCESS or UNSTABLE. Returns single integer 0..N on stdout.
#
# Designed for /shift Objectives that target "all pipelines UNSTABLE-or-better"
# (no FAILURE across the dispatch surface). Anonymous Jenkins reads — no auth
# header per the Jenkins anonymous-mode (OIDC) memory entry; reads work, writes
# don't.
#
# Pipeline names mirror the orchestrator's PIPELINES map in
# genesis/orchestrator/Jenkinsfile, EXCLUDING:
#   - elohim-orchestrator (the meta-pipeline; its result is a roll-up of these)
#   - elohim-steward (manual-only; manualOnly: true in the PIPELINES map)
#   - elohim-epr (parent job not yet created in Jenkins — separate config gap;
#     orchestrator references it but the multibranch parent returns 404 even
#     with auth, so it can't be measured)
#
# Note: the DNA pipeline is `elohim-holochain` (one job that packs all DNAs:
# elohim, hrea, imagodei, infrastructure, lamad-v1, mishpat, node-registry);
# there is NO separate `elohim-dna-lamad` or `elohim-dna-mishpat` parent job.
# The app pipeline is `elohim` (NOT `elohim-app`).
#
# JENKINS_URL defaults to https://jenkins.ethosengine.com (the actual host —
# `jenkins.elohim.host` is a different earlier hostname that doesn't resolve).
set -euo pipefail

JENKINS_URL="${JENKINS_URL:-https://jenkins.ethosengine.com}"
BRANCH="${BRANCH:-dev}"

PIPELINES=(
  "elohim"
  "elohim-edge"
  "elohim-genesis"
  "elohim-holochain"
  "elohim-sophia"
  "elohim-storybook"
)

count=0
for pipeline in "${PIPELINES[@]}"; do
  url="$JENKINS_URL/job/$pipeline/job/$BRANCH/lastCompletedBuild/api/json?tree=result"
  response=$(curl -sS --globoff --max-time 15 "$url" 2>/dev/null || echo '{}')
  # Reject HTML responses (404 / sign-in pages) — only JSON counts
  if ! echo "$response" | head -c 1 | grep -q '{'; then
    continue
  fi
  result=$(printf '%s' "$response" | node -e 'let d; try { d = JSON.parse(require("fs").readFileSync(0,"utf8")); } catch { d = {}; } console.log(d.result || "");' 2>/dev/null || echo '')
  case "$result" in
    SUCCESS|UNSTABLE) count=$((count + 1)) ;;
    *) ;;
  esac
done

echo "$count"
