#!/usr/bin/env bash
# Jenkins-based measure helper for /shift Objectives.
#
# Usage: jenkins-measure.sh <job-path>
# Example: jenkins-measure.sh elohim-edge/job/dev
#
# Prints a single integer:
#   1  if the job's lastCompletedBuild has result == SUCCESS
#   0  otherwise (FAILURE, UNSTABLE, ABORTED, no build yet, or API error)
#
# Requires JENKINS_URL and JENKINS_TOKEN env vars.
set -euo pipefail

JOB_PATH="${1:?usage: jenkins-measure.sh <job-path like elohim-edge/job/dev>}"
: "${JENKINS_URL:?JENKINS_URL must be set}"
: "${JENKINS_TOKEN:?JENKINS_TOKEN must be set}"

response=$(curl -sSf --globoff -H "Jenkins-Token: $JENKINS_TOKEN" \
  "$JENKINS_URL/job/$JOB_PATH/lastCompletedBuild/api/json?tree=result" 2>/dev/null || echo '{}')

result=$(printf '%s' "$response" | node -e 'let d; try { d = JSON.parse(require("fs").readFileSync(0,"utf8")); } catch { d = {}; } console.log(d.result || "");')

if [[ "$result" == "SUCCESS" ]]; then
  echo 1
else
  echo 0
fi
