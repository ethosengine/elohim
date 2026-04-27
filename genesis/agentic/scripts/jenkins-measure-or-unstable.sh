#!/usr/bin/env bash
# Jenkins-based measure helper for /shift Objectives — UNSTABLE-or-better variant.
#
# Usage: jenkins-measure-or-unstable.sh <job-path>
# Example: jenkins-measure-or-unstable.sh elohim-orchestrator/job/dev
#
# Prints a single integer:
#   1  if the job's lastCompletedBuild has result == SUCCESS or UNSTABLE
#   0  otherwise (FAILURE, ABORTED, NOT_BUILT, no build yet, or API error)
#
# Sibling of jenkins-measure.sh (which is SUCCESS-only). Use this when the
# Objective treats UNSTABLE as acceptable — e.g. orchestrator pipelines where
# the goal is end-to-end execution rather than every stage green.
#
# Requires JENKINS_URL and JENKINS_TOKEN env vars.
set -euo pipefail

JOB_PATH="${1:?usage: jenkins-measure-or-unstable.sh <job-path like elohim-orchestrator/job/dev>}"
: "${JENKINS_URL:?JENKINS_URL must be set}"
: "${JENKINS_TOKEN:?JENKINS_TOKEN must be set}"

response=$(curl -sSf --globoff -H "Jenkins-Token: $JENKINS_TOKEN" \
  "$JENKINS_URL/job/$JOB_PATH/lastCompletedBuild/api/json?tree=result" 2>/dev/null || echo '{}')

result=$(printf '%s' "$response" | node -e 'let d; try { d = JSON.parse(require("fs").readFileSync(0,"utf8")); } catch { d = {}; } console.log(d.result || "");')

case "$result" in
  SUCCESS|UNSTABLE) echo 1 ;;
  *)                echo 0 ;;
esac
