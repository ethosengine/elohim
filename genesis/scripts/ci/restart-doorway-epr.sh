#!/usr/bin/env bash
# restart-doorway-epr.sh <namespace> <deployment> <doorwayHost>
#
# Force a doorway pod restart so the EprRouter's boot-time fetch picks up
# freshly seeded project-epr commitments, then wait at the PUBLIC boundary
# until the doorway serves stable 200s with the conductor connected.
# Extracted verbatim from genesis/Jenkinsfile seedProjectionsStage()
# (2026-06-10: CPS method-size hard limit) — full rationale lives at the
# call site. ee-jenkins can delete pods but not `get deployments` (RBAC),
# hence the pods API + label/grep hedge.
set -uo pipefail

NAMESPACE="$1"
DEPLOYMENT="$2"
DOORWAY_HOST="$3"

# DEPLOYMENT is interpolated into an ERE below — require a literal DNS-1123
# name so regex metacharacters can never widen the match.
if ! [[ "$DEPLOYMENT" =~ ^[a-z0-9]([a-z0-9-]*[a-z0-9])?$ ]]; then
    echo "ERROR: DEPLOYMENT '$DEPLOYMENT' is not a plain DNS-1123 name"
    exit 1
fi

echo "═══════════════════════════════════════════════════════════"
echo "RESTART DOORWAY POD — refresh EprRouter post-seed"
echo "═══════════════════════════════════════════════════════════"
echo "Namespace: ${NAMESPACE}"
echo "Target:    pods matching '${DEPLOYMENT}'"
echo ""

# Try canonical k8s recommended label first, then a common alternative,
# then a grep-by-name fallback. Each branch is gated on the prior branch
# failing (-z stdout check).
PODS=$(kubectl get pods -n "${NAMESPACE}" -l "app.kubernetes.io/name=${DEPLOYMENT}" -o name 2>/dev/null || true)
if [ -z "$PODS" ]; then
    PODS=$(kubectl get pods -n "${NAMESPACE}" -l "app=${DEPLOYMENT}" -o name 2>/dev/null || true)
fi
if [ -z "$PODS" ]; then
    # ANCHORED name match: a Deployment's pods are exactly
    # `pod/<deployment>-<replicaset-hash>-<pod-hash>` — two dash-segments after
    # the deployment name. An unanchored substring grep here deleted the
    # alpha-b federation peer's pod as collateral when targeting
    # `elohim-doorway-alpha` (`elohim-doorway-alpha-b-…` is a substring
    # superset; genesis #1229, 2026-07-01 — and the health-wait below only
    # covers ${DOORWAY_HOST}, so the collateral peer restarted UNVERIFIED).
    # Note: the label branches above rarely hit for doorway deployments (labels
    # carry `app.kubernetes.io/name: doorway` / `app: elohim-doorway`, not the
    # deployment name), so THIS branch is the one that usually selects.
    PODS=$(kubectl get pods -n "${NAMESPACE}" -o name 2>/dev/null | grep -E "^pod/${DEPLOYMENT}(-[a-z0-9]+){2}\$" || true)
fi

if [ -z "$PODS" ]; then
    echo "WARNING: no doorway pods matched any selector in ${NAMESPACE}"
    echo "  Tried: -l app.kubernetes.io/name=${DEPLOYMENT}"
    echo "         -l app=${DEPLOYMENT}"
    echo "         pod name grep '${DEPLOYMENT}'"
    kubectl get pods -n "${NAMESPACE}" 2>&1 | head -20 || true
    exit 1
fi

echo "Deleting pods:"
echo "$PODS" | sed 's/^/  /'
echo ""
echo "$PODS" | xargs -r kubectl delete -n "${NAMESPACE}" --wait=false
echo ""
echo "✓ doorway pods deleted; Deployment controller will recreate"
echo "  New pod's boot will fetch project-epr commitments from storage"
echo "  → EprRouter populated → / and /lamad serve 200"
echo ""

# ── Rollout-wait at the PUBLIC boundary ────────────────────────
# The e2e bench tests THROUGH the public ingress, so readiness is defined
# there — not at pod or internal-DNS level. Without this wait, E2E starts
# while the Deployment controller is still recreating pods and the ENTIRE
# API run 503s at the ingress (genesis #1078). Two consecutive 200s
# required — a single 200 can be a draining old pod.
#
# A bare 200 is NOT enough: a freshly-recreated doorway pod binds its HTTP
# listener and answers 200 on /health BEFORE its admin-WS handshake with
# the conductor pool completes (the james/conductor-2 race). A 200 only
# counts toward the stable streak when the body ALSO reports the conductor
# connected. Degrade gracefully on older images whose /health body has no
# conductor object at all (3 consecutive field-absent 200s → HTTP-only).
echo "⏳ waiting for doorway rollout to serve 200 (conductor-connected) at ${DOORWAY_HOST}/health ..."
OK_COUNT=0
CONDUCTOR_FIELD_MISSING=0
DEGRADE_TO_HTTP_ONLY=0
for i in $(seq 1 72); do
    BODY=$(curl -s --max-time 5 -w '\n%{http_code}' "${DOORWAY_HOST}/health" || printf '\n000')
    CODE=$(printf '%s' "$BODY" | tail -n1)
    if [ "$CODE" = "200" ]; then
        # A 200 with the conductor reported connected is fully ready.
        if printf '%s' "$BODY" | grep -q '"connected":true'; then
            OK_COUNT=$((OK_COUNT+1))
            CONDUCTOR_FIELD_MISSING=0
        elif printf '%s' "$BODY" | grep -q '"conductor"'; then
            # Conductor object present but not yet connected — the listener
            # is up but the admin-WS handshake is still pending. Reset the
            # streak; this is the race we guard.
            echo "  ↻ doorway 200 but conductor not yet connected — waiting ($((i*5))s)"
            OK_COUNT=0
            CONDUCTOR_FIELD_MISSING=0
        else
            # No conductor object at all in the body. Tolerate an older
            # image: count consecutive field-absent 200s and degrade after 3
            # so we never deadlock on a body shape we can't read.
            CONDUCTOR_FIELD_MISSING=$((CONDUCTOR_FIELD_MISSING+1))
            if [ "$CONDUCTOR_FIELD_MISSING" -ge 3 ] && [ "$DEGRADE_TO_HTTP_ONLY" = "0" ]; then
                echo "  ⚠️  conductor readiness field absent from /health body — proceeding on HTTP-200 only (degrade)"
                DEGRADE_TO_HTTP_ONLY=1
            fi
            if [ "$DEGRADE_TO_HTTP_ONLY" = "1" ]; then
                OK_COUNT=$((OK_COUNT+1))
            else
                OK_COUNT=0
            fi
        fi
        if [ "$OK_COUNT" -ge 2 ]; then
            if [ "$DEGRADE_TO_HTTP_ONLY" = "1" ]; then
                echo "✅ doorway serving 200 at public /health (stable ×2 after ~$((i*5))s, conductor field absent — HTTP-only degrade)"
            else
                echo "✅ doorway serving 200 + conductor-connected at public /health (stable ×2 after ~$((i*5))s)"
            fi
            break
        fi
    else
        OK_COUNT=0
    fi
    sleep 5
done
if [ "$OK_COUNT" -lt 2 ]; then
    echo "❌ doorway did not reach a stable 200 (conductor-connected) at the public boundary within 6min."
    echo "   Failing THIS stage loudly (one true cause) instead of letting the"
    echo "   e2e bench drown in dozens of misleading ingress-503 failures."
    exit 1
fi
