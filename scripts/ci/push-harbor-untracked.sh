#!/usr/bin/env bash
# push-harbor-untracked.sh — tag + push one Harbor image, NOT tracked via
# returnStatus (elohim-storage, elohim-happ-installer, doorway-app, and
# elohim-agent-sdk in the Jenkinsfile's stage('Push to Harbor')). Extracted
# 2026-07-30 to shrink the edge Jenkinsfile's CPS pipeline-block bytecode —
# see CLAUDE.md "Jenkinsfile Size Limit" (MethodTooLargeException, 64KB JVM
# method-size limit).
#
# Behavior is preserved EXACTLY from the original inline heredoc: there is NO
# explicit exit anywhere and NO `set -e` (matching the original, which had
# neither). The script's own exit status is therefore whatever the LAST
# command run leaves it — the second `nerdctl push` in the found-branch, or
# the `echo` in the not-found branch (always 0). The Jenkinsfile calls this as
# a plain `sh` step (no returnStatus): if the image was found and the final
# push fails, this step — and the stage/build — fails, exactly as it did
# inline; if the image wasn't found, the step always succeeds (echo-only).
# Do NOT add set -e/set -u/early exits here — that would tighten error
# handling beyond what the original tolerated.
#
# Args (positional, non-secret — Groovy-interpolated at the call site):
#   $1  LOCAL_IMAGE   local nerdctl image name, e.g. elohim-storage
#   $2  REMOTE_IMAGE  Harbor repo path under ethosengine/, e.g. elohim-storage
#   $3  TAG           the component's resolved tag value, e.g. $STORAGE_TAG
#   $4  LABEL         human label for log messages, e.g. "elohim-storage"
#
# Env (already exported by the Jenkinsfile's withBuildVars/withEnv — read
# directly, never re-passed as argv):
#   IMAGE_TAG          the just-built local tag suffix, used for the `images | grep` check
#   GIT_COMMIT_HASH    provenance tag pushed alongside the component tag

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
else
    echo "${LABEL} image not found, skipping push"
fi
