#!/bin/bash
# push-iroh-relay.sh — ensure the iroh-relay image exists locally, then tag +
# push it to Harbor (:<version>-<commit-hash> always, :<version>-dev-latest on
# dev). The doorway manifests pull :<version>-dev-latest.
#
# WHY THIS IS SELF-HEALING (mirrors push-relay-addr-beacon.sh, edge #1193/#1194):
# a cross-stage `nerdctl images | grep "name.*tag"` table-text gate can miss an
# image that built and tagged cleanly earlier in the pipeline, silently
# skipping the push. This script instead (1) checks existence by EXACT
# reference and (2) if the local image is not resolvable at push time,
# rebuilds it in place (buildkit is available in the Push stage's builder
# container) — so the push can never silently skip again. It always exits 0:
# the relay image is non-critical delivery (the Jenkinsfile catchError-isolates
# its build), and a push failure must not abort the edge deploy — set -x keeps
# any failure visible in the console.
#
# The tag is VERSION-SCOPED (iroh-relay:0.95.1-dev-latest, not a bare
# iroh-relay:dev-latest) so a future crate bump (e.g. 1.0.3) publishes under
# its own dev-latest line instead of clobbering the 0.95.1 one the manifests
# currently pin.
#
# Env: IMAGE_TAG, GIT_COMMIT_HASH, BRANCH_NAME, IROH_RELAY_VERSION
set -uo pipefail
set -x

IROH_RELAY_VERSION="${IROH_RELAY_VERSION:-0.95.1}"
IMAGE="harbor.ethosengine.com/ethosengine/iroh-relay"
LOCAL="iroh-relay:${IMAGE_TAG}"

echo "=== iroh-relay images in k8s.io at push time (diagnostic) ==="
nerdctl -n k8s.io images | grep -i iroh-relay || echo "(no iroh-relay image in k8s.io at push time)"

ensure_and_push() {
    # Exact-reference existence check (robust vs. parsing the images table text).
    if ! nerdctl -n k8s.io image inspect "${LOCAL}" >/dev/null 2>&1; then
        echo "Local ${LOCAL} not resolvable at push time — rebuilding in the push stage."
        buildctl --addr unix:///run/buildkit/buildkitd.sock debug workers > /dev/null || return 1
        BUILDKIT_HOST=unix:///run/buildkit/buildkitd.sock \
            nerdctl -n k8s.io build \
            --build-arg IROH_RELAY_VERSION="${IROH_RELAY_VERSION}" \
            -t "${LOCAL}" \
            -f iroh-relay/Dockerfile iroh-relay || return 1
    fi

    echo "Pushing iroh-relay provenance tag: ${IROH_RELAY_VERSION}-${GIT_COMMIT_HASH}"
    nerdctl -n k8s.io tag "${LOCAL}" "${IMAGE}:${IROH_RELAY_VERSION}-${GIT_COMMIT_HASH}" || return 1
    nerdctl -n k8s.io push "${IMAGE}:${IROH_RELAY_VERSION}-${GIT_COMMIT_HASH}" || return 1

    if [ "${BRANCH_NAME}" = "dev" ]; then
        nerdctl -n k8s.io tag "${LOCAL}" "${IMAGE}:${IROH_RELAY_VERSION}-dev-latest" || return 1
        nerdctl -n k8s.io push "${IMAGE}:${IROH_RELAY_VERSION}-dev-latest" || return 1
    fi
    return 0
}

if ensure_and_push; then
    echo "✓ iroh-relay pushed to Harbor (${IMAGE}:${IROH_RELAY_VERSION}-dev-latest)"
else
    echo "⚠ iroh-relay push failed (non-critical) — the relay pods keep their current image until the next edge build. See the trace above."
fi

exit 0
