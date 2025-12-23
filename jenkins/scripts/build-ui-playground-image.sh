#!/bin/bash
set -euo pipefail

IMAGE_TAG=$1
GIT_COMMIT_HASH=$2
BRANCH_NAME=$3

# Verify BuildKit
buildctl --addr unix:///run/buildkit/buildkitd.sock debug workers > /dev/null

# Create build context
mkdir -p /tmp/build-context-playground
cp -r elohim-library /tmp/build-context-playground/
cp images/Dockerfile.ui-playground /tmp/build-context-playground/Dockerfile
cp images/nginx-ui-playground.conf /tmp/build-context-playground/

# Build image
cd /tmp/build-context-playground
BUILDKIT_HOST=unix:///run/buildkit/buildkitd.sock \
  nerdctl -n k8s.io build -t elohim-ui-playground:${IMAGE_TAG} -f Dockerfile .

# Additional tags
nerdctl -n k8s.io tag elohim-ui-playground:${IMAGE_TAG} elohim-ui-playground:${GIT_COMMIT_HASH}

if [ "${BRANCH_NAME}" = "main" ]; then
    nerdctl -n k8s.io tag elohim-ui-playground:${IMAGE_TAG} elohim-ui-playground:latest
fi
