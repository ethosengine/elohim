#!/bin/bash
# detect-stale-resources.sh <namespace> <deploy-version> <component-name>
#
# Post-deploy advisory check: finds resources labeled as part of the elohim
# system that have a stale version label. These may be orphans from a
# previous architecture (e.g., Deployment-to-StatefulSet migration).
#
# ADVISORY ONLY — warns but never deletes. Always exits 0.
#
# Usage:
#   bash orchestrator/scripts/detect-stale-resources.sh elohim-alpha abc1234 edgenode
#
# Requires app.kubernetes.io/part-of and app.kubernetes.io/name labels
# to be present on resources (added by the labeling improvement).
set -euo pipefail

NAMESPACE="${1:?Usage: $0 <namespace> <deploy-version> <component-name>}"
DEPLOY_VERSION="${2:?Usage: $0 <namespace> <deploy-version> <component-name>}"
COMPONENT="${3:?Usage: $0 <namespace> <deploy-version> <component-name>}"

RESOURCE_TYPES="deployments,statefulsets,services,ingresses,configmaps,secrets"

echo "[stale-check] Checking ${COMPONENT} resources in ${NAMESPACE} (expected version: ${DEPLOY_VERSION})"

# List resources matching this component
ALL_RESOURCES=$(kubectl get $RESOURCE_TYPES -n "$NAMESPACE" \
    -l "app.kubernetes.io/part-of=elohim,app.kubernetes.io/name=${COMPONENT}" \
    -o jsonpath='{range .items[*]}{.kind}{"/"}{.metadata.name}{" version="}{.metadata.labels.app\.kubernetes\.io/version}{"\n"}{end}' 2>/dev/null || true)

if [ -z "$ALL_RESOURCES" ]; then
    echo "[stale-check] No labeled ${COMPONENT} resources found (labeling may not be deployed yet)"
    exit 0
fi

STALE_COUNT=0
while IFS= read -r line; do
    [ -z "$line" ] && continue
    RESOURCE=$(echo "$line" | awk '{print $1}')
    VERSION=$(echo "$line" | sed 's/.*version=//')

    # Skip resources with no version label (infra/static resources)
    if [ -z "$VERSION" ]; then
        continue
    fi

    if [ "$VERSION" != "$DEPLOY_VERSION" ]; then
        echo "[stale-check] WARNING: stale ${RESOURCE} (version=${VERSION}, expected=${DEPLOY_VERSION})"
        STALE_COUNT=$((STALE_COUNT + 1))
    fi
done <<< "$ALL_RESOURCES"

if [ $STALE_COUNT -gt 0 ]; then
    echo ""
    echo "[stale-check] WARNING: ${STALE_COUNT} potentially stale ${COMPONENT} resource(s) in ${NAMESPACE}"
    echo "[stale-check] These may be leftovers from a previous architecture."
    echo "[stale-check] Review: kubectl get ${RESOURCE_TYPES} -n ${NAMESPACE} -l app.kubernetes.io/name=${COMPONENT} --show-labels"
else
    echo "[stale-check] No stale ${COMPONENT} resources detected"
fi

# Advisory only — always exit 0
exit 0
