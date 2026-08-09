#!/usr/bin/env bash
# push-storage-iroh.sh — build + push the PER-COMMIT iroh-transport lane of the
# storage image (harbor.ethosengine.com/ethosengine/elohim-storage-iroh:${STORAGE_TAG}).
#
# WHY THIS EXISTS (edge #1319 RCA, 2026-08-07): alpha's storage container was
# rendered from the one-time artifact `elohim-storage-iroh:hc-elohim-0.6.3-iroh`,
# built once on 2026-08-06 by the che-devworkspaces edgenode job. That tag never
# moves, so every storage-side merge to dev after the Wave-2 iroh flip was
# SILENTLY INERT on alpha — pods restarted on each deploy and re-pulled the same
# frozen digest. The fail-closed fleet-quiesce gate finally surfaced it as
# actionable=None / unmeasured=None (the honesty-floor gauge could not exist
# because the code that emits it had never shipped). This script makes the iroh
# lane track dev exactly like the tx5 lane does.
#
# WHAT IT BUILDS: storage-iroh/Dockerfile — a two-line binary-swap overlay
# (per-commit storage service + iroh conductor). No Rust rebuild: the expensive
# work already happened in stage('Build Storage'). See that Dockerfile's header
# for the full recipe-provenance and rollback-contract notes.
#
# ROLLBACK CONTRACT: this script NEVER writes `:hc-elohim-0.6.3-iroh` (storage or
# edgenode). It reads the edgenode anchor as a build base and publishes only
# `:${STORAGE_TAG}`, `:${GIT_COMMIT_HASH}`, and (on dev) `:dev-latest`.
#
# FAILURE POSTURE — deliberately asymmetric:
#   * storage image NOT built this run  -> echo + exit 0. Mirrors
#     push-harbor-untracked.sh's "image not found, skipping push". A doorway-only
#     edge build must not start failing; the tx5 lane has the identical hole and
#     it is orthogonal to this change.
#   * storage image built, iroh build/push fails -> exit non-zero, FAILING the
#     Push stage. This is NOT the non-critical posture used by the relay/beacon
#     scripts: resolveStorageImage() points every alpha human at
#     elohim-storage-iroh:${STORAGE_TAG}, so a missing tag would ImagePullBackOff
#     the whole alpha namespace. Better to abort before the deploy stage runs.
#
# Env (exported by withBuildVars in elohim/holochain/Jenkinsfile):
#   IMAGE_TAG              local build tag applied by stage('Build Storage')
#   STORAGE_TAG            the storage component tag the alpha render resolves
#   GIT_COMMIT_HASH        full commit hash (provenance tag)
#   BRANCH_NAME            branch under build (dev gets the floating alias)
# Optional:
#   IROH_CONDUCTOR_SOURCE  override the iroh conductor base (digest pin, or a new
#                          conductor branch's anchor) without editing the Dockerfile
set -euo pipefail

REGISTRY="harbor.ethosengine.com/ethosengine"
STORAGE_IROH="${REGISTRY}/elohim-storage-iroh"
LOCAL_BUILT="elohim-storage:${IMAGE_TAG}"
LOCAL_IROH="elohim-storage-iroh:${IMAGE_TAG}"
STORAGE_BASE="${REGISTRY}/elohim-storage:${STORAGE_TAG}"
# 2026-08-09: repointed to the relay-fallback conductor (fork e4a1c9bb2 —
# cross-relay preflight fix, vendored kitsune2_transport_iroh patch). The
# previous anchor hc-elohim-0.6.3-iroh is UNTOUCHED and remains the D9
# rollback anchor.
CONDUCTOR_SOURCE="${IROH_CONDUCTOR_SOURCE:-${REGISTRY}/elohim-edgenode-iroh:hc-elohim-0.6.3-relay-fallback-iroh}"

# Exact-reference existence check (never `nerdctl images | grep -q`: grep -q
# exits at first match and SIGPIPEs a still-writing nerdctl, which under
# pipefail reads as "not found" — that trap cost edge #1266-#1273).
if ! nerdctl -n k8s.io image inspect "${LOCAL_BUILT}" >/dev/null 2>&1; then
    echo "elohim-storage:${IMAGE_TAG} not built this run — skipping the iroh lane."
    echo "(The tx5 lane's push is skipped on the same condition; alpha keeps its"
    echo " currently-deployed image because no new STORAGE_TAG reached Harbor.)"
    exit 0
fi

echo "=== elohim-storage-iroh (Wave-2 iroh transport lane) ==="
echo "  storage base    : ${STORAGE_BASE}"
echo "  conductor source: ${CONDUCTOR_SOURCE}"
echo "  publishing      : ${STORAGE_IROH}:${STORAGE_TAG}"

buildctl --addr unix:///run/buildkit/buildkitd.sock debug workers > /dev/null

# Own dir as build context, zero internal path-deps (same shape as
# iroh-relay/ and relay-addr-beacon/). Built under the SHORT local name first —
# a full-registry-name build under BUILDKIT_HOST does not land in the local
# `nerdctl -n k8s.io` image store (edge #1193) — then re-tagged to the registry.
BUILDKIT_HOST=unix:///run/buildkit/buildkitd.sock \
    nerdctl -n k8s.io build \
    --build-arg STORAGE_BASE="${STORAGE_BASE}" \
    --build-arg CONDUCTOR_SOURCE="${CONDUCTOR_SOURCE}" \
    -t "${LOCAL_IROH}" \
    -f storage-iroh/Dockerfile storage-iroh

nerdctl -n k8s.io tag "${LOCAL_IROH}" "${STORAGE_IROH}:${STORAGE_TAG}"
nerdctl -n k8s.io tag "${LOCAL_IROH}" "${STORAGE_IROH}:${GIT_COMMIT_HASH}"

nerdctl -n k8s.io push "${STORAGE_IROH}:${STORAGE_TAG}"
nerdctl -n k8s.io push "${STORAGE_IROH}:${GIT_COMMIT_HASH}"

# Floating alias for canary/manual repoints. A DISTINCT tag from the ratified
# rollback anchor `:hc-elohim-0.6.3-iroh`, which this script never writes.
if [ "${BRANCH_NAME}" = "dev" ]; then
    nerdctl -n k8s.io tag "${LOCAL_IROH}" "${STORAGE_IROH}:dev-latest"
    nerdctl -n k8s.io push "${STORAGE_IROH}:dev-latest"
fi

echo "✓ elohim-storage-iroh pushed: ${STORAGE_IROH}:${STORAGE_TAG}"
