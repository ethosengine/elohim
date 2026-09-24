#!/usr/bin/env bash
# conductor-happ-stamp.sh — decide the value of a conductor pod template's
# `elohim.host/happ-digest` annotation, keyed on hApp CONTENT rather than on
# the OCI artifact that happens to carry it.
#
# WHY (backlog ci-edge-depends-on-holochain-rebuilds-dna-without-dna-change,
# 2026-09-03 observation; re-measured 2026-09-22): the annotation used to be
# the floating tag's OCI manifest digest. Every DNA-pipeline run republishes
# `elohim-happ:dev-latest` with a NEW manifest digest even when the packed DNA
# hashes are byte-identical (the DNA Hash Guard passes), so every such rebuild
# moved all seven conductor pod templates and rolled the fleet — each conductor
# then needs 56 min to 6 h before its cells are enabled again. Edge #1474 is
# one: the digest moved (#1473 fc0b67ae… → #1474 130e3c1b…), every conductor
# rolled, and the settle gates burned ~50 min.
#
# THE CONTENT KEY: the DNA pipeline stamps a manifest annotation
# `elohim.host/roll-key` on the hApp artifact (scripts/ci/happ-roll-key.sh):
#   dna-set:sha256:…  the packed DNA hash set + happ.yaml — used when the
#                     same build's coordinator hot-swap reached every peer, so
#                     coordinator bytes are ALREADY on the fleet without a roll;
#   bundle:sha256:…   the .happ bytes as well — used whenever that hot-swap did
#                     not provably complete (so a coordinator change still
#                     reaches the conductors by the old road: a roll).
#
# DECIDE:  conductor-happ-stamp.sh <happ-tag> <conductor-sts> <namespace>
#   stdout: "<stamp> <roll-key|->"   (one line)
#   stderr: the reason, one line, for the build log
#   exit 1 + empty stdout when the manifest digest itself cannot be resolved
#   (the Groovy caller keeps its FAIL-SAFE unique marker → roll).
#   The stamp is the NEW manifest digest (→ roll if it differs from live) when:
#     · operator asks: `[conductor-roll]` in HEAD's message or CONDUCTOR_ROLL=true;
#     · the live StatefulSet has no happ-digest annotation (first rollout);
#     · the artifact carries no roll key (pre-key artifact / other publisher);
#     · the live object has no recorded key, or it differs (content moved).
#   Otherwise the stamp is the LIVE annotation value, verbatim: the pod
#   template does not move and the conductor does not restart.
#
# RECORD:  conductor-happ-stamp.sh --record <conductor-sts> <namespace> <roll-key|->
#   Writes (or for `-`/empty removes) the OBJECT annotation
#   elohim.host/happ-roll-key after apply — object metadata, never the pod
#   template.
#
# Env: HARBOR_USER/HARBOR_PASS (optional; oras login), KUBECTL, ORAS (tests).
set -uo pipefail

KUBECTL="${KUBECTL:-kubectl}"
ORAS="${ORAS:-oras}"
REGISTRY="harbor.ethosengine.com"
REPO="ethosengine/elohim-happ"
KEY_ANNOTATION="elohim.host/happ-roll-key"

if [ "${1:-}" = "--record" ]; then
    STS="${2:?--record <sts> <namespace> <roll-key|->}"
    NS="${3:?--record <sts> <namespace> <roll-key|->}"
    KEY="${4:-}"
    if [ -n "${KEY}" ] && [ "${KEY}" != "-" ]; then
        "${KUBECTL}" annotate "statefulset/${STS}" -n "${NS}" "${KEY_ANNOTATION}=${KEY}" --overwrite
        exit $?
    fi
    "${KUBECTL}" annotate "statefulset/${STS}" -n "${NS}" "${KEY_ANNOTATION}-" >/dev/null 2>&1 || true
    exit 0
fi

TAG="${1:?usage: conductor-happ-stamp.sh <happ-tag> <conductor-sts> <namespace>}"
STS="${2:?usage}"
NS="${3:?usage}"
REF="${REGISTRY}/${REPO}:${TAG}"

# oras: same pinned install as genesis/orchestrator/scripts/resolve-happ-digest.sh.
if [ "${ORAS}" = "oras" ] && ! command -v oras >/dev/null 2>&1; then
    ORAS_TMP="$(mktemp -d)"
    if curl -sSL -o "${ORAS_TMP}/oras.tar.gz" \
        https://github.com/oras-project/oras/releases/download/v1.1.0/oras_1.1.0_linux_amd64.tar.gz \
        && tar -xzf "${ORAS_TMP}/oras.tar.gz" -C "${ORAS_TMP}" oras; then
        chmod +x "${ORAS_TMP}/oras"
        export PATH="${ORAS_TMP}:${PATH}"
    else
        echo "conductor-happ-stamp: failed to install oras CLI" >&2
        exit 1
    fi
fi
if [ -n "${HARBOR_USER:-}" ] && [ -n "${HARBOR_PASS:-}" ]; then
    printf '%s' "${HARBOR_PASS}" | "${ORAS}" login "${REGISTRY}" -u "${HARBOR_USER}" --password-stdin >/dev/null 2>&1 || true
fi

DIGEST="$("${ORAS}" manifest fetch --descriptor "${REF}" 2>/dev/null | grep -oE 'sha256:[a-f0-9]{64}' | head -n1)"
if [ -z "${DIGEST}" ]; then
    echo "conductor-happ-stamp: could not resolve digest for ${REF}" >&2
    exit 1
fi
# Pin the manifest read to the digest just resolved, so a concurrent tag move
# cannot pair one artifact's digest with another's key.
MANIFEST="$("${ORAS}" manifest fetch "${REGISTRY}/${REPO}@${DIGEST}" 2>/dev/null || true)"
KEY="$(printf '%s' "${MANIFEST}" | grep -oE '"elohim\.host/roll-key"[[:space:]]*:[[:space:]]*"(dna-set|bundle):sha256:[a-f0-9]{64}"' \
    | head -n1 | grep -oE '(dna-set|bundle):sha256:[a-f0-9]{64}')"

out() { echo "conductor hApp ${STS}: $1" >&2; echo "$2 ${KEY:--}"; exit 0; }

COMMIT_MSG="$(git log -1 --pretty=%B 2>/dev/null || true)"
case "${CONDUCTOR_ROLL:-}" in true|1) out "ROLL requested by operator (CONDUCTOR_ROLL) — stamping ${DIGEST}" "${DIGEST}" ;; esac
case "${COMMIT_MSG}" in *'[conductor-roll]'*) out "ROLL requested by operator ([conductor-roll]) — stamping ${DIGEST}" "${DIGEST}" ;; esac

LIVE_STAMP="$("${KUBECTL}" get "statefulset/${STS}" -n "${NS}" \
    -o jsonpath='{.spec.template.metadata.annotations.elohim\.host/happ-digest}' 2>/dev/null || true)"
LIVE_KEY="$("${KUBECTL}" get "statefulset/${STS}" -n "${NS}" \
    -o jsonpath='{.metadata.annotations.elohim\.host/happ-roll-key}' 2>/dev/null || true)"

[ -n "${LIVE_STAMP}" ] || out "no live happ-digest (first rollout) — stamping ${DIGEST}" "${DIGEST}"
[ -n "${KEY}" ] || out "artifact carries no elohim.host/roll-key — stamping manifest digest ${DIGEST} (legacy rule: rolls iff the digest moved)" "${DIGEST}"
[ -n "${LIVE_KEY}" ] || out "no recorded ${KEY_ANNOTATION} on the live StatefulSet (cannot judge) — stamping ${DIGEST}; recording ${KEY}" "${DIGEST}"
[ "${LIVE_KEY}" = "${KEY}" ] || out "hApp content moved (${LIVE_KEY} -> ${KEY}) — stamping ${DIGEST} (conductor ROLLS)" "${DIGEST}"
out "⏭️ hApp content unchanged (${KEY}) — holding live stamp ${LIVE_STAMP} though the artifact digest is ${DIGEST}; no conductor restart (coordinators ride the hot-swap)" "${LIVE_STAMP}"
