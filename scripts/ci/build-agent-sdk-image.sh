#!/usr/bin/env bash
# Build the standalone Elohim Agent SDK sidecar image. Its Docker context reads
# only elohim-agent-sdk/{Dockerfile,package.json,tsconfig.json,src/}.
set -euo pipefail

: "${AGENT_SDK_TAG:?AGENT_SDK_TAG is required}"
: "${GIT_COMMIT_HASH:?GIT_COMMIT_HASH is required}"

buildctl --addr unix:///run/buildkit/buildkitd.sock debug workers > /dev/null

labels=(--label "org.opencontainers.image.revision=${GIT_COMMIT_HASH}")
if [[ "${AGENT_SDK_CLOSURE:-}" =~ ^sha256:[a-f0-9]{64}$ ]]; then
    labels+=(
        --label "elohim.host/closure=${AGENT_SDK_CLOSURE}"
        --label "elohim.host/closure-material=${AGENT_SDK_CLOSURE_MATERIAL:-${AGENT_SDK_CLOSURE}}"
    )
fi

cd elohim/elohim-agent/elohim-agent-sdk
BUILDKIT_HOST=unix:///run/buildkit/buildkitd.sock \
    nerdctl -n k8s.io build \
    "${labels[@]}" \
    -t "elohim-agent-sdk:${AGENT_SDK_TAG}" \
    -f Dockerfile .
