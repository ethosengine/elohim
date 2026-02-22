#!/bin/bash
# check-ingress-conflicts.sh <rendered-manifest> <namespace>
#
# Pre-deploy check: ensures no existing ingress in the namespace routes
# the same hostname as an ingress being applied under a different name.
# Prevents the exact scenario where stale ingresses capture traffic.
#
# Usage:
#   bash orchestrator/scripts/check-ingress-conflicts.sh manifest.yaml elohim-staging
#
# Exit codes:
#   0 = no conflicts (or no ingress resources in manifest)
#   1 = hostname conflict detected — deploy should be aborted
set -euo pipefail

MANIFEST="${1:?Usage: $0 <rendered-manifest> <namespace>}"
NAMESPACE="${2:?Usage: $0 <rendered-manifest> <namespace>}"

# Extract hostnames from Ingress rules in the manifest.
# Matches lines like "    - host: doorway-staging.elohim.host"
MANIFEST_HOSTS=$(grep -E '^\s+- host:\s' "$MANIFEST" 2>/dev/null | awk '{print $NF}' | sort -u || true)

if [ -z "$MANIFEST_HOSTS" ]; then
    echo "[ingress-check] No Ingress hostnames in manifest — skipping"
    exit 0
fi

# Extract Ingress resource names from the manifest.
# Looks for "name:" lines that precede "kind: Ingress" within a few lines.
# We use a simple state machine approach.
MANIFEST_INGRESS_NAMES=""
LAST_NAME=""
while IFS= read -r line; do
    # Track the most recent name: field
    if echo "$line" | grep -qE '^\s+name:\s'; then
        LAST_NAME=$(echo "$line" | awk '{print $NF}' | tr -d '"')
    fi
    # When we hit "kind: Ingress", the preceding name: is the ingress name
    if echo "$line" | grep -qE '^\s*kind:\s*Ingress'; then
        MANIFEST_INGRESS_NAMES="${MANIFEST_INGRESS_NAMES} ${LAST_NAME}"
    fi
done < "$MANIFEST"

MANIFEST_INGRESS_NAMES=$(echo "$MANIFEST_INGRESS_NAMES" | xargs)

if [ -z "$MANIFEST_INGRESS_NAMES" ]; then
    echo "[ingress-check] No Ingress resources detected in manifest — skipping"
    exit 0
fi

echo "[ingress-check] Manifest ingresses: ${MANIFEST_INGRESS_NAMES}"
echo "[ingress-check] Manifest hostnames: $(echo $MANIFEST_HOSTS | tr '\n' ' ')"

# Query existing ingresses in the namespace.
# Format: name=host1,host2,...;name2=host3,...;
EXISTING_RAW=$(kubectl get ingress -n "$NAMESPACE" \
    -o jsonpath='{range .items[*]}{.metadata.name}{"="}{range .spec.rules[*]}{.host}{","}{end}{";"}{end}' 2>/dev/null || true)

if [ -z "$EXISTING_RAW" ]; then
    echo "[ingress-check] No existing ingresses in namespace ${NAMESPACE} — PASSED"
    exit 0
fi

CONFLICTS=0
for HOST in $MANIFEST_HOSTS; do
    # Parse each existing ingress entry
    IFS=';' read -ra ENTRIES <<< "$EXISTING_RAW"
    for ENTRY in "${ENTRIES[@]}"; do
        [ -z "$ENTRY" ] && continue
        EXISTING_NAME="${ENTRY%%=*}"
        EXISTING_HOSTS="${ENTRY#*=}"

        # Skip if this is an ingress we're about to apply (same name = update, not conflict)
        IS_OUR_INGRESS=false
        for MN in $MANIFEST_INGRESS_NAMES; do
            if [ "$EXISTING_NAME" = "$MN" ]; then
                IS_OUR_INGRESS=true
                break
            fi
        done
        $IS_OUR_INGRESS && continue

        # Check if the existing ingress routes our hostname
        if echo "$EXISTING_HOSTS" | grep -qF "$HOST"; then
            echo ""
            echo "  CONFLICT: hostname '${HOST}' is claimed by manifest ingress"
            echo "            but already routed by existing ingress '${EXISTING_NAME}'"
            echo "            in namespace '${NAMESPACE}'"
            echo ""
            echo "  Fix: kubectl delete ingress ${EXISTING_NAME} -n ${NAMESPACE}"
            echo ""
            CONFLICTS=$((CONFLICTS + 1))
        fi
    done
done

if [ $CONFLICTS -gt 0 ]; then
    echo "[ingress-check] ERROR: ${CONFLICTS} hostname conflict(s) detected. Aborting deploy."
    exit 1
fi

echo "[ingress-check] PASSED — no hostname conflicts"
exit 0
