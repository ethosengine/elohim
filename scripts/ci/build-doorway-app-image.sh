#!/usr/bin/env bash
# Build the doorway operator dashboard in the root pnpm workspace, then package
# its derived dist/ tree in the thin nginx image. Invoked by elohim-edge.
set -euo pipefail

: "${DOORWAY_APP_TAG:?DOORWAY_APP_TAG is required}"
: "${GIT_COMMIT_HASH:?GIT_COMMIT_HASH is required}"

buildctl --addr unix:///run/buildkit/buildkitd.sock debug workers > /dev/null

pnpm install --frozen-lockfile --filter doorway-app...
pnpm --filter doorway-app build

labels=(--label "org.opencontainers.image.revision=${GIT_COMMIT_HASH}")
if [[ "${DOORWAY_APP_CLOSURE:-}" =~ ^sha256:[a-f0-9]{64}$ ]]; then
    labels+=(
        --label "elohim.host/closure=${DOORWAY_APP_CLOSURE}"
        --label "elohim.host/closure-material=${DOORWAY_APP_CLOSURE_MATERIAL:-${DOORWAY_APP_CLOSURE}}"
    )
fi

cd doorway/doorway-app
BUILDKIT_HOST=unix:///run/buildkit/buildkitd.sock \
    nerdctl -n k8s.io build \
    "${labels[@]}" \
    -t "doorway-app:${DOORWAY_APP_TAG}" \
    -f Dockerfile .
