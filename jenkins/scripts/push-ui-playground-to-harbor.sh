#!/bin/bash
set -euo pipefail

IMAGE_TAG=$1
GIT_COMMIT_HASH=$2
BRANCH_NAME=$3

nerdctl -n k8s.io tag elohim-ui-playground:${IMAGE_TAG} harbor.ethosengine.com/ethosengine/elohim-ui-playground:${IMAGE_TAG}
nerdctl -n k8s.io tag elohim-ui-playground:${IMAGE_TAG} harbor.ethosengine.com/ethosengine/elohim-ui-playground:${GIT_COMMIT_HASH}

nerdctl -n k8s.io push harbor.ethosengine.com/ethosengine/elohim-ui-playground:${IMAGE_TAG}
nerdctl -n k8s.io push harbor.ethosengine.com/ethosengine/elohim-ui-playground:${GIT_COMMIT_HASH}

if [ "${BRANCH_NAME}" = "main" ]; then
    nerdctl -n k8s.io tag elohim-ui-playground:${IMAGE_TAG} harbor.ethosengine.com/ethosengine/elohim-ui-playground:latest
    nerdctl -n k8s.io push harbor.ethosengine.com/ethosengine/elohim-ui-playground:latest
fi
