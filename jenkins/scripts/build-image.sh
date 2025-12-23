#!/bin/bash
set -euo pipefail

IMAGE_TAG=$1
GIT_COMMIT_HASH=$2
BRANCH_NAME=$3

# Verify BuildKit
buildctl --addr unix:///run/buildkit/buildkitd.sock debug workers > /dev/null

# Create build context
mkdir -p /tmp/build-context
cp -r elohim-app /tmp/build-context/
cp images/Dockerfile /tmp/build-context/
cp images/nginx.conf /tmp/build-context/

# Build image
cd /tmp/build-context
BUILDKIT_HOST=unix:///run/buildkit/buildkitd.sock \
  nerdctl -n k8s.io build -t elohim-app:${IMAGE_TAG} -f Dockerfile .

# Additional tags
nerdctl -n k8s.io tag elohim-app:${IMAGE_TAG} elohim-app:${GIT_COMMIT_HASH}

if [ "${BRANCH_NAME}" = "main" ]; then
    nerdctl -n k8s.io tag elohim-app:${IMAGE_TAG} elohim-app:latest
fi
