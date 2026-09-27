#!/usr/bin/env bash
# await-happ-candidate.sh — the hApp barrier INSIDE the edge pipeline.
#
# The orchestrator starts the DNA job and the edge job side by side
# (genesis/orchestrator/Jenkinsfile BARRIER_PAIRS). The DNA job publishes its
# packed hApp as `elohim-happ:candidate-<commit8>` right after `Build DNA`
# (scripts/ci/push-happ-candidate.sh), before its sweettest. Edge needs those
# bytes only when it packages them, so it waits here instead of the
# orchestrator waiting for the whole DNA job.
#
# Usage: await-happ-candidate.sh <candidate-tag> <deadline-secs>
#   exit 0  the tag's manifest resolves (HAPP-CANDIDATE-PRESENT line)
#   exit 3  still absent at the deadline (HAPP-CANDIDATE-ABSENT line); the
#           caller marks the build UNSTABLE and falls back loudly
#   exit 2  usage: an empty or floating tag is refused, because a moving tag
#           cannot name one commit's bytes
# Env: HARBOR_USER/HARBOR_PASS (optional oras login), AWAIT_POLL_SECS (default 15).
#
# Price (habit push-delivers-within-budget, genesis/orchestrator/.epr-meta): the
# edge caller passes 2400 s. The wait normally costs nothing, because edge spends
# longer building doorway + storage than DNA spends in `Build DNA`; it replaces
# a job-level wait on the WHOLE DNA job (sweettest included, ~49 min) that every
# DNA+edge run paid before edge could start at all.
set -uo pipefail

TAG="${1:-}"
DEADLINE="${2:-}"
POLL="${AWAIT_POLL_SECS:-15}"
REF="harbor.ethosengine.com/ethosengine/elohim-happ:${TAG}"

case "${TAG}" in
    ''|dev-latest|latest)
        echo "await-happ-candidate: refusing tag '${TAG}' — pass the per-commit candidate tag" >&2
        exit 2 ;;
esac
[[ "${DEADLINE}" =~ ^[0-9]+$ ]] || { echo "usage: await-happ-candidate.sh <candidate-tag> <deadline-secs>" >&2; exit 2; }

ORAS="$(bash "$(dirname "$0")/ensure-oras.sh")" || { echo "await-happ-candidate: no oras CLI" >&2; exit 3; }
if [ -n "${HARBOR_USER:-}" ]; then
    printf '%s' "${HARBOR_PASS:-}" | "${ORAS}" login harbor.ethosengine.com -u "${HARBOR_USER}" --password-stdin >/dev/null 2>&1 \
        || echo "await-happ-candidate: oras login failed — polling anonymously" >&2
fi

START=${SECONDS}
echo "await-happ-candidate: waiting up to ${DEADLINE}s for ${REF} (poll ${POLL}s)"
while :; do
    if "${ORAS}" manifest fetch "${REF}" >/dev/null 2>&1; then
        echo "HAPP-CANDIDATE-PRESENT tag=${TAG} after $((SECONDS - START))s"
        exit 0
    fi
    ELAPSED=$((SECONDS - START))
    if [ "${ELAPSED}" -ge "${DEADLINE}" ]; then
        echo "HAPP-CANDIDATE-ABSENT tag=${TAG} after ${ELAPSED}s"
        exit 3
    fi
    REMAINING=$((DEADLINE - ELAPSED))
    sleep $(( POLL < REMAINING ? POLL : REMAINING ))
done
