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
# set -u only — NO pipefail. The image-exists guard is `nerdctl images | grep -q`,
# and grep -q exits at first match, SIGPIPE-killing a still-writing nerdctl (141).
# Under pipefail that read as pipeline failure → "image not found, skipping push"
# for EVERY tracked push since the 2026-07-30 extraction (edge #1266-#1273:
# artifacts reached Harbor only via the pipefail-free dev-latest alias script,
# SHA tags never pushed). The inline original had no pipefail; the siblings
# (push-harbor-untracked.sh, push-harbor-dev-latest-aliases.sh) don't either.
set -u

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

    # Resolve the pushed manifest digest so deploys can pin by digest — Harbor's
    # tag-retention policy strips this repo's SHA tags after push (backlog:
    # harbor-retention-strips-doorway-sha-tags), and a digest reference cannot be
    # untagged. Local containerd inspect first (race-free), Harbor's
    # Docker-Content-Digest header as fallback. Fail-open: no digest file just
    # means the deploy renders the plain tag (pre-digest behavior).
    DIGEST="$(nerdctl -n k8s.io image inspect "harbor.ethosengine.com/ethosengine/${REMOTE_IMAGE}:${TAG}" --format '{{range .RepoDigests}}{{println .}}{{end}}' 2>/dev/null | grep "ethosengine/${REMOTE_IMAGE}@" | grep -oE 'sha256:[a-f0-9]{64}' | head -n1)"
    if [ -z "${DIGEST}" ]; then
        DIGEST="$(curl -sfI -H 'Accept: application/vnd.oci.image.manifest.v1+json, application/vnd.docker.distribution.manifest.v2+json, application/vnd.docker.distribution.manifest.list.v2+json, application/vnd.oci.image.index.v1+json' "https://harbor.ethosengine.com/v2/ethosengine/${REMOTE_IMAGE}/manifests/${TAG}" 2>/dev/null | tr -d '\r' | awk 'tolower($1)=="docker-content-digest:" {print $2}' | grep -oE 'sha256:[a-f0-9]{64}' | head -n1)"
    fi
    if [ -n "${DIGEST}" ]; then
        printf '%s\n' "${DIGEST}" > "${WORKSPACE:-.}/${REMOTE_IMAGE}.digest"
        echo "${LABEL} pushed digest: ${DIGEST}"
    else
        echo "${LABEL} digest unresolved — deploy will render the plain tag"
    fi
    exit 0
else
    echo "${LABEL} image not found, skipping push"
    exit 1
fi
