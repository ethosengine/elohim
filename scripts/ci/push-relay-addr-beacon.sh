#!/bin/bash
# push-relay-addr-beacon.sh — ensure the relay-addr-beacon image exists locally,
# then tag + push it to Harbor (:dev-latest on dev + :<commit-hash>). The coturn
# manifests pull :dev-latest.
#
# WHY THIS IS SELF-HEALING (edge #1193 full-name, #1194 short-name both failed):
# the image builds + tags cleanly in the Build Agent SDK stage (log shows
# "Loaded image: docker.io/library/relay-addr-beacon:<tag>" and the provenance
# tag succeeds), yet the prior gate — `nerdctl images | grep "name.*tag"` run in
# the LATER Push stage — missed it and silently skipped the push, so coturn
# ImagePullBackOff'd. The structurally identical elohim-agent-sdk push in the
# same stage matched, so the fault is the cross-stage table-text gate, not the
# name. This script instead (1) checks existence by EXACT reference and (2) if
# the local image is not resolvable at push time, rebuilds it in place (buildkit
# is available in the Push stage's builder container) — so the push can never
# silently skip again. It always exits 0: the beacon is non-critical delivery
# (the Jenkinsfile catchError-isolates its build), and a push failure must not
# abort the edge deploy — set -x keeps any failure visible in the console.
#
# Env: IMAGE_TAG, GIT_COMMIT_HASH, BRANCH_NAME
set -uo pipefail
set -x

IMAGE="harbor.ethosengine.com/ethosengine/relay-addr-beacon"
LOCAL="relay-addr-beacon:${IMAGE_TAG}"

echo "=== relay-addr-beacon images in k8s.io at push time (diagnostic) ==="
nerdctl -n k8s.io images | grep -i relay-addr-beacon || echo "(no relay-addr-beacon image in k8s.io at push time)"

ensure_and_push() {
    # Exact-reference existence check (robust vs. parsing the images table text).
    if ! nerdctl -n k8s.io image inspect "${LOCAL}" >/dev/null 2>&1; then
        echo "Local ${LOCAL} not resolvable at push time — rebuilding in the push stage."
        buildctl --addr unix:///run/buildkit/buildkitd.sock debug workers > /dev/null || return 1
        BUILDKIT_HOST=unix:///run/buildkit/buildkitd.sock \
            nerdctl -n k8s.io build \
            -t "${LOCAL}" \
            -f relay-addr-beacon/Dockerfile relay-addr-beacon || return 1
    fi

    echo "Pushing Relay Addr Beacon provenance tag: ${GIT_COMMIT_HASH}"
    nerdctl -n k8s.io tag "${LOCAL}" "${IMAGE}:${GIT_COMMIT_HASH}" || return 1
    nerdctl -n k8s.io push "${IMAGE}:${GIT_COMMIT_HASH}" || return 1

    if [ "${BRANCH_NAME}" = "dev" ]; then
        nerdctl -n k8s.io tag "${LOCAL}" "${IMAGE}:dev-latest" || return 1
        nerdctl -n k8s.io push "${IMAGE}:dev-latest" || return 1
    fi
    return 0
}

if ensure_and_push; then
    echo "✓ relay-addr-beacon pushed to Harbor (${IMAGE}:dev-latest)"
else
    echo "⚠ relay-addr-beacon push failed (non-critical) — coturn keeps its current image until the next edge build. See the trace above."
fi

exit 0
