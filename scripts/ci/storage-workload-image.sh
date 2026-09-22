#!/usr/bin/env bash
# storage-workload-image.sh — pick the image a human's STORAGE StatefulSet
# renders, holding the live one when the storage build inputs did not change.
#
# The conductor workload already does this ("two pin cadences over one image
# artifact", resolveConductorWorkloadImage in elohim/holochain/Jenkinsfile).
# This is the same rule for the storage workload, keyed on content instead of
# the commit: the commit-derived tag moves on EVERY edge build, so rendering it
# unconditionally restarted all seven storage peers for a doorway-only or
# docs-only commit (2026-09-22 CI wall-clock repair).
#
# DECIDE:  storage-workload-image.sh <sts> <namespace> <built-image> <input-digest>
#   stdout: the image to render (always exactly one line)
#   stderr: the reason, one line, for the build log
#   Rolls to <built-image> when ANY of:
#     · operator asks: `[storage-roll]` in the HEAD commit message or
#       STORAGE_ROLL=1|true in the environment;
#     · the StatefulSet (or its elohim-node container image) does not exist;
#     · <input-digest> is empty ("cannot judge" — storage-input-digest.sh
#       refused), or the live object carries no recorded digest;
#     · the recorded digest differs from <input-digest>;
#     · the live image's tag is no longer in Harbor (retention) — holding a
#       tag a rescheduled pod cannot pull would turn a skipped roll into an
#       ImagePullBackOff.
#   Otherwise prints the LIVE image, so the pod template stays byte-stable.
#
# RECORD:  storage-workload-image.sh --record <sts> <namespace> <input-digest>
#   Writes (or, for an empty digest, removes) the OBJECT annotation
#   elohim.host/storage-inputs after apply. Object metadata, never the pod
#   template — a value that moves must not itself restart the pod.
#
# Env: KUBECTL (default kubectl); HARBOR_USER/HARBOR_PASS (optional robot
# creds for the registry token); HARBOR_MANIFEST_URL_PREFIX and
# HARBOR_TOKEN_URL (tests).
set -uo pipefail

KUBECTL="${KUBECTL:-kubectl}"
ANNOTATION="elohim.host/storage-inputs"
ANNOTATION_JSONPATH='{.metadata.annotations.elohim\.host/storage-inputs}'
HARBOR_PREFIX="${HARBOR_MANIFEST_URL_PREFIX:-https://harbor.ethosengine.com/v2}"

if [ "${1:-}" = "--record" ]; then
    STS="${2:?--record <sts> <namespace> <input-digest>}"
    NS="${3:?--record <sts> <namespace> <input-digest>}"
    DIGEST="${4:-}"
    if [ -n "${DIGEST}" ]; then
        "${KUBECTL}" annotate "statefulset/${STS}" -n "${NS}" "${ANNOTATION}=${DIGEST}" --overwrite
    else
        # Unknown inputs: drop the record so the next deploy cannot hold on it.
        "${KUBECTL}" annotate "statefulset/${STS}" -n "${NS}" "${ANNOTATION}-" 2>/dev/null || true
    fi
    exit $?
fi

STS="${1:?usage: storage-workload-image.sh <sts> <namespace> <built-image> <input-digest>}"
NS="${2:?usage}"
BUILT="${3:?usage}"
DIGEST="${4:-}"

roll() { echo "storage image ${STS}: $* — rolling to ${BUILT}" >&2; echo "${BUILT}"; exit 0; }

COMMIT_MSG="$(git log -1 --pretty=%B 2>/dev/null || true)"
case "${STORAGE_ROLL:-}" in 1|true|TRUE|yes) roll "ROLL requested by operator (STORAGE_ROLL)" ;; esac
case "${COMMIT_MSG}" in *'[storage-roll]'*) roll "ROLL requested by operator ([storage-roll])" ;; esac

LIVE_IMAGE="$("${KUBECTL}" get "statefulset/${STS}" -n "${NS}" \
    -o jsonpath='{.spec.template.spec.containers[?(@.name=="elohim-node")].image}' 2>/dev/null || true)"
[ -n "${LIVE_IMAGE}" ] || roll "no live elohim-node image (first rollout)"
[ -n "${DIGEST}" ] || roll "input digest unavailable (cannot judge)"

LIVE_DIGEST="$("${KUBECTL}" get "statefulset/${STS}" -n "${NS}" -o jsonpath="${ANNOTATION_JSONPATH}" 2>/dev/null || true)"
[ -n "${LIVE_DIGEST}" ] || roll "no recorded ${ANNOTATION} on the live StatefulSet (cannot judge)"
[ "${LIVE_DIGEST}" = "${DIGEST}" ] || roll "storage inputs changed (${LIVE_DIGEST} -> ${DIGEST})"
[ "${LIVE_IMAGE}" != "${BUILT}" ] || roll "live image already is this build's image (nothing to hold)"

# Harbor retention guard: harbor.ethosengine.com/<project>/<repo>:<tag> only.
# 200 → pullable, hold. 404 → the tag is gone (retention), roll. Anything else
# (registry down, auth refused) is INCONCLUSIVE about pullability, not about
# content: the running pod needs no pull, so hold and say so loudly — the same
# exposure the conductor workload's held image already carries.
MANIFEST_ACCEPT='application/vnd.oci.image.index.v1+json, application/vnd.oci.image.manifest.v1+json, application/vnd.docker.distribution.manifest.list.v2+json, application/vnd.docker.distribution.manifest.v2+json'
harbor_manifest_code() {
    local repo="$1" tag="$2" code token
    local creds=()
    code="$(curl -s -o /dev/null -w '%{http_code}' -H "Accept: ${MANIFEST_ACCEPT}" \
        "${HARBOR_PREFIX}/${repo}/manifests/${tag}" 2>/dev/null || echo 000)"
    if [ "${code}" = "401" ]; then
        # Registry v2 token dance: anonymous for public projects, robot creds
        # when the caller exported HARBOR_USER/HARBOR_PASS.
        if [ -n "${HARBOR_USER:-}" ] && [ -n "${HARBOR_PASS:-}" ]; then
            creds=(--user "${HARBOR_USER}:${HARBOR_PASS}")
        fi
        token="$(curl -s "${creds[@]}" "${HARBOR_TOKEN_URL:-https://harbor.ethosengine.com/service/token}?service=harbor-registry&scope=repository:${repo}:pull" 2>/dev/null \
            | sed -n 's/.*"token"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p')"
        if [ -n "${token}" ]; then
            code="$(curl -s -o /dev/null -w '%{http_code}' -H "Accept: ${MANIFEST_ACCEPT}" -H "Authorization: Bearer ${token}" \
                "${HARBOR_PREFIX}/${repo}/manifests/${tag}" 2>/dev/null || echo 000)"
        fi
    fi
    echo "${code}"
}

ref="${LIVE_IMAGE#harbor.ethosengine.com/}"
if [ "${ref}" != "${LIVE_IMAGE}" ] && [ "${ref%%@*}" = "${ref}" ] && [ "${ref%:*}" != "${ref}" ]; then
    code="$(harbor_manifest_code "${ref%:*}" "${ref##*:}")"
    case "${code}" in
        200) ;;
        404) roll "held image ${LIVE_IMAGE} is gone from Harbor (HTTP 404 — retention?)" ;;
        *) echo "storage image ${STS}: WARNING — could not confirm ${LIVE_IMAGE} is still in Harbor (HTTP ${code}); holding anyway (the running pod needs no pull)" >&2 ;;
    esac
fi

echo "⏭️ storage image ${STS}: inputs unchanged (${DIGEST}) — holding ${LIVE_IMAGE}, pod template stays byte-stable (no restart)" >&2
echo "${LIVE_IMAGE}"
