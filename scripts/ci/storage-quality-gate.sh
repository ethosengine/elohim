#!/bin/bash
# storage-quality-gate.sh — CI home for the elohim-storage lib suite (1835+
# tests incl. the CRDT sync/heal proofs), via the Dockerfile's `check` target.
#
# Before this gate the edge pipeline only BUILT the storage image, so a
# storage-logic regression rode to prod as long as it compiled (found
# 2026-07-01, crdt-content-dataplane handoff §3). Mirrors the doorway gate:
# `--target check` reuses the BuildKit dep-layer cache, so no Rust toolchain
# is needed in the CI builder image. This check-only solve omits the image
# exporter: its result is the exit status, not a runtime artifact.
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
STORAGE_BUILD_MANIFEST="${WORKSPACE_ROOT}/elohim/holochain/build-manifest.json"

# Keep the Edge check target on the same resource contract as the local gate.
# Node is already part of the CI builder; the manifest remains the one source
# for elohim-storage's cargo environment instead of duplicating the cap here.
STORAGE_CARGO_BUILD_JOBS="$(node -e '
const fs = require("node:fs");
const manifest = JSON.parse(fs.readFileSync(process.argv[1], "utf8"));
const value = manifest.gate?.projects?.["elohim-storage"]?.run?.cargo?.env?.CARGO_BUILD_JOBS;
if (!/^[1-9][0-9]*$/.test(value ?? "")) {
  console.error("elohim-storage gate CARGO_BUILD_JOBS must be a positive integer");
  process.exit(1);
}
process.stdout.write(value);
' "${STORAGE_BUILD_MANIFEST}")"

# Dead-config lint first — cheap (grep-only, no toolchain), and a knob with no
# reader is a design defect worth catching before spending Docker build time.
# See dead-config-lint.sh: ListDocumentsSince + AnnounceChange were fully
# handled on the receive side and never constructed for months (habit
# sync-scale-honesty) before this check existed.
bash "$WORKSPACE_ROOT/scripts/ci/dead-config-lint.sh"

buildctl --addr unix:///run/buildkit/buildkitd.sock debug workers > /dev/null

cd "${WORKSPACE_ROOT}"
buildctl --addr unix:///run/buildkit/buildkitd.sock build \
    --frontend dockerfile.v0 \
    --local context=. \
    --local dockerfile=. \
    --opt filename=elohim/elohim-storage/Dockerfile \
    --opt target=check \
    --opt build-arg:CARGO_BUILD_JOBS="${STORAGE_CARGO_BUILD_JOBS}" \
    --progress plain
