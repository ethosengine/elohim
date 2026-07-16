#!/bin/bash
# push-relay-addr-beacon.sh — tag + push the relay-addr-beacon image to Harbor.
# The coturn manifests pull the floating :dev-latest tag (static, no _TAG_PLACEHOLDER
# render), so we push :dev-latest (on dev branch) plus a :<commit-hash> provenance
# tag — no per-version tag is needed. Extracted out of the Push to Harbor stage's
# inline sh block (Jenkinsfile-size canon in CLAUDE.md; bash bodies live in scripts/ci).
#
# Env (exported by the pipeline):
#   IMAGE_TAG         local build tag the image was built under
#   GIT_COMMIT_HASH   full commit hash (provenance tag)
#   BRANCH_NAME       gates the dev-latest alias push
set -euo pipefail

IMAGE="harbor.ethosengine.com/ethosengine/relay-addr-beacon"

if nerdctl -n k8s.io images | grep -q "relay-addr-beacon.*${IMAGE_TAG}"; then
    echo "Pushing Relay Addr Beacon provenance tag: ${GIT_COMMIT_HASH}"
    nerdctl -n k8s.io tag "${IMAGE}:${IMAGE_TAG}" "${IMAGE}:${GIT_COMMIT_HASH}"
    nerdctl -n k8s.io push "${IMAGE}:${GIT_COMMIT_HASH}"

    if [ "${BRANCH_NAME}" = "dev" ]; then
        nerdctl -n k8s.io tag "${IMAGE}:${IMAGE_TAG}" "${IMAGE}:dev-latest"
        nerdctl -n k8s.io push "${IMAGE}:dev-latest"
    fi
else
    echo "Relay Addr Beacon image not found, skipping push"
fi
