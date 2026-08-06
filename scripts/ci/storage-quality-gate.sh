#!/bin/bash
# storage-quality-gate.sh — CI home for the elohim-storage lib suite (1835+
# tests incl. the CRDT sync/heal proofs), via the Dockerfile's `check` target.
#
# Before this gate the edge pipeline only BUILT the storage image, so a
# storage-logic regression rode to prod as long as it compiled (found
# 2026-07-01, crdt-content-dataplane handoff §3). Mirrors the doorway gate:
# `--target check` reuses the BuildKit dep-layer cache, so no Rust toolchain
# is needed in the CI builder image.
#
# Deliberately NO CACHE_BUST: COPY layers key on content hashes, so an
# unchanged source tree is a full cache hit (seconds) and a changed tree
# re-runs fmt+tests exactly once. (CACHE_BUST is load-bearing only in the
# release path, where GIT_COMMIT_* args must be re-baked per commit.)
#
# Called from elohim/holochain/Jenkinsfile runStorageQualityGate() inside
# catchError→UNSTABLE + a 45-minute stage-local timeout.
set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

# Dead-config lint first — cheap (grep-only, no toolchain), and a knob with no
# reader is a design defect worth catching before spending Docker build time.
# See dead-config-lint.sh: ListDocumentsSince + AnnounceChange were fully
# handled on the receive side and never constructed for months (charter article
# sync-scale-honesty) before this check existed.
bash "$WORKSPACE_ROOT/scripts/ci/dead-config-lint.sh"

buildctl --addr unix:///run/buildkit/buildkitd.sock debug workers > /dev/null

# Tagged so superseded gate runs become prunable dangling images instead of
# accumulating anonymous ones in the k8s.io namespace.
BUILDKIT_HOST=unix:///run/buildkit/buildkitd.sock \
    nerdctl -n k8s.io build \
    --target check \
    -t elohim-storage-check:ci \
    -f elohim/elohim-storage/Dockerfile .
