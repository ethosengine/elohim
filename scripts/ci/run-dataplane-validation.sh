#!/bin/bash
# run-dataplane-validation.sh — run @dataplane cucumber suite against deployed alpha,
# then build sprint-report-dataplane.{json,md} with per-concern byConcern rollup.
#
# Called from the edge Jenkinsfile "Dataplane Validation" stage.
# The Jenkinsfile caller wraps in catchError->UNSTABLE so a red concern
# (e.g. blob-replication, epr-projection-fallback — both RED-FIRST as of
# 2026-06-29) surfaces as UNSTABLE without blocking the pipeline or the
# orchestrator's downstream cascade.
#
# CPS extraction note (2026-06-29): bash body lives here so the Jenkinsfile
# stage stays heredoc-free and does not inflate the CPS dispatch method.
# Pattern: sh "bash '${env.WORKSPACE}/scripts/ci/run-dataplane-validation.sh'"
#
# Env:
#   WORKSPACE            Jenkins workspace root (default: git rev-parse --show-toplevel)
#   E2E_DOORWAY_ALPHA    target doorway URL (default: https://doorway-alpha.elohim.host)
#   E2E_STORAGE_URL       internal elohim-storage base URL for scenarios that talk to
#                        storage directly (default: alpha's genesis peer, matthew —
#                        mirrors genesis/scripts/ci/e2e-verify-api.sh's INTERNAL_STORAGE_URL
#                        pattern / genesis/Jenkinsfile resolveInternalStorageUrl()). Without
#                        this every @dataplane scenario that resolves storage falls back to
#                        localhost:8090 (unreachable in the builder container) and dies on
#                        the reachability precondition before any real assertion runs.
#   BUILD_TAG            Jenkins BUILD_TAG string used as run-id in the sprint report

set -euo pipefail

WORKSPACE="${WORKSPACE:-$(git -C "$(dirname "$0")" rev-parse --show-toplevel)}"
A2O_DIR="${WORKSPACE}/genesis/a2o"
REPORTS_DIR="${A2O_DIR}/reports"
CUCUMBER_OUT="${REPORTS_DIR}/cucumber-report-dataplane.json"
SPRINT_JSON="${REPORTS_DIR}/sprint-report-dataplane.json"
SPRINT_MD="${REPORTS_DIR}/sprint-report-dataplane.md"
DOORWAY="${E2E_DOORWAY_ALPHA:-https://doorway-alpha.elohim.host}"
STORAGE_URL="${E2E_STORAGE_URL:-http://elohim-matthew-alpha.elohim-alpha.svc.cluster.local:8090}"
RUN_ID="${BUILD_TAG:-dataplane-$(date +%Y%m%dT%H%M%S)}"

read_scenario_count() {
  awk '
    /^[[:space:]]*"scenarios"[[:space:]]*:[[:space:]]*\{/ {
      in_scenarios = 1
      next
    }
    in_scenarios && /^[[:space:]]*"total"[[:space:]]*:/ {
      count = $0
      sub(/^[^:]*:[[:space:]]*/, "", count)
      sub(/,.*/, "", count)
      gsub(/[[:space:]]/, "", count)
      if (count ~ /^[0-9]+$/) {
        print count
      }
      exit
    }
  ' "$1"
}

echo "=== Dataplane Validation ==="
echo "  Doorway : ${DOORWAY}"
echo "  Storage : ${STORAGE_URL}"
echo "  Run-id  : ${RUN_ID}"
echo "  Reports : ${REPORTS_DIR}"

mkdir -p "${REPORTS_DIR}"

# Fleet-quiesce gate (2026-08-01, edge #1280 RCA): a deploy restarts the whole
# fleet and the reconcile catch-up can run hours; measuring mid-churn records a
# false red the ledger sync can read as a regression. The gate polls until the
# fleet is calm (pull.caughtUp both sides, converged==1 on A sustained across a
# fresh sweep, both doorways serving content 200) or the deadline lapses — on
# deadline we exit 3 ("did not measure", same idiom as the zero-scenario guard
# below) BEFORE any cucumber run, so no sprint-report exists for a churn window.
# Lives here, not in the Jenkinsfile stage, per the CPS extraction note above
# (the stage stays statement-light; inflating it breached the 64KB CPS method
# limit on edge #1282). QUIESCE_SKIP=1 bypasses for local/manual runs.
if [ "${QUIESCE_SKIP:-0}" != "1" ]; then
  QUIESCE_EXIT=0
  bash "${WORKSPACE}/scripts/ci/fleet-quiesce-gate.sh" \
    "${DOORWAY}" \
    "${E2E_DOORWAY_B:-https://elohim.host}" \
    "${QUIESCE_CONTENT_ID:-elohim-host-landing}" \
    "${STORAGE_URL}" \
    "${E2E_STORAGE_URL_B:-http://elohim-adam-alpha.elohim-alpha.svc.cluster.local:8090}" \
    || QUIESCE_EXIT=$?
  if [ "${QUIESCE_EXIT}" -ne 0 ]; then
    echo ""
    echo "=== Dataplane Validation: DID NOT MEASURE ==="
    echo "Fleet-quiesce gate exited ${QUIESCE_EXIT} — the fleet is still churning"
    echo "(or the gate could not read it). Skipping the cucumber suite entirely:"
    echo "no sprint-report is generated, so recorded saga state cannot move on"
    echo "this run. This is a no-measure outcome, not a validation failure."
    exit 3
  fi
fi

# Install a2o workspace dependencies (frozen — never mutates pnpm-lock.yaml in CI).
# Run from WORKSPACE root so the pnpm workspace resolver finds all packages.
cd "${WORKSPACE}"
pnpm install --frozen-lockfile --filter "@elohim/a2o..."

# Run the @dataplane cucumber suite.
# --format json:... ADDS this formatter alongside any config defaults so we get a
# per-run file (cucumber-report-dataplane.json) distinct from the main report.
# `and not @browser-only`: this stage runs plain cucumber-js (no
# E2E_DEVICE_MODE=playwright), so any @browser-only scenario would hit
# requirePwDevice / assert.fail on a missing device — a structural hard-fail on
# a job that cannot render a page. @browser-only scenarios are covered by the
# playwright-mode jobs (test:browser:e2e / delivery-browser). Excluding them
# here yields a clean HELD-skip instead of a false red. See backlog
# a2o-playwright-device-hardfail-topology.
cd "${A2O_DIR}"
CUCUMBER_EXIT=0
E2E_DOORWAY_ALPHA="${DOORWAY}" \
E2E_STORAGE_URL="${STORAGE_URL}" \
  pnpm exec cucumber-js \
    --format "json:${CUCUMBER_OUT}" \
    --tags '@dataplane and not @wip and not @browser-only' \
    || CUCUMBER_EXIT=$?

# Build the per-concern sprint report.
# Runs even on cucumber failure so the byConcern block is always available
# for the agentic-developer loop measure surface (a concern flipping ❌→✅
# is forward progress; ❌ is a named candidate for the next fix).
pnpm exec tsx scripts/build-sprint-report.ts \
  --cucumber  "${CUCUMBER_OUT}" \
  --out-json  "${SPRINT_JSON}" \
  --out-md    "${SPRINT_MD}" \
  --profile   dataplane \
  --run-id    "${RUN_ID}" \
  --doorway   "${DOORWAY}"

# Print a byConcern summary for the Jenkins build log.
if [ -f "${SPRINT_JSON}" ]; then
  echo ""
  echo "=== Dataplane byConcern summary ==="
  python3 -c "
import json, sys
path = sys.argv[1]
try:
    r = json.load(open(path))
except Exception as e:
    print(f'  (could not parse report: {e})')
    sys.exit(0)
bc = r.get('summary', {}).get('byConcern', {})
if not bc:
    print('  (no @concern: tagged scenarios ran)')
for concern, stats in bc.items():
    f = stats.get('failed', 0)
    p = stats.get('passed', 0)
    pend = stats.get('pending', 0)
    glyph = 'OK' if (f == 0 and p > 0) else ('PENDING' if f == 0 else 'FAIL')
    print(f'  [{glyph}] {concern}: passed={p} failed={f} pending={pend}')
" "${SPRINT_JSON}"
fi

# A support-load crash or broken tag filter can leave cucumber with zero
# scenarios. The report builder legitimately renders that empty input, so the
# cucumber exit alone is not a sufficient measurement invariant: Jenkins may
# otherwise downgrade the error and display a false-green SUCCESS. Treat an
# absent/unreadable count and a measured zero as an explicit validation error.
SCENARIO_COUNT=""
if [ -r "${SPRINT_JSON}" ]; then
  SCENARIO_COUNT="$(read_scenario_count "${SPRINT_JSON}" || true)"
fi
VALIDATION_EXIT="${CUCUMBER_EXIT}"

if [ -z "${SCENARIO_COUNT}" ]; then
  echo "ERROR: Dataplane scenario count is missing or unreadable — the report is invalid, this is NOT a pass."
  VALIDATION_EXIT=3
elif [ "${SCENARIO_COUNT}" -eq 0 ]; then
  echo "ERROR: 0 dataplane scenarios ran — support load or tag filter is broken, this is NOT a pass."
  VALIDATION_EXIT=3
fi

# Propagate cucumber failures and the measurement invariant above. The
# Jenkinsfile catchError(stageResult:'UNSTABLE') converts non-zero to UNSTABLE
# (advisory) rather than FAILURE.
exit "${VALIDATION_EXIT}"
