#!/usr/bin/env bash
# pod-inputs-fingerprint.sh — one digest over the LIVE state a manifest's pods
# actually consume, so a deploy can compare it before and after `kubectl apply`
# and know whether anything that restarts a pod moved.
#
# WHY (2026-09-22 CI wall-clock repair): the edge deploy judged "did this
# workload change?" by grepping `kubectl apply` for "<sts> unchanged". But
# every human manifest stamps the commit into OBJECT metadata
# (`app.kubernetes.io/version`), so apply reports "configured" on EVERY commit.
# Consequences, measured on edge #1474 (Deploy Alpha 75.0m):
#   · the storage restart ran for every peer, genesis surge guard included;
#   · the sequenced conductor phase counted every conductor as CHANGED and ran
#     its settle gate after each step — 5 of 6 gates burned their full
#     fair-share deadline (~450-750s each) on a CFS-throttled fleet.
# The pod template (and the ConfigMaps/Secrets beside it) is the honest
# signal; object labels are not.
#
# Usage: pod-inputs-fingerprint.sh <rendered-manifest> <namespace> [template|all]
#   template  hash `.spec.template` of every StatefulSet/Deployment/DaemonSet
#             in the manifest (what a rollout is keyed on)
#   all       additionally hash `.data` + `.binaryData` of every ConfigMap and
#             Secret in the manifest (content a restart would re-read)
# Objects that do not exist yet hash as empty, so a first apply always reads
# as a change.
#
# RUNTIME: bash + coreutils + awk + kubectl ONLY (scripts/ci/.epr-meta — the
# deploy container invariant; edge #1183). kubectl's jsonpath prints a map
# value through encoding/json, which sorts keys, so the text is canonical.
#
# Prints sha256:<64 hex>. On ANY failure (kubectl error, RBAC, a manifest with
# no workload/config objects parsed) prints nothing and exits 1 — the caller
# must then treat the workload as CHANGED (roll). Never guess "unchanged".
#
# Env (tests): KUBECTL (default kubectl).
set -uo pipefail

FILE="${1:?usage: pod-inputs-fingerprint.sh <rendered-manifest> <namespace> [template|all]}"
NS="${2:?usage: pod-inputs-fingerprint.sh <rendered-manifest> <namespace> [template|all]}"
MODE="${3:-template}"
KUBECTL="${KUBECTL:-kubectl}"

fail() { echo "pod-inputs-fingerprint: $* — cannot judge" >&2; exit 1; }

case "${MODE}" in template|all) ;; *) fail "unknown mode '${MODE}'" ;; esac
[ -f "${FILE}" ] || fail "no such manifest: ${FILE}"

# "<Kind> <name>" for every top-level document: `kind:` at column 0 and the
# first two-space-indented `name:` inside the document's top-level `metadata:`.
OBJECTS="$(awk '
    function flush() { if (kind != "" && name != "") print kind, name; kind = ""; name = ""; inmeta = 0 }
    /^---/                 { flush(); next }
    /^kind:[[:space:]]/    { kind = $2; gsub(/["\047]/, "", kind); next }
    /^metadata:[[:space:]]*$/ { inmeta = 1; next }
    /^[^[:space:]#]/       { inmeta = 0 }
    inmeta && name == "" && /^  name:[[:space:]]/ { name = $2; gsub(/["\047]/, "", name) }
    END { flush() }
' "${FILE}")"

LINES=""
COUNT=0
while read -r kind name; do
    [ -n "${kind}" ] || continue
    case "${kind}" in
        StatefulSet|Deployment|DaemonSet) path='{.spec.template}' ;;
        ConfigMap|Secret) [ "${MODE}" = "all" ] || continue; path='{.data}{"\t"}{.binaryData}' ;;
        *) continue ;;
    esac
    value="$("${KUBECTL}" get "${kind}/${name}" -n "${NS}" --ignore-not-found -o "jsonpath=${path}" 2>/dev/null)" \
        || fail "kubectl get ${kind}/${name} -n ${NS} failed"
    LINES+="${kind}/${name}"$'\t'"${value}"$'\n'
    COUNT=$((COUNT + 1))
done <<< "${OBJECTS}"

[ "${COUNT}" -gt 0 ] || fail "no workload objects parsed from ${FILE}"

printf 'sha256:%s\n' "$(printf '%s' "${LINES}" | sha256sum | awk '{print $1}')"
