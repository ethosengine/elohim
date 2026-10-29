#!/bin/bash
# build-storage-image.sh — build + tag the elohim-storage release image.
# Extracted verbatim from the edge Jenkinsfile's Build Storage stage
# (2026-07-01: pipeline block at the JVM 64KB CPS limit; bash bodies live in
# scripts/ci per the Jenkinsfile-size canon).
#
# Env (exported by withBuildVars in the Jenkinsfile):
#   STORAGE_TAG / DOORWAY_TAG: closure tags; *_CLOSURE / *_CLOSURE_MATERIAL: keys
#   GIT_COMMIT_HASH    full commit hash (provenance)
set -euo pipefail

# The doorway build uses this same recipe with build-doorway. Image config
# labels preserve closure identity across registry manifest conversions.
STEP="${1:-build-storage}"
case "$STEP" in
    build-storage) IMAGE=elohim-storage; TAG="${STORAGE_TAG:?}"; KEY="${STORAGE_CLOSURE:?}"; MATERIAL="${STORAGE_CLOSURE_MATERIAL:?}"; DOCKERFILE=elohim/elohim-storage/Dockerfile ;;
    build-doorway) IMAGE=elohim-doorway; TAG="${DOORWAY_TAG:?}"; KEY="${DOORWAY_CLOSURE:?}"; MATERIAL="${DOORWAY_CLOSURE_MATERIAL:?}"; DOCKERFILE=doorway/doorway-service/Dockerfile ;;
    *) exit 2 ;;
esac
[[ "$KEY" =~ ^sha256:[a-f0-9]{64}$ && "$MATERIAL" =~ ^sha256:[a-f0-9]{64}$ ]] || exit 2
[[ "$GIT_COMMIT_HASH" =~ ^[a-f0-9]{40}$ ]] || exit 2
hex="${KEY#sha256:}"
[ "$TAG" = "closure-${hex:0:16}" ] || exit 2
buildctl --addr unix:///run/buildkit/buildkitd.sock debug workers > /dev/null
args=()
if [ "$STEP" = build-storage ]; then
# ── Conductor pin: resolved from the submodule pointer, not hand-edited ──────
# elohim-storage embeds the holochain conductor (embedded-conductor mode). Which
# conductor is decided HERE, by the SHA this commit's elohim/holochain-conductor
# submodule pointer records — the same SHA the orchestrator hands the
# elohim-edgenode job as HC_REF, which that job publishes as
# `elohim-edgenode:conductor-<sha12>`.
#
# This replaces a hand-edited `sha256:` ARG in the Dockerfile. That pin had to be
# bumped by a human after every conductor build, which meant a rebuilt conductor
# silently did NOT reach the fleet; and being a bare digest it was un-diagnosable
# when Harbor GC'd it (edge #1261).
#
# `rev-parse HEAD:<path>` reads the gitlink from the commit object, so it works
# for these `update = none` submodules that CI never clones.
# Holochain 0.7 has one iroh transport line, so the conductor gitlink alone
# determines the source-derived tag. elohim/kitsune2 is deliberately absent: its
# sibling-clone path patches are commented out in the fork, so that submodule
# cannot change the binary. Keep this derivation identical to CONDUCTOR_PIN_TAG in
# che-devworkspaces jenkins/Jenkinsfile-elohim-edgenode — the two ends must agree
# or edge asks for a tag the conductor job never published.
CONDUCTOR_REGISTRY="harbor.ethosengine.com/ethosengine/elohim-edgenode"
CONDUCTOR_SHA="$(git rev-parse "HEAD:elohim/holochain-conductor" 2>/dev/null || true)"

if [ -z "${CONDUCTOR_SHA}" ]; then
    echo "FATAL: cannot read the conductor fork submodule pointer."
    echo "  elohim/holochain-conductor = '${CONDUCTOR_SHA:-<unreadable>}'"
    echo "  The storage image embeds that conductor; without the pointer there is"
    echo "  no way to know which conductor this commit means. Refusing to guess."
    exit 1
fi

CONDUCTOR_PIN="${CONDUCTOR_REGISTRY}:conductor-$(git rev-parse HEAD:elohim/holochain-conductor | cut -c1-12)"
echo "Conductor pin: ${CONDUCTOR_PIN}"
echo "  holochain-conductor ${CONDUCTOR_SHA}"

# No pre-flight pull: `nerdctl login harbor` does not run until the push stage,
# so a pull here would fail on auth even when the image exists. Buildkit resolves
# the FROM with the node's own registry config and is the authority. If it can't,
# the build below fails and the trap explains why in the pin's terms.
conductor_pin_hint() {
    echo ""
    echo "── If the failure above is a missing/unauthorized ${CONDUCTOR_PIN} ──"
    echo "  The committed conductor source has never been built, or its tag was"
    echo "  garbage-collected. Publish it, then re-run this pipeline:"
    echo ""
    echo "    git commit --allow-empty -m 'build(conductor): publish pin [build:conductor]' && git push"
    echo ""
    echo "  There is deliberately no fallback conductor. A storage image whose"
    echo "  conductor does not match its committed source is the exact failure"
    echo "  this pin exists to prevent."
}
trap 'conductor_pin_hint' ERR

    args+=(--build-arg "CONDUCTOR_SOURCE_IMAGE=${CONDUCTOR_PIN}")
fi

# CACHE_BUST is the closure key; unrelated commit ids never bust the cargo layer.
# Pull the pinned base image (matches the Dockerfile FROM; keep in sync).
nerdctl -n k8s.io pull rust:1.94.0-slim-bookworm || true

BUILDKIT_HOST=unix:///run/buildkit/buildkitd.sock \
    nerdctl -n k8s.io build \
    --build-arg CACHE_BUST="${KEY}" \
    --build-arg GIT_COMMIT_SHORT="${GIT_COMMIT_HASH:0:7}" \
    --build-arg GIT_COMMIT_FULL="${GIT_COMMIT_HASH}" \
    --build-arg BUILD_TIMESTAMP="$(date -u +'%Y-%m-%dT%H:%M:%SZ')" \
    --build-arg RUSTC_VERSION="$(rustc --version 2>/dev/null | head -1 || echo unknown)" \
    "${args[@]}" \
    --label "elohim.host/closure=${KEY}" \
    --label "elohim.host/closure-material=${MATERIAL}" \
    --label "org.opencontainers.image.revision=${GIT_COMMIT_HASH}" \
    -t "${IMAGE}:${TAG}" \
    -f "$DOCKERFILE" .
