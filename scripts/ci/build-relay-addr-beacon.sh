#!/bin/bash
# build-relay-addr-beacon.sh — build + tag the relay-addr-beacon sidecar image.
# Own dir as build context, zero internal path-deps (see relay-addr-beacon/).
# Builds under the SHORT local name (relay-addr-beacon:<tag>) exactly like the
# elohim-agent-sdk build in the Jenkinsfile — a full-registry-name build under
# BUILDKIT_HOST does not land in the local `nerdctl -n k8s.io images` store the
# push stage greps, so the push silently skipped and coturn ImagePullBackOff'd
# (edge #1193). push-relay-addr-beacon.sh re-tags short -> full registry.
#
# Env (exported by withBuildVars in the Jenkinsfile):
#   IMAGE_TAG          local build tag to apply
#   GIT_COMMIT_HASH    full commit hash (provenance tag)
set -euo pipefail

echo "Building Relay Addr Beacon: ${IMAGE_TAG}"
buildctl --addr unix:///run/buildkit/buildkitd.sock debug workers > /dev/null

BUILDKIT_HOST=unix:///run/buildkit/buildkitd.sock \
    nerdctl -n k8s.io build \
    -t relay-addr-beacon:${IMAGE_TAG} \
    -f relay-addr-beacon/Dockerfile relay-addr-beacon

nerdctl -n k8s.io tag relay-addr-beacon:${IMAGE_TAG} relay-addr-beacon:${GIT_COMMIT_HASH}
