#!/bin/bash
# push-relay-addr-beacon.sh — tag + push the relay-addr-beacon image to Harbor.
# The image is built under the SHORT local name relay-addr-beacon:<IMAGE_TAG>
# (see build-relay-addr-beacon.sh); here we tag it to the full registry name and
# push :dev-latest (on dev) + :<commit-hash>. The coturn manifests pull
# :dev-latest. Mirrors the elohim-agent-sdk push block in the Jenkinsfile.
#
# Env: IMAGE_TAG, GIT_COMMIT_HASH, BRANCH_NAME
set -euo pipefail

IMAGE="harbor.ethosengine.com/ethosengine/relay-addr-beacon"

if nerdctl -n k8s.io images | grep -q "relay-addr-beacon.*${IMAGE_TAG}"; then
    echo "Pushing Relay Addr Beacon provenance tag: ${GIT_COMMIT_HASH}"
    nerdctl -n k8s.io tag relay-addr-beacon:${IMAGE_TAG} "${IMAGE}:${GIT_COMMIT_HASH}"
    nerdctl -n k8s.io push "${IMAGE}:${GIT_COMMIT_HASH}"

    if [ "${BRANCH_NAME}" = "dev" ]; then
        nerdctl -n k8s.io tag relay-addr-beacon:${IMAGE_TAG} "${IMAGE}:dev-latest"
        nerdctl -n k8s.io push "${IMAGE}:dev-latest"
    fi
else
    echo "Relay Addr Beacon image not found, skipping push"
fi
