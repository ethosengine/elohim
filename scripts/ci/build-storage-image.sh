#!/bin/bash
# build-storage-image.sh — build + tag the elohim-storage release image.
# Extracted verbatim from the edge Jenkinsfile's Build Storage stage
# (2026-07-01: pipeline block at the JVM 64KB CPS limit; bash bodies live in
# scripts/ci per the Jenkinsfile-size canon).
#
# Env (exported by withBuildVars in the Jenkinsfile):
#   IMAGE_TAG          image tag for this build (e.g. 1.0.0-dev-<sha>)
#   GIT_COMMIT_HASH    full commit hash (cache-bust + provenance args)
set -euo pipefail

buildctl --addr unix:///run/buildkit/buildkitd.sock debug workers > /dev/null

# CACHE_BUST=${GIT_COMMIT_HASH} forces re-execution of the cargo build layer
# when source changes. BuildKit retains layer cache for unchanged COPY+RUN
# pairs (base image, apt installs, cargo deps), which is what we want —
# removing --no-cache restores ~15 min savings on rebuilds with no source
# change. Base-image freshness is addressed by pinning the FROM tag.

# Pull the pinned base image (matches the Dockerfile FROM; keep in sync).
nerdctl -n k8s.io pull rust:1.94.0-slim-bookworm || true

BUILDKIT_HOST=unix:///run/buildkit/buildkitd.sock \
    nerdctl -n k8s.io build \
    --build-arg CACHE_BUST="${GIT_COMMIT_HASH}" \
    --build-arg GIT_COMMIT_SHORT="$(echo "${GIT_COMMIT_HASH}" | cut -c1-7)" \
    --build-arg GIT_COMMIT_FULL="${GIT_COMMIT_HASH}" \
    --build-arg BUILD_TIMESTAMP="$(date -u +'%Y-%m-%dT%H:%M:%SZ')" \
    --build-arg RUSTC_VERSION="$(rustc --version 2>/dev/null | head -1 || echo unknown)" \
    -t "elohim-storage:${IMAGE_TAG}" \
    -f elohim/elohim-storage/Dockerfile .

nerdctl -n k8s.io tag "elohim-storage:${IMAGE_TAG}" "elohim-storage:${GIT_COMMIT_HASH}"
