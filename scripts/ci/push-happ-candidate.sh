#!/usr/bin/env bash
# push-happ-candidate.sh — publish the DNA pipeline's packed hApp as the
# per-commit CANDIDATE `elohim-happ:candidate-<commit8>`, right after
# `Build DNA` and before the sweettest.
#
# The edge pipeline, started beside the DNA job by the orchestrator
# (genesis/orchestrator/Jenkinsfile BARRIER_PAIRS), waits for exactly this
# reference (scripts/ci/await-happ-candidate.sh) instead of the orchestrator
# waiting for the whole DNA job. A candidate is NOT an attestation: edge still
# refuses to deploy until the DNA run it names ends SUCCESS (awaitDnaVerdict).
# The floating tags the fleet follows move only at the DNA job's
# `Push to Harbor` stage, after the sweettest; this script never names them.
#
# Usage (cwd = elohim/holochain/dna/elohim, as `Build DNA` leaves it):
#   push-happ-candidate.sh <commit8>
#   The commit is the DNA build's GIT_COMMIT_HASH (git rev-parse HEAD | cut -c1-8),
#   the same value the orchestrator derives from its checkout.
# Env: HARBOR_USER/HARBOR_PASS (harbor-robot-registry).
# Exit: 0 pushed · 2 usage (not a lowercase hex commit) · other = oras failure.
set -uo pipefail

COMMIT="${1:-}"
if ! [[ "${COMMIT}" =~ ^[0-9a-f]{7,40}$ ]]; then
    echo "usage: push-happ-candidate.sh <commit8> — got '${COMMIT}'" >&2
    exit 2
fi
REF="harbor.ethosengine.com/ethosengine/elohim-happ:candidate-${COMMIT}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

ORAS="$(bash "${SCRIPT_DIR}/ensure-oras.sh")" || { echo "push-happ-candidate: no oras CLI" >&2; exit 1; }
printf '%s' "${HARBOR_PASS:-}" | "${ORAS}" login harbor.ethosengine.com -u "${HARBOR_USER:-}" --password-stdin || exit 1

# Content roll key (scripts/ci/happ-roll-key.sh). No coordinator hot-swap has
# run yet, so this is the conservative bundle key; `Push to Harbor` stamps the
# final key on the versioned tag after the hot-swap.
ROLL_KEY="$(bash "${SCRIPT_DIR}/happ-roll-key.sh" dna-hashes.actual workdir/happ.yaml elohim.happ || true)"
ANNOTATION=()
[ -n "${ROLL_KEY}" ] && ANNOTATION=(--annotation "elohim.host/roll-key=${ROLL_KEY}")
echo "hApp candidate ${REF} (roll key ${ROLL_KEY:-<none>})"

"${ORAS}" push "${REF}" "${ANNOTATION[@]}" elohim.happ:application/octet-stream
