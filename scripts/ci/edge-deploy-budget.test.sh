#!/usr/bin/env bash
# Hermetic test for edge-deploy-budget.sh — the arithmetic that refuses an edge
# deploy the edge run's own timeout would kill mid-roll.
set -euo pipefail

REPO_ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
SCRIPT="${REPO_ROOT}/scripts/ci/edge-deploy-budget.sh"
EDGE="${REPO_ROOT}/elohim/holochain/Jenkinsfile"

fail() { echo "FAIL: $*" >&2; exit 1; }
START_MS=1000000000000   # an arbitrary build start, epoch ms
at() { # at <minutes-after-start> <timeout-min> <need-min> → RC, OUT
  set +e
  OUT="$(NOW_MS=$((START_MS + $1 * 60000)) bash "${SCRIPT}" "${START_MS}" "$2" "$3" 2>&1)"
  RC=$?
  set -e
}

# 1. early verdict: plenty left → exit 0, names the margin.
at 60 240 115
[ "${RC}" -eq 0 ] || fail "(1) exited ${RC}: ${OUT}"
grep -q '^deploy budget: 180 min left, roll+validation needs 115$' <<< "${OUT}" || fail "(1) line: ${OUT}"
echo "ok 1 verdict at 60 min → deploy (180 left)"

# 2. exactly enough is enough.
at 125 240 115
[ "${RC}" -eq 0 ] || fail "(2) exited ${RC}: ${OUT}"
echo "ok 2 verdict at 125 min → deploy (115 left, boundary)"

# 3. a late verdict would cut validation or the roll → refuse, named, exit 3.
at 126 240 115
[ "${RC}" -eq 3 ] || fail "(3) exited ${RC} (want 3): ${OUT}"
grep -q '^deploy refused: 114 min left, roll+validation needs 115$' <<< "${OUT}" || fail "(3) line: ${OUT}"
at 200 240 115
[ "${RC}" -eq 3 ] || fail "(3b) exited ${RC}: ${OUT}"
grep -q '^deploy refused: 40 min left, roll+validation needs 115$' <<< "${OUT}" || fail "(3b) line: ${OUT}"
echo "ok 3 verdict at 126/200 min → refused with the named line"

# 4. past the timeout (clock skew) never goes negative in the line.
at 250 240 115
[ "${RC}" -eq 3 ] || fail "(4) exited ${RC}: ${OUT}"
grep -q '^deploy refused: 0 min left' <<< "${OUT}" || fail "(4) line: ${OUT}"
echo "ok 4 past the timeout → 0 min left, refused"

# 5. malformed input is a usage error (2), not a silent deploy.
for args in "" "abc 240 115" "${START_MS} x 115" "${START_MS} 240"; do
  set +e; OUT="$(bash "${SCRIPT}" ${args} 2>&1)"; RC=$?; set -e
  [ "${RC}" -eq 2 ] || fail "(5) '${args}' exited ${RC} (want 2): ${OUT}"
done
echo "ok 5 malformed input → exit 2"

# 6. the Jenkinsfile passes the same timeout its options block declares.
OPT="$(sed -n '/^pipeline {/,$p' "${EDGE}" | grep -m1 -oE "timeout\(time: [0-9]+, unit: 'MINUTES'\)" | grep -oE '[0-9]+')"
grep -qE "edge-deploy-budget\.sh' \\\$\{currentBuild\.startTimeInMillis\} ${OPT} 115" "${EDGE}" \
  || fail "(6) the Jenkinsfile must pass its options timeout (${OPT}) to edge-deploy-budget.sh"
echo "ok 6 Jenkinsfile passes its own options timeout (${OPT} min)"

echo "PASS edge-deploy-budget.test.sh"
