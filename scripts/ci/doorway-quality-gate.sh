#!/usr/bin/env bash
# Quality Gate: Doorway — fmt/clippy/test inside doorway-service's Dockerfile
# `check` target. The BuildKit dep layer does the Rust work, so the CI builder
# needs no toolchain. Solve without exporting/loading an image: the result is
# only the gate's exit status. Called from Jenkins runDoorwayQualityGate().
set -euo pipefail

buildctl --addr unix:///run/buildkit/buildkitd.sock debug workers > /dev/null

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${WORKSPACE_ROOT}"

buildctl --addr unix:///run/buildkit/buildkitd.sock build \
    --frontend dockerfile.v0 \
    --local context=. \
    --local dockerfile=. \
    --opt filename=doorway/doorway-service/Dockerfile \
    --opt target=check \
    --progress plain
