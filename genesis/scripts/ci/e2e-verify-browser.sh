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

# Ensure the chromium build matches the workspace's
# pnpm-resolved Playwright version. The sidecar image
# bundles browsers, but a Playwright minor bump in the
# workspace expects a different chromium build (see
# 'chromium_headless_shell-XXXX' suffix). Idempotent.
echo "🎭 ensuring matching chromium browser..."
npx playwright install chromium 2>&1 | tail -5

E2E_DEVICE_MODE=playwright \
E2E_DOORWAY_ALPHA="${DOORWAY_HOST}" \
E2E_STORAGE_URL="http://${INTERNAL_STORAGE_URL}" \
ELOHIM_REMOTE_COMPUTE_STATUS="${REMOTE_COMPUTE_STATUS}" \
    npx cucumber-js --tags '@e2e and @browser-only and not @wip' \
    --format progress-bar \
    --format json:reports/cucumber-report-browser.json \
    --format html:reports/cucumber-report-browser.html

echo "✅ Browser E2E tests passed"
