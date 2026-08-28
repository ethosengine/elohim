#!/usr/bin/env bash
# restart-doorway-epr.sh <namespace> <deployment> <doorwayHost>
#
# Make the doorway's EprRouter pick up freshly seeded project-epr commitments,
# then wait at the PUBLIC boundary until the doorway serves stable 200s with
# the conductor connected.
#
# TWO PATHS, cheapest first:
#
#   1. REFRESH-WAIT (default) — no pod churn. The doorway already re-fetches
#      the whole projection set every DOORWAY_EPR_REFRESH_SECS (default 30) and
#      atomically replaces the routing table: `main.rs` "Periodic EPR-router
#      self-heal refresh (operator-free recovery)" runs
#      `resolve_epr_storage_pool` → `fetch_projections_with_fallback` →
#      `apply_epr_fallback_outcome` → `prewarm_projected_shells`, i.e. the
#      byte-for-byte SAME sequence the boot fetch runs, on a 30s cadence, with
#      last-good preservation on failure. Its own comment says it: "the router
#      self-populates once storage recovers — no kubectl restart needed."
#      So all this stage has to do is wait out two refresh ticks and VERIFY.
#
#   2. POD-DELETE (fallback only) — the original 2026-06 behaviour, kept
#      verbatim below and taken only when path 1 does not converge (or when
#      EPR_FORCE_POD_RESTART=1). It is preserved because it is the stronger
#      hammer, not because it is free: deleting the pod resets the doorway's
#      p2p snapshot cache and its upstream breakers, and genesis then measures
#      that recovering pod minutes later in the E2E stage. That is a
#      self-inflicted measurement-by-restart floor of ~2 fingerprints per run
#      (`p2p.caughtUp is undefined/false`, `status=degraded`) — the same
#      anti-pattern the root CLAUDE.md documents for a bare `[build:edge]`
#      fired "just to measure".
#
# Why the change is safe: the pod-delete's stated premise — "the router only
# refreshes at boot OR via SSE projection.registered events" (genesis/Jenkinsfile
# seedProjectionsStage) — was already stale when it was written. The periodic
# refresh landed in 379668123 (2026-05-30); this script was extracted from the
# Jenkinsfile on 2026-06-10, eleven days later, carrying the pre-refresh
# rationale forward unexamined.
#
# Extracted from genesis/Jenkinsfile seedProjectionsStage()
# (2026-06-10: CPS method-size hard limit) — full rationale lives at the
# call site. ee-jenkins can delete pods but not `get deployments` (RBAC),
# hence the pods API + label/grep hedge.
set -uo pipefail

NAMESPACE="$1"
DEPLOYMENT="$2"
DOORWAY_HOST="$3"

# Two refresh ticks + slack. Must exceed 2 * DOORWAY_EPR_REFRESH_SECS so the
# wait cannot straddle a single tick that started before seeding finished.
EPR_REFRESH_WAIT_SECS="${EPR_REFRESH_WAIT_SECS:-70}"
# Post-wait verification budget (5s cadence).
EPR_VERIFY_POLLS="${EPR_VERIFY_POLLS:-12}"
# Operator escape hatch: skip path 1 entirely.
EPR_FORCE_POD_RESTART="${EPR_FORCE_POD_RESTART:-0}"

# ── Path 1: refresh-wait ───────────────────────────────────────
# Ready ⟺ /health is 200 with the conductor connected AND a projected route
# answers 200 carrying `x-epr-router: dispatched` (server/http.rs — the header
# is set only on an EPR-dispatched response, so its presence IS proof the
# routing table is populated). Two consecutive OK polls, same as path 2.
epr_route_dispatched() {
    local path="$1" hdrs
    hdrs=$(curl -s -o /dev/null -D - --max-time 5 "${DOORWAY_HOST}${path}" 2>/dev/null) || return 1
    printf '%s' "$hdrs" | head -n1 | grep -q ' 200' || return 1
    printf '%s' "$hdrs" | grep -qi '^x-epr-router:[[:space:]]*dispatched' || return 1
    return 0
}

if [ "$EPR_FORCE_POD_RESTART" != "1" ]; then
    echo "═══════════════════════════════════════════════════════════"
    echo "REFRESH EprRouter post-seed — NO pod churn (path 1)"
    echo "═══════════════════════════════════════════════════════════"
    echo "Target:  ${DOORWAY_HOST}"
    echo "Waiting ${EPR_REFRESH_WAIT_SECS}s for >=2 periodic EPR refresh ticks"
    echo "  (doorway main.rs: DOORWAY_EPR_REFRESH_SECS, default 30)"
    # Chunked with a line per chunk: the Jenkins call site bounds this step on
    # ACTIVITY (timeout activity: true), so a silent sleep would make a real
    # controller stall indistinguishable from a healthy wait.
    WAITED=0
    while [ "$WAITED" -lt "$EPR_REFRESH_WAIT_SECS" ]; do
        sleep 10
        WAITED=$((WAITED + 10))
        echo "  … ${WAITED}s / ${EPR_REFRESH_WAIT_SECS}s"
    done

    OK_COUNT=0
    for i in $(seq 1 "$EPR_VERIFY_POLLS"); do
        BODY=$(curl -s --max-time 5 -w '\n%{http_code}' "${DOORWAY_HOST}/health" || printf '\n000')
        CODE=$(printf '%s' "$BODY" | tail -n1)
        if [ "$CODE" = "200" ] && printf '%s' "$BODY" | grep -q '"connected":true' \
            && { epr_route_dispatched "/" || epr_route_dispatched "/lamad"; }; then
            OK_COUNT=$((OK_COUNT + 1))
        else
            OK_COUNT=0
        fi
        if [ "$OK_COUNT" -ge 2 ]; then
            echo "✅ EprRouter populated without a restart — /health 200 + conductor-connected"
            echo "   and a projected route answered with x-epr-router: dispatched"
            echo "   (stable ×2 after ~$((EPR_REFRESH_WAIT_SECS + i * 5))s, zero pod churn)"
            exit 0
        fi
        sleep 5
    done

    echo ""
    echo "⚠️  refresh-wait did NOT converge in $((EPR_REFRESH_WAIT_SECS + EPR_VERIFY_POLLS * 5))s"
    echo "   (no 200 + conductor-connected + x-epr-router:dispatched pair)."
    echo "   Falling back to the pod-delete path — note this resets the doorway's"
    echo "   p2p snapshot cache and upstream breakers, so a later E2E measure of"
    echo "   this doorway is measuring a recovering pod."
    echo ""
fi

# DEPLOYMENT is interpolated into an ERE below — require a literal DNS-1123
# name so regex metacharacters can never widen the match.
if ! [[ "$DEPLOYMENT" =~ ^[a-z0-9]([a-z0-9-]*[a-z0-9])?$ ]]; then
    echo "ERROR: DEPLOYMENT '$DEPLOYMENT' is not a plain DNS-1123 name"
    exit 1
fi

echo "═══════════════════════════════════════════════════════════"
echo "RESTART DOORWAY POD — refresh EprRouter post-seed (path 2, FALLBACK)"
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
