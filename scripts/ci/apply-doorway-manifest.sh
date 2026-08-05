#!/bin/bash
# apply-doorway-manifest.sh — kubectl apply a rendered doorway manifest, tolerating
# ONLY the known jenkins-deployer RBAC drift on `podmonitors` (loudly), and failing
# on anything else.
#
# WHY THIS EXISTS (edge #1306/#1308/#1309, 2026-08-05)
# Wave-2 added an iroh-relay PodMonitor to each doorway manifest. jenkins-deployer
# has no grant on monitoring.coreos.com/podmonitors (backlog: ci-rbac-jenkins-deployer,
# 5th recurrence), so `kubectl apply -f <doorway>-rendered.yaml` now exits non-zero
# even though every core resource (Secret/Deployment/Service/Ingress) applied fine.
# In the Jenkinsfile that non-zero ABORTED deployDoorwayManifest before its
# `rollout restart` + `rollout status --timeout=300s` ever ran — so every edge deploy
# reported "2/2 peers did not reach Ready" while nobody actually waited on, or
# reported, the real rollout. Doorway-A was healthy the whole time; doorway-B was NOT,
# and the poisoned exit is precisely what hid that for hours.
#
# CONTRACT
#   - Core resources must apply. Any error that is not a podmonitors-Forbidden fails.
#   - A podmonitors-Forbidden is reported with a loud RBAC DRIFT banner and exit 0,
#     so the caller proceeds to the rollout wait — the honest readiness signal.
#   - When the operator lands the grant, this script silently becomes a plain apply
#     (no Forbidden lines → nothing to tolerate). It needs no follow-up removal, but
#     the banner disappearing IS the evidence the grant landed.
#
# Usage: apply-doorway-manifest.sh <rendered-manifest.yaml>
set -uo pipefail

MANIFEST="${1:?usage: apply-doorway-manifest.sh <rendered-manifest.yaml>}"

OUT="$(kubectl apply -f "${MANIFEST}" 2>&1)"
RC=$?
echo "${OUT}"

if [ "${RC}" -eq 0 ]; then
    exit 0
fi

# Classify: every server-side error must be a podmonitors-Forbidden to tolerate.
TOTAL_ERRORS="$(printf '%s\n' "${OUT}" | grep -c 'Error from server' || true)"
PODMONITOR_ERRORS="$(printf '%s\n' "${OUT}" | grep -c 'resource "podmonitors"' || true)"

if [ "${TOTAL_ERRORS}" -gt 0 ] && [ "${TOTAL_ERRORS}" -eq "${PODMONITOR_ERRORS}" ]; then
    echo "=================================================================="
    echo "⚠  RBAC DRIFT (tolerated): jenkins-deployer cannot apply PodMonitors"
    echo "   manifest: ${MANIFEST}"
    echo "   ${PODMONITOR_ERRORS} PodMonitor(s) NOT applied — the workload rolled,"
    echo "   but Prometheus scrape config for it is MISSING."
    echo "   Grant needed: monitoring.coreos.com/podmonitors {get,create,patch}"
    echo "   Backlog: genesis/data/timeline/backlog/ci-rbac-jenkins-deployer.md"
    echo "   Proceeding to rollout wait — core resources applied cleanly."
    echo "=================================================================="
    exit 0
fi

echo "kubectl apply failed with ${TOTAL_ERRORS} server error(s), ${PODMONITOR_ERRORS} of them tolerable — NOT tolerating."
exit "${RC}"
