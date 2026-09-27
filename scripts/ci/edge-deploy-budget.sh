#!/usr/bin/env bash
# edge-deploy-budget.sh — may this edge run still start its fleet roll?
#
# Edge runs beside the DNA job (genesis/orchestrator/Jenkinsfile BARRIER_PAIRS)
# and waits for the DNA verdict before it deploys (awaitDnaVerdict), so a late
# verdict eats the edge run's own options timeout. A roll the timeout kills
# halfway leaves peers on mixed images; a refused deploy is honest. So after a
# SUCCESS verdict the deploy starts only if the time left covers the roll plus
# the validation that follows it.
#
# Usage: edge-deploy-budget.sh <build-start-epoch-ms> <timeout-min> <need-min>
#   exit 0  "deploy budget: <n> min left, roll+validation needs <m>"
#   exit 3  "deploy refused: <n> min left, roll+validation needs <m>"
#   exit 2  usage (never a silent pass)
# Env: NOW_MS (tests; default the current clock).
#
# Price (habit push-delivers-within-budget): this adds no wait — it refuses in
# milliseconds instead of letting the timeout kill a roll.
set -uo pipefail

START_MS="${1:-}"; TIMEOUT_MIN="${2:-}"; NEED_MIN="${3:-}"
for v in "${START_MS}" "${TIMEOUT_MIN}" "${NEED_MIN}"; do
    [[ "${v}" =~ ^[0-9]+$ ]] || { echo "usage: edge-deploy-budget.sh <build-start-epoch-ms> <timeout-min> <need-min>" >&2; exit 2; }
done
NOW_MS="${NOW_MS:-$(( $(date +%s) * 1000 ))}"

ELAPSED_MIN=$(( (NOW_MS - START_MS) / 60000 ))
LEFT=$(( TIMEOUT_MIN - ELAPSED_MIN ))
[ "${LEFT}" -lt 0 ] && LEFT=0

if [ "${LEFT}" -ge "${NEED_MIN}" ]; then
    echo "deploy budget: ${LEFT} min left, roll+validation needs ${NEED_MIN}"
    exit 0
fi
echo "deploy refused: ${LEFT} min left, roll+validation needs ${NEED_MIN}"
exit 3
