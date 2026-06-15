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

E2E_DOORWAY_ALPHA="${DOORWAY_HOST}" \
E2E_DOORWAY_STAGING="${DOORWAY_HOST}" \
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
