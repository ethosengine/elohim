#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# build-storage-canary.sh — build + push a CANARY elohim-storage image that embeds
# a chosen conductor (e.g. the jemalloc-prof leak-hunt conductor) via the storage
# Dockerfile's CONDUCTOR_SOURCE_IMAGE arg.
#
# WHY THIS EXISTS (companion to edgenode/build-zombie-fix.sh): the conductor is
# COPY'd into the storage image at BUILD time (elohim-storage/Dockerfile:13,231 —
# `FROM ${CONDUCTOR_SOURCE_IMAGE}` → `COPY --from=conductor-source …/holochain`),
# so swapping the conductor on a live pod needs a rebuilt STORAGE image. The auto
# edge pipeline builds storage from the Dockerfile DEFAULT (elohim-edgenode:latest)
# and DEPLOYS FLEET-WIDE — wrong for a profiling canary (overhead on all 14 pods,
# genesis pair included). This builds a distinct canary tag and DOES NOT deploy;
# the operator `kubectl set image`s it onto ONE pod.
#
# OPT-IN / OUT-OF-BAND ON PURPOSE — like build-zombie-fix.sh, run this as a MANUAL
# Jenkins job / one-off. It pushes `:${TAG}` (default jemalloc-prof-canary-<hash>);
# it does NOT touch deploy manifests. Nothing auto-deploys.
#
# Usage (from anywhere in the repo):
#   CONDUCTOR_SOURCE_IMAGE=harbor.ethosengine.com/ethosengine/elohim-edgenode:zombie-fix-canary-<hash> \
#     ./build-storage-canary.sh [push]
# Env:
#   CONDUCTOR_SOURCE_IMAGE  REQUIRED — the conductor/edgenode image to embed
#                           (build it first via edgenode/build-zombie-fix.sh with
#                            HC_FEATURES=sqlite-encrypted,wasmer_sys,backend-go-pion,jemalloc-prof)
#   REGISTRY   target registry/namespace (default harbor.ethosengine.com/ethosengine)
#   TAG        image tag (default jemalloc-prof-canary-<short HEAD>)
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
REGISTRY="${REGISTRY:-harbor.ethosengine.com/ethosengine}"
GIT_HASH="$(git -C "$REPO_ROOT" rev-parse --short HEAD 2>/dev/null || echo dev)"
TAG="${TAG:-jemalloc-prof-canary-${GIT_HASH}}"
IMAGE="${REGISTRY}/elohim-storage:${TAG}"
DOCKERFILE="${REPO_ROOT}/elohim/elohim-storage/Dockerfile"

if [ -z "${CONDUCTOR_SOURCE_IMAGE:-}" ]; then
  echo "ERROR: CONDUCTOR_SOURCE_IMAGE is required (the conductor image to embed)." >&2
# ─── RESTORED 2026-06-22: the lines below were reconstructed from the script's own
#     documented spec after an accidental deletion (the verbatim original was captured
#     only through the line above). Behavior matches the header; please verify the build
#     context / build-args against your intent before relying on it. ───────────────────
  echo "  e.g. CONDUCTOR_SOURCE_IMAGE=${REGISTRY}/elohim-edgenode:zombie-fix-canary-<hash> $0 [push]" >&2
  exit 1
fi

echo "▶ building canary storage image:"
echo "    image:    ${IMAGE}"
echo "    conductor:${CONDUCTOR_SOURCE_IMAGE}"
echo "    dockerfile:${DOCKERFILE}"
echo "    context:  ${REPO_ROOT}"

docker build \
  --build-arg "CONDUCTOR_SOURCE_IMAGE=${CONDUCTOR_SOURCE_IMAGE}" \
  -t "${IMAGE}" \
  -f "${DOCKERFILE}" \
  "${REPO_ROOT}"

echo "✔ built ${IMAGE}"

if [ "${1:-}" = "push" ]; then
  echo "▶ pushing ${IMAGE}"
  docker push "${IMAGE}"
  echo "✔ pushed ${IMAGE} — operator: kubectl set image onto ONE pod (does NOT auto-deploy)"
else
  echo "ℹ built only (no push). Re-run with 'push' to publish: $0 push"
fi
