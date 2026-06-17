#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# build-zombie-fix.sh — build + push the CANARY edgenode image carrying the
# tx5/go-pion zombie-PeerConnection leak fix (tx5 #194/#199 + holochain #5719).
#
# OPT-IN / OUT-OF-BAND ON PURPOSE. This is NOT wired into the auto edge pipeline:
#   - The edge Jenkinsfile also builds doorway + storage; a heavy from-source
#     holochain build (CGo, ~30 min, toolchain-fragile) that fails would turn the
#     whole edge pipeline red and block unrelated deploys.
#   - That Jenkinsfile is near the CPS method-size limit (see CLAUDE.md — it has
#     breached before). Don't grow it for this.
#   Run this as a MANUAL Jenkins job / one-off build when you're ready to canary.
#
# BUILD-ONLY: it pushes a distinct `:zombie-fix-canary-*` tag. It does NOT touch
#   the deploy manifests (genesis/orchestrator/manifests/edgenode/*.yaml still
#   resolve EDGENODE_TAG_PLACEHOLDER → the normal image). Nothing auto-deploys.
#
# ⚠ NOT CLEARED FOR DEPLOY until BOTH are verified (see
#   .claude/data/conductor-leak-deploy-recipe-2026-06-17.md):
#     1. exact go-pion-custom production feature set vs holo-host's recipe
#     2. kitsune2 0.3.2 ↔ 0.3.0-dev.3 wire-compat (canary one non-genesis leecher)
#
# Usage (from repo root or this dir):
#   REGISTRY=harbor.ethosengine.com/ethosengine ./build-zombie-fix.sh [push]
# Env:
#   REGISTRY   target registry/namespace (default harbor.ethosengine.com/ethosengine)
#   GIT_HASH   tag suffix (default: short HEAD of this repo)
#   HC_FEATURES override the conductor feature set (see Dockerfile.zombie-fix)
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REGISTRY="${REGISTRY:-harbor.ethosengine.com/ethosengine}"
GIT_HASH="${GIT_HASH:-$(git -C "$HERE" rev-parse --short HEAD 2>/dev/null || echo dev)}"
IMAGE="${REGISTRY}/elohim-edgenode:zombie-fix-canary-${GIT_HASH}"
DOCKERFILE="${HERE}/Dockerfile.zombie-fix"

echo "── Building CANARY edgenode (zombie-fix) ──"
echo "  image:      ${IMAGE}"
echo "  dockerfile: ${DOCKERFILE}"
echo "  context:    ${HERE}"
echo "  NOTE: canary tag; deploy manifests are untouched; NOT cleared for fleet deploy."

# buildkit is what the edge pipeline uses; `docker build` works too. The build
# args (fork URLs/branches, feature set) have sane defaults in the Dockerfile.
BUILD_ARGS=()
[ -n "${HC_FEATURES:-}" ] && BUILD_ARGS+=(--build-arg "HC_FEATURES=${HC_FEATURES}")

if command -v buildctl >/dev/null 2>&1 && [ -n "${BUILDKIT_HOST:-}" ]; then
  buildctl build \
    --frontend dockerfile.v0 \
    --local context="${HERE}" \
    --local dockerfile="${HERE}" \
    --opt filename=Dockerfile.zombie-fix \
    $( [ -n "${HC_FEATURES:-}" ] && echo --opt build-arg:HC_FEATURES="${HC_FEATURES}" ) \
    --output type=image,name="${IMAGE}",push=true
else
  docker build -f "${DOCKERFILE}" -t "${IMAGE}" "${BUILD_ARGS[@]}" "${HERE}"
  if [ "${1:-}" = "push" ]; then docker push "${IMAGE}"; fi
fi

echo "── Done: ${IMAGE} ──"
echo "Canary deploy (operator, when the two checks pass): point ONE non-genesis"
echo "leecher's edgenode StatefulSet at this tag, confirm smaps_anon{class=other}"
echo "FLATTENS + gossip/DHT healthy, THEN roll wider (genesis pair last)."
