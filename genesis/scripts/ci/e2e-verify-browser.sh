#!/bin/bash
# E2E Verification (BROWSER) — Playwright @browser-only cucumber run.
#
# Externalized verbatim from genesis/Jenkinsfile (E2E Verification stage,
# playwright container) to keep the pipeline's single CPS dispatch method under
# the 64KB MethodTooLargeException limit — see CLAUDE.md "Jenkinsfile Size Limit".
#
# Args:
#   $1 = doorway host base URL (e.g. https://doorway-alpha.elohim.host)
#   $2 = internal storage URL (host:port, no scheme)
#   $3 = ELOHIM_REMOTE_COMPUTE_STATUS ('available' | 'unavailable' | 'unknown')
set -euo pipefail

DOORWAY_HOST="$1"
INTERNAL_STORAGE_URL="$2"
REMOTE_COMPUTE_STATUS="$3"

echo "═══════════════════════════════════════════════════════════"
echo "🧪 E2E VERIFICATION (BROWSER)"
echo "═══════════════════════════════════════════════════════════"
echo "Doorway: ${DOORWAY_HOST}"
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
echo "Household fixture: ${E2E_HOUSEHOLD_FIXTURE_PATH}"
echo "  doorway apex/beta: ${E2E_DOORWAY_APEX:-<undeclared>} / ${E2E_DOORWAY_BETA:-<undeclared>}"
echo "  storage peers:     matthew=${E2E_STORAGE_MATTHEW:-<undeclared>} jessica=${E2E_STORAGE_JESSICA:-<undeclared>} james=${E2E_STORAGE_JAMES:-<undeclared>}"
echo "  doorway pool:      ${E2E_DOORWAY_POOL_STORAGE_URLS:-<undeclared>}"
echo ""

# Ensure the chromium build matches the workspace's
# pnpm-resolved Playwright version. The sidecar image
# bundles browsers, but a Playwright minor bump in the
# workspace expects a different chromium build (see
# 'chromium_headless_shell-XXXX' suffix). Idempotent.
echo "🎭 ensuring matching chromium browser..."
npx playwright install chromium 2>&1 | tail -5

# --- Tag selection ---------------------------------------------------------
# `not @local` mirrors e2e-verify-api.sh. This run targets a DEPLOYED fleet, so
# @local scenarios — the ones that need a PID to SIGSTOP, a log file on this
# disk, or an ingest-then-place loop against the mesh being measured (the
# household-mesh fixture above declares processControl:false for exactly that
# first reason) — cannot pass here. They keep their coverage on the local stack
# (`npx cucumber-js -p local`).
#
# THE EXCLUSION IS ONLY AS HONEST AS THE TAGS IT READS. Adding it here first cut
# 11 of 68 selected scenarios, the whole of
# features/resilience/observable-distribution.feature — because that feature
# carried @local on its FEATURE header. The audit against build #1489 found the
# header tag over-broad: of those 11, one PASSED against
# doorway-alpha.elohim.host, four reached their navigation step with every
# fixture Given green before timing out on page load, and six reported
# `pending`; not one called a process-control step. @local now sits on the two
# scenarios in that feature that actually ingest into the mesh, and the other 11
# are back in this gate. If this exclusion ever drops the count again, audit the
# tag before accepting the number — a feature-level @local silently withdraws
# every scenario under it, including deployed-capable ones.
#
# CUCUMBER_JSON_REPORT / CUCUMBER_HTML_REPORT point cucumber.mjs's base profile
# at this stage's own report instead of the generic reports/cucumber-report.json
# it would otherwise ALSO write (cucumber-js merges config formats with CLI
# ones rather than replacing them, so naming a --format below is not enough).
# That generic copy is what the CucumberReport plugin ingested as a third,
# phantom run — the browser stage's failed steps counted twice in the "Found N
# failed steps" headline.
CUCUMBER_JSON_REPORT=reports/cucumber-report-browser.json \
CUCUMBER_HTML_REPORT=reports/cucumber-report-browser.html \
E2E_DEVICE_MODE=playwright \
E2E_DOORWAY_ALPHA="${DOORWAY_HOST}" \
E2E_STORAGE_URL="http://${INTERNAL_STORAGE_URL}" \
ELOHIM_REMOTE_COMPUTE_STATUS="${REMOTE_COMPUTE_STATUS}" \
    npx cucumber-js --tags '@e2e and @browser-only and not @local and not @wip' \
    --format progress-bar \
    --format json:reports/cucumber-report-browser.json \
    --format html:reports/cucumber-report-browser.html || CUCUMBER_EXIT=$?

# --- Report de-duplication -------------------------------------------------
# Runs pass or fail — cucumber writes its JSON either way and this stage is
# expected to go red — so the cleanup must not sit behind `set -e`. Hence the
# captured exit code above and the explicit exit below.
#
# Second line of defence behind the pipeline's `cucumber-report-*.json` (hyphen)
# publish glob: leave exactly the two snapshots the two stages own, so any
# consumer globbing `cucumber-report*.json` counts each run once.
#
# With the env override above, whatever sits at the generic path is the API
# stage's run, which e2e-verify-api.sh already snapshotted. Three cases:
#   - API snapshot present  -> the generic file is a duplicate. Drop it.
#   - generic == this run   -> the override did not take (older cucumber.mjs).
#                              It is OUR output under the wrong name. Drop it;
#                              never file browser data as the API report.
#   - otherwise             -> the API cp did not happen and the generic file is
#                              the only copy of that run. Promote, don't discard.
if [ -f reports/cucumber-report.json ]; then
    if [ -f reports/cucumber-report-api.json ]; then
        rm -f reports/cucumber-report.json
        echo "🧹 dropped duplicate reports/cucumber-report.json (API run is snapshotted as cucumber-report-api.json)"
    elif cmp -s reports/cucumber-report.json reports/cucumber-report-browser.json; then
        rm -f reports/cucumber-report.json
        echo "🧹 dropped reports/cucumber-report.json (identical to this stage's browser report)"
    else
        mv -f reports/cucumber-report.json reports/cucumber-report-api.json
        echo "🧹 promoted reports/cucumber-report.json to cucumber-report-api.json (API snapshot was missing)"
    fi
fi

if [ "${CUCUMBER_EXIT:-0}" -ne 0 ]; then
    exit "${CUCUMBER_EXIT}"
fi

echo "✅ Browser E2E tests passed"
