#!/bin/bash
# build-iroh-relay.sh — build + tag the iroh-relay server image.
# Own dir as build context, zero internal path-deps (see iroh-relay/).
# Builds under the SHORT local name (iroh-relay:<tag>) exactly like the
# relay-addr-beacon build in the Jenkinsfile — a full-registry-name build under
# BUILDKIT_HOST does not land in the local `nerdctl -n k8s.io images` store the
# push stage greps, so the push silently skips (edge #1193). push-iroh-relay.sh
# re-tags short -> full registry.
#
# Env (exported by withBuildVars in the Jenkinsfile):
#   IMAGE_TAG            local build tag to apply
#   GIT_COMMIT_HASH      full commit hash (provenance tag)
#   IROH_RELAY_VERSION   iroh-relay crate version (default below)
set -euo pipefail

export IROH_RELAY_VERSION="${IROH_RELAY_VERSION:-0.95.1}"

echo "Building iroh-relay: ${IMAGE_TAG} (iroh-relay crate ${IROH_RELAY_VERSION})"
buildctl --addr unix:///run/buildkit/buildkitd.sock debug workers > /dev/null

BUILDKIT_HOST=unix:///run/buildkit/buildkitd.sock \
    nerdctl -n k8s.io build \
    --build-arg IROH_RELAY_VERSION="${IROH_RELAY_VERSION}" \
    -t iroh-relay:${IMAGE_TAG} \
    -f iroh-relay/Dockerfile iroh-relay

nerdctl -n k8s.io tag iroh-relay:${IMAGE_TAG} iroh-relay:${GIT_COMMIT_HASH}

# Diagnostic: prove the image is in the k8s.io store immediately after build+tag.
# If it is present here but absent when push-iroh-relay.sh runs in the later
# Push stage, the image is being evicted between stages (that script rebuilds
# on demand to survive it).
echo "=== iroh-relay images in k8s.io immediately after build ==="
nerdctl -n k8s.io images | grep -i iroh-relay || echo "(WARN: iroh-relay NOT in k8s.io right after build)"
