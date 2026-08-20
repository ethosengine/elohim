#!/bin/bash
# E2E Verification (API) — fast, no-browser @e2e cucumber run.
#
# Externalized verbatim from genesis/Jenkinsfile (E2E Verification stage) to
# keep the pipeline's single CPS dispatch method under the 64KB
# MethodTooLargeException limit — see CLAUDE.md "Jenkinsfile Size Limit".
#
# Args:
#   $1 = doorway host base URL (e.g. https://doorway-alpha.elohim.host)
#   $2 = internal storage URL (host:port, no scheme)
#   $3 = ELOHIM_REMOTE_COMPUTE_STATUS ('available' | 'unavailable' | 'unknown')
#
# Credential (NOT in argv): E2E_ADMIN_BOOTSTRAP_KEY comes via withEnv.
set -euo pipefail

DOORWAY_HOST="$1"
INTERNAL_STORAGE_URL="$2"
REMOTE_COMPUTE_STATUS="$3"

echo "═══════════════════════════════════════════════════════════"
echo "🧪 E2E VERIFICATION (API)"
echo "═══════════════════════════════════════════════════════════"
echo "Doorway: ${DOORWAY_HOST}"
echo "Substrate: ELOHIM_REMOTE_COMPUTE_STATUS=${REMOTE_COMPUTE_STATUS} (remote-only scenarios auto-skip when unavailable)"
echo ""
# --- Live household-mesh fixture -------------------------------------------
# genesis/a2o/src/framework/fixtures/household-mesh.ts is the single authority
# for the multi-doorway / multi-peer legs of the federation and resilience
# features. It reads a manifest (E2E_HOUSEHOLD_FIXTURE_PATH) that conventional
# E2E_* variables then override. Nothing used to write either half, so every
# scenario naming a second doorway or a named storage peer died reporting an
# environment-variable NAME — which says nothing about the fleet.
#
# The POSITIVE legs (apex/beta doorway URLs, per-peer storage URLs, the
# doorway's storage pool) are topology and come from the Jenkinsfile, which
# already owns that knowledge (householdMeshEnv(), derived from
# genesis/orchestrator/data/deployments.json + the doorway manifests). They
# arrive here as inherited environment.
#
# The NEGATIVE legs are properties of running against a DEPLOYED fleet at all,
# so they are declared here:
#   - processControl:false — every peer and doorway is a remote pod. There is
#     no PID on this host to SIGSTOP and no log file on this disk to tail.
#     Scenarios needing that are local-stack-only (@local); they now fail
#     naming the substrate, not a variable.
#   - doorways.gamma absent — the alpha namespace deploys exactly two doorways
#     (manifests/doorway/alpha.yaml -> alpha-elohim-host, alpha-b.yaml ->
#     apex-elohim-host). There is no third.
# An externally supplied E2E_HOUSEHOLD_FIXTURE_PATH always wins.
if [ -z "${E2E_HOUSEHOLD_FIXTURE_PATH:-}" ]; then
    mkdir -p reports
    E2E_HOUSEHOLD_FIXTURE_PATH="$(pwd)/reports/household-mesh.fixture.json"
    cat > "${E2E_HOUSEHOLD_FIXTURE_PATH}" <<'FIXTURE_JSON'
{
  "$comment": "Written by genesis/scripts/ci/e2e-verify-*.sh. Deployed-fleet counterpart of genesis/a2o/household-mesh.fixture.example.json.",
  "processControl": false,
  "processControlReason": "this run targets a deployed fleet - every storage peer and doorway is a Kubernetes pod, so the harness has no PID to signal and no log file to tail",
  "doorways": {
    "gamma": {
      "absentReason": "the alpha namespace deploys exactly two doorways - alpha-elohim-host (doorway-alpha.elohim.host, matthew-backed) and apex-elohim-host (elohim.host, adam-backed). There is no third; a scenario needing one must say which topology it wants"
    }
  }
}
FIXTURE_JSON
    export E2E_HOUSEHOLD_FIXTURE_PATH
fi
# --- peer "staging" is a SECOND PEER, not a second Kubernetes environment ------
# The federation features use "alpha" and "staging" as two independently-
# stewarded peers with asymmetric custody. This used to be exported as
# E2E_DOORWAY_STAGING="${DOORWAY_HOST}" — the same value as E2E_DOORWAY_ALPHA —
# which made every cross-peer claim a statement about ONE host, i.e. a tautology.
# The alpha fleet does deploy a real second doorway: doorway-B
# (manifests/doorway/alpha-b.yaml — elohim.host, adam-backed, its own storage and
# its own MongoDB projection DB), which the Jenkinsfile declares as
# E2E_DOORWAY_BETA. Point staging at it when it is declared.
#
# Falling back to DOORWAY_HOST keeps single-doorway environments runnable, and it
# is NOT a silent collapse: the step definitions compare the two resolved URLs
# (distinctPeers in genesis/a2o/steps/federation-epr.steps.ts) and HOLD the
# cross-peer steps as pending, naming the collapse, rather than asserting against
# one host.
STAGING_DOORWAY="${E2E_DOORWAY_BETA:-${DOORWAY_HOST}}"

echo "Household fixture: ${E2E_HOUSEHOLD_FIXTURE_PATH}"
echo "  doorway apex/beta: ${E2E_DOORWAY_APEX:-<undeclared>} / ${E2E_DOORWAY_BETA:-<undeclared>}"
echo "  storage peers:     matthew=${E2E_STORAGE_MATTHEW:-<undeclared>} jessica=${E2E_STORAGE_JESSICA:-<undeclared>} james=${E2E_STORAGE_JAMES:-<undeclared>}"
echo "  doorway pool:      ${E2E_DOORWAY_POOL_STORAGE_URLS:-<undeclared>}"
if [ "${STAGING_DOORWAY}" = "${DOORWAY_HOST}" ]; then
    echo "  peer staging:      ${STAGING_DOORWAY}  (SAME host as peer alpha — no second doorway declared; cross-peer steps HOLD instead of asserting a tautology)"
else
    echo "  peer staging:      ${STAGING_DOORWAY}  (doorway-B — a real federation peer of doorway-A: own pod, own backing storage, own projection DB)"
fi
echo ""

E2E_DOORWAY_ALPHA="${DOORWAY_HOST}" \
E2E_DOORWAY_STAGING="${STAGING_DOORWAY}" \
E2E_STORAGE_URL="http://${INTERNAL_STORAGE_URL}" \
ELOHIM_REMOTE_COMPUTE_STATUS="${REMOTE_COMPUTE_STATUS}" \
    npx cucumber-js --tags '@e2e and not @browser-only and not @local and not @wip' \
    --format progress-bar \
    --format json:reports/cucumber-report.json \
    --format html:reports/cucumber-report.html || CUCUMBER_EXIT=$?

# Snapshot the API run's JSON before the browser stage clobbers it:
# cucumber.mjs's base config writes reports/cucumber-report.json on
# EVERY run (including the browser one), so without this copy the
# API sprint-report is silently built from BROWSER-stage data
# (diagnosed on genesis #1077: "API" report = 42 @browser-only scenarios).
cp -f reports/cucumber-report.json reports/cucumber-report-api.json || true

if [ "${CUCUMBER_EXIT:-0}" -ne 0 ]; then
    exit "${CUCUMBER_EXIT}"
fi

echo "✅ API E2E tests passed"
