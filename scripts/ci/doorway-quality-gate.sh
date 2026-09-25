#!/usr/bin/env bash
# Quality Gate: Doorway — fmt/clippy/test inside doorway-service's Dockerfile
# `check` target. The BuildKit dep layer does the Rust work, so the CI builder
# needs no toolchain. Called from elohim/holochain/Jenkinsfile runDoorwayQualityGate().
set -euo pipefail

buildctl --addr unix:///run/buildkit/buildkitd.sock debug workers > /dev/null

BUILDKIT_HOST=unix:///run/buildkit/buildkitd.sock \
    nerdctl -n k8s.io build \
    --target check \
    -f doorway/doorway-service/Dockerfile .
