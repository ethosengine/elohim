#!/bin/bash
# check-ingress-conflicts.sh <rendered-manifest> <namespace>
#
# Pre-deploy check: ensures no existing ingress in the namespace routes the
# same (hostname, ingressClassName) pair as an ingress being applied under a
# different name. Prevents the exact scenario where stale ingresses capture
# traffic — and, since apex multi-A (2026-09-12 operator ruling on
# doorway-failover, see doorway/doorway-service/.epr-meta/doorway-failover.habit.md),
# LETS two ingresses legitimately claim the SAME host on DIFFERENT classes:
# per-premise ingress classes (public-ethosengine / public-shem) are how both
# doorway premises can each answer for elohim.host without one ingress
# object racing the other for ownership.
#
# Conflict key: (host, ingressClassName). Same host + same class in the
# namespace is still refused — that is the unresolved-owner hazard this
# check exists to catch. Same host + different class is allowed — that is
# the multi-A ingress-per-premise shape.
#
# A manifest or existing ingress with no ingressClassName set (legacy, or an
# object created before ingressClassName was declared) is treated as class
# "public" — this repo's original default class before the per-premise
# classes existed.
#
# Usage:
#   bash genesis/orchestrator/scripts/check-ingress-conflicts.sh manifest.yaml elohim-staging
#
# Exit codes:
#   0 = no conflicts (or no ingress resources in manifest)
#   1 = hostname+class conflict detected — deploy should be aborted
set -euo pipefail

MANIFEST="${1:?Usage: $0 <rendered-manifest> <namespace>}"
NAMESPACE="${2:?Usage: $0 <rendered-manifest> <namespace>}"

DEFAULT_CLASS="public"

# ---------------------------------------------------------------------------
# Parse the rendered manifest document-by-document (split on a bare `---`
# line) rather than the old "last name: line before kind: Ingress" heuristic
# — that heuristic silently depended on kind: appearing AFTER metadata.name
# in every manifest, which is not how apiVersion/kind/metadata/spec ordering
# actually reads (kind precedes metadata.name in every resource in this
# repo). Per-document parsing reads each field from the document it actually
# belongs to, independent of field order.
#
# Emits one TAB-separated record per Ingress resource found:
#   <name>\t<ingressClassName-or-default>\t<host1>,<host2>,...
# ---------------------------------------------------------------------------
parse_manifest_ingresses() {
    local file="$1"
    local cur_kind="" cur_name="" cur_class="" cur_hosts=""

    flush() {
        if [ "$cur_kind" = "Ingress" ] && [ -n "$cur_name" ]; then
            local class="${cur_class:-$DEFAULT_CLASS}"
            printf '%s\t%s\t%s\n' "$cur_name" "$class" "${cur_hosts%,}"
        fi
        cur_kind=""
        cur_name=""
        cur_class=""
        cur_hosts=""
    }

    while IFS= read -r line || [ -n "$line" ]; do
        if [[ "$line" =~ ^---[[:space:]]*$ ]]; then
            flush
            continue
        fi
        if [ -z "$cur_kind" ] && [[ "$line" =~ ^[[:space:]]*kind:[[:space:]]*\"?([A-Za-z]+)\"? ]]; then
            cur_kind="${BASH_REMATCH[1]}"
            continue
        fi
        # First bare `  name:` field in the document is metadata.name — that
        # block always precedes spec (where ingressClassName/rules live) and
        # this repo's label convention never uses a bare `name:` key (it's
        # always dotted, e.g. app.kubernetes.io/name:, which this pattern
        # does not match because a leading non-whitespace token precedes it).
        if [ -z "$cur_name" ] && [[ "$line" =~ ^[[:space:]]+name:[[:space:]]*\"?([^[:space:]\"]+)\"? ]]; then
            cur_name="${BASH_REMATCH[1]}"
            continue
        fi
        if [ -z "$cur_class" ] && [[ "$line" =~ ^[[:space:]]+ingressClassName:[[:space:]]*\"?([^[:space:]\"]+)\"? ]]; then
            cur_class="${BASH_REMATCH[1]}"
            continue
        fi
        # Matches `- host: foo` (rules[].host), never `- hosts:` (tls[].hosts,
        # plural — a different field, deliberately not collected here).
        if [[ "$line" =~ ^[[:space:]]+-[[:space:]]host:[[:space:]]*\"?([^[:space:]\"]+)\"? ]]; then
            cur_hosts="${cur_hosts}${BASH_REMATCH[1]},"
            continue
        fi
    done < "$file"
    flush
}

MANIFEST_INGRESSES="$(parse_manifest_ingresses "$MANIFEST")"

if [ -z "$MANIFEST_INGRESSES" ]; then
    echo "[ingress-check] No Ingress resources in manifest — skipping"
    exit 0
fi

MANIFEST_NAMES="$(printf '%s\n' "$MANIFEST_INGRESSES" | cut -f1 | xargs)"
echo "[ingress-check] Manifest ingresses: ${MANIFEST_NAMES}"
while IFS=$'\t' read -r name class hosts; do
    [ -z "$name" ] && continue
    echo "[ingress-check]   ${name} (class=${class}): ${hosts}"
done <<< "$MANIFEST_INGRESSES"

# ---------------------------------------------------------------------------
# Existing ingresses in the namespace: name|class|host1,host2,...;name2|...;
# An absent ingressClassName renders as an empty jsonpath segment, defaulted
# below the same way a manifest omission is.
# ---------------------------------------------------------------------------
EXISTING_RAW=$(kubectl get ingress -n "$NAMESPACE" \
    -o jsonpath='{range .items[*]}{.metadata.name}{"|"}{.spec.ingressClassName}{"|"}{range .spec.rules[*]}{.host}{","}{end}{";"}{end}' 2>/dev/null || true)

if [ -z "$EXISTING_RAW" ]; then
    echo "[ingress-check] No existing ingresses in namespace ${NAMESPACE} — PASSED"
    exit 0
fi

is_our_ingress() {
    local candidate="$1"
    local n
    for n in $MANIFEST_NAMES; do
        [ "$candidate" = "$n" ] && return 0
    done
    return 1
}

CONFLICTS=0
while IFS=$'\t' read -r m_name m_class m_hosts; do
    [ -z "$m_name" ] && continue
    IFS=';' read -ra ENTRIES <<< "$EXISTING_RAW"
    for ENTRY in "${ENTRIES[@]}"; do
        [ -z "$ENTRY" ] && continue
        EXISTING_NAME="$(echo "$ENTRY" | cut -d'|' -f1)"
        EXISTING_CLASS="$(echo "$ENTRY" | cut -d'|' -f2)"
        EXISTING_CLASS="${EXISTING_CLASS:-$DEFAULT_CLASS}"
        EXISTING_HOSTS="$(echo "$ENTRY" | cut -d'|' -f3-)"

        # Same name = an update to the ingress we're about to apply, not a
        # conflict with a stranger.
        is_our_ingress "$EXISTING_NAME" && continue

        # Different class entirely = a different ingress controller instance
        # deliberately allowed to answer for the same host (multi-A per
        # premise). Only compare hosts when the class matches.
        [ "$EXISTING_CLASS" != "$m_class" ] && continue

        for HOST in $(echo "$m_hosts" | tr ',' ' '); do
            [ -z "$HOST" ] && continue
            # EXACT-match per host — `elohim.host` is a substring of
            # `alpha.elohim.host` and substring grep would false-positive.
            if echo "$EXISTING_HOSTS" | tr ',' '\n' | grep -qFx "$HOST"; then
                echo ""
                echo "  CONFLICT: hostname '${HOST}' + class '${m_class}' is claimed by"
                echo "            manifest ingress '${m_name}' but already routed by"
                echo "            existing ingress '${EXISTING_NAME}' (class '${EXISTING_CLASS}')"
                echo "            in namespace '${NAMESPACE}'"
                echo ""
                echo "  Fix: kubectl delete ingress ${EXISTING_NAME} -n ${NAMESPACE}"
                echo "       (or give the manifest ingress a different ingressClassName if"
                echo "       both should legitimately serve this host)"
                echo ""
                CONFLICTS=$((CONFLICTS + 1))
            fi
        done
    done
done <<< "$MANIFEST_INGRESSES"

if [ "$CONFLICTS" -gt 0 ]; then
    echo "[ingress-check] ERROR: ${CONFLICTS} hostname+class conflict(s) detected. Aborting deploy."
    exit 1
fi

echo "[ingress-check] PASSED — no hostname+class conflicts"
exit 0
