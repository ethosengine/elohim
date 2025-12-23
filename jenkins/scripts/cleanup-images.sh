#!/bin/bash
set -euo pipefail

IMAGE_TAG=$1
GIT_COMMIT_HASH=$2
BRANCH_NAME=$3

echo 'Cleaning up Docker images...'
nerdctl -n k8s.io rmi elohim-app:${IMAGE_TAG} || true
nerdctl -n k8s.io rmi elohim-app:${GIT_COMMIT_HASH} || true
nerdctl -n k8s.io rmi harbor.ethosengine.com/ethosengine/elohim-site:${IMAGE_TAG} || true
nerdctl -n k8s.io rmi harbor.ethosengine.com/ethosengine/elohim-site:${GIT_COMMIT_HASH} || true
nerdctl -n k8s.io rmi elohim-ui-playground:${IMAGE_TAG} || true
nerdctl -n k8s.io rmi elohim-ui-playground:${GIT_COMMIT_HASH} || true
nerdctl -n k8s.io rmi harbor.ethosengine.com/ethosengine/elohim-ui-playground:${IMAGE_TAG} || true
nerdctl -n k8s.io rmi harbor.ethosengine.com/ethosengine/elohim-ui-playground:${GIT_COMMIT_HASH} || true

if [ "${BRANCH_NAME}" = "main" ]; then
    nerdctl -n k8s.io rmi elohim-app:latest || true
    nerdctl -n k8s.io rmi harbor.ethosengine.com/ethosengine/elohim-site:latest || true
    nerdctl -n k8s.io rmi elohim-ui-playground:latest || true
    nerdctl -n k8s.io rmi harbor.ethosengine.com/ethosengine/elohim-ui-playground:latest || true
fi

nerdctl -n k8s.io system prune -af --volumes || true
