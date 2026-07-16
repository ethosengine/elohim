#!/bin/bash
# build-relay-addr-beacon.sh — build + tag the relay-addr-beacon sidecar image.
# Own dir as build context, zero internal path-deps (see relay-addr-beacon/).
# Extracted out of the Build Agent SDK stage's script{} closure (2026-07-16:
# that body sits at the per-file 11000B CPS-method-size cliff — see the
# Jenkinsfile-size canon in CLAUDE.md; bash bodies live in scripts/ci).
#
# Env (exported by withBuildVars in the Jenkinsfile):
#   IMAGE_TAG          local build tag to apply
#   GIT_COMMIT_HASH    full commit hash (provenance tag)
set -euo pipefail

IMAGE="harbor.ethosengine.com/ethosengine/relay-addr-beacon"

echo "Building Relay Addr Beacon: ${IMAGE_TAG}"
buildctl --addr unix:///run/buildkit/buildkitd.sock debug workers > /dev/null

BUILDKIT_HOST=unix:///run/buildkit/buildkitd.sock \
    nerdctl -n k8s.io build \
    -t "${IMAGE}:${IMAGE_TAG}" \
    -f relay-addr-beacon/Dockerfile relay-addr-beacon

nerdctl -n k8s.io tag "${IMAGE}:${IMAGE_TAG}" "${IMAGE}:${GIT_COMMIT_HASH}"
