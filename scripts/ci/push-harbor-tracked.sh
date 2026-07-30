#!/usr/bin/env bash
# push-harbor-tracked.sh — tag + push one Harbor image, tracked by the caller via
# `returnStatus: true` (elohim-edgenode + elohim-doorway in the Jenkinsfile's
# stage('Push to Harbor')). Extracted 2026-07-30 to shrink the edge Jenkinsfile's
# CPS pipeline-block bytecode — see CLAUDE.md "Jenkinsfile Size Limit"
# (MethodTooLargeException, 64KB JVM method-size limit).
#
# Behavior is preserved EXACTLY from the original inline heredoc: on a found
# image it tags+pushes both the component tag and the commit-hash tag, then
# `exit 0`; on a not-found image it echoes and `exit 1`. There is no `set -e`
# (matching the original, which had none) — a mid-push nerdctl failure does
# NOT stop the script; the unconditional `exit 0` at the end of the found-branch
# still fires regardless, exactly as it did inline. The Jenkinsfile calls this
# with `returnStatus: true` and uses the boolean result to set env.IMAGES_PUSHED
# — it never lets a non-zero here fail the build outright.
#
# Args (positional, non-secret — Groovy-interpolated at the call site):
#   $1  LOCAL_IMAGE   local nerdctl image name, e.g. elohim-edgenode
#   $2  REMOTE_IMAGE  Harbor repo path under ethosengine/, e.g. elohim-edgenode
#   $3  TAG           the component's resolved tag value, e.g. $EDGENODE_TAG
#   $4  LABEL         human label for log messages, e.g. "Edge Node"
#
# Env (already exported by the Jenkinsfile's withBuildVars/withEnv — read
# directly, never re-passed as argv):
#   IMAGE_TAG          the just-built local tag suffix, used for the `images | grep` check
#   GIT_COMMIT_HASH    provenance tag pushed alongside the component tag
set -uo pipefail

LOCAL_IMAGE="$1"
REMOTE_IMAGE="$2"
TAG="$3"
LABEL="$4"

if nerdctl -n k8s.io images | grep -q "${LOCAL_IMAGE}.*${IMAGE_TAG}"; then
    echo "Pushing ${LABEL} image: ${TAG}"
    nerdctl -n k8s.io tag "${LOCAL_IMAGE}:${IMAGE_TAG}" "harbor.ethosengine.com/ethosengine/${REMOTE_IMAGE}:${TAG}"
    nerdctl -n k8s.io tag "${LOCAL_IMAGE}:${IMAGE_TAG}" "harbor.ethosengine.com/ethosengine/${REMOTE_IMAGE}:${GIT_COMMIT_HASH}"

    nerdctl -n k8s.io push "harbor.ethosengine.com/ethosengine/${REMOTE_IMAGE}:${TAG}"
    nerdctl -n k8s.io push "harbor.ethosengine.com/ethosengine/${REMOTE_IMAGE}:${GIT_COMMIT_HASH}"
    exit 0
else
    echo "${LABEL} image not found, skipping push"
    exit 1
fi
