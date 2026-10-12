#!/bin/bash
# run-dataplane-validation.sh — run @dataplane cucumber suite against deployed alpha,
# then build sprint-report-dataplane.{json,md} with per-concern byConcern rollup.
#
# Exit semantics: cucumber exits 1 for failed, ambiguous and undefined steps alike
# (edge #1591-#1607: "149 scenarios (14 undefined, 126 skipped, 9 passed)").
# Born-red habit features deliberately keep undefined steps visible. A cucumber
# exit 1 is downgraded to 0 (PENDING-ONLY banner) only when the RAW cucumber JSON
# shows no failed/ambiguous/unknown step status and every undefined/pending
# scenario carries a @concern: listed in dataplane-born-red-concerns.txt. Anything
# else keeps CUCUMBER_EXIT. The zero-scenario / unreadable-count guards force 3
# and take precedence; stale reports are deleted before the run.
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
CUCUMBER_LOG="${REPORTS_DIR}/cucumber-console-dataplane.log"
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
# fleet is calm (pull.caughtUp on A, QUIESCED on A — blocked_by
# divergent_actionable within tolerance (2 on this path — see the export
# below) and unmeasured==0, per the 2026-08-07/08 decisions in
# the content-gap-limit-cycle backlog doc; converged==1 is telemetry, not the
# gate — sustained across a fresh sweep, both doorways serving content 200)
# or the deadline lapses — on
# deadline we exit 3 ("did not measure", same idiom as the zero-scenario guard
# below) BEFORE any cucumber run, so no sprint-report exists for a churn window.
# Lives here, not in the Jenkinsfile stage, per the CPS extraction note above
# (the stage stays statement-light; inflating it breached the 64KB CPS method
# limit on edge #1282). QUIESCE_SKIP=1 bypasses for local/manual runs.
if [ "${QUIESCE_SKIP:-0}" != "1" ]; then
  QUIESCE_EXIT=0
  # Tolerance 2 on this path (2026-08-08): steady-state in-flight retry rows
  # (undeclared-divergent, mid-budget — 0-2 at any instant, re-minted by page
  # rotation) defeated strict-zero sustain in 2/2 live 45-min windows. See the
  # QUIESCE_ACTIONABLE_TOLERANCE header note in fleet-quiesce-gate.sh.
  export QUIESCE_ACTIONABLE_TOLERANCE="${QUIESCE_ACTIONABLE_TOLERANCE:-2}"
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

# a2o support code imports @elohim/storage-client, whose entry point is a built
# dist/ that no stage of a validate-only run produces. Unbuilt, cucumber dies at
# support load and the zero-scenario guard below reads it as "did not measure"
# (edge #1464: MODULE_NOT_FOUND). Same build install-substrate-runner.sh performs.
pnpm --filter @elohim/storage-client build

# The served-shell stations (served-shell-boots.feature "opens the page … in a browser") launch
# Playwright's chromium even in plain cucumber mode; without the browser they red on
# `browserType.launch: Executable doesn't exist` on every run (edge #1450, #1452). Same install
# verify-served-shell.sh already performs in this job; a failed install is reported, not fatal,
# so the non-browser stations still measure.
pnpm --dir "${A2O_DIR}" exec playwright install --with-deps chromium \
  || echo "WARNING: playwright chromium install failed — browser stations will report Executable doesn't exist"

# Remove last run's evidence so a crash before the formatter initialises cannot
# leave a stale report for the guards below (or the pending-only rule) to re-read.
rm -f "${CUCUMBER_OUT}" "${SPRINT_JSON}" "${SPRINT_MD}" "${CUCUMBER_LOG}"

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
# B-side env reaches the suite too: the quiesce gate above already receives the
# pair, but fixtures resolve the second doorway as E2E_DOORWAY_BETA (household
# fixtures / doorway-footprint-convergence) or E2E_DOORWAY_B (surfaces.ts) —
# without these exports the beta-leg scenarios hard-fail on URL resolution
# ("Cannot resolve doorway URL from: E2E_DOORWAY_BETA", edge #1362) instead of
# probing the live pair.
E2E_DOORWAY_ALPHA="${DOORWAY}" \
E2E_STORAGE_URL="${STORAGE_URL}" \
E2E_DOORWAY_B="${E2E_DOORWAY_B:-https://elohim.host}" \
E2E_DOORWAY_BETA="${E2E_DOORWAY_BETA:-${E2E_DOORWAY_B:-https://elohim.host}}" \
E2E_STORAGE_URL_B="${E2E_STORAGE_URL_B:-http://elohim-adam-alpha.elohim-alpha.svc.cluster.local:8090}" \
  pnpm exec cucumber-js \
    --format "json:${CUCUMBER_OUT}" \
    --tags '@dataplane and not @wip and not @browser-only' 2>&1 \
    | tee "${CUCUMBER_LOG}" \
    || CUCUMBER_EXIT=$?
# pipefail (set above) keeps CUCUMBER_EXIT cucumber's own status through the tee.

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

# Pending-only downgrade for DECLARED born-red concerns. Cucumber exits 1 for
# failed, ambiguous AND undefined steps (and for exceptions), so the decision is
# made from the RAW cucumber JSON, not the sprint report (whose loader folds
# ambiguous into passed/skipped/pending): every step/hook status is collected and
# the run downgrades only when the non-passed/skipped statuses are a non-empty
# subset of {undefined, pending} AND every scenario holding one carries
# @concern:<value> listed in dataplane-born-red-concerns.txt. No JSON (a crash
# before the formatter starts; stale files were removed above) -> no downgrade.
# The sprint report supplies the banner's scenario count only.
BORN_RED_FILE="${WORKSPACE}/scripts/ci/dataplane-born-red-concerns.txt"
if [ "${CUCUMBER_EXIT}" -eq 1 ] && [ "${VALIDATION_EXIT}" -eq 1 ] \
   && [ -r "${CUCUMBER_LOG}" ] && [ -r "${CUCUMBER_OUT}" ] && [ -r "${SPRINT_JSON}" ] && [ -r "${BORN_RED_FILE}" ]; then
  PENDING_INFO="$(python3 -c "
import json, sys
try:
    raw = json.load(open(sys.argv[1]))
    allowed = set()
    for line in open(sys.argv[3]):
        line = line.split('#', 1)[0].strip()
        if line:
            allowed.add(line)
    statuses = set()
    concerns = set()
    for feat in raw:
        ftags = {t.get('name', '') for t in feat.get('tags', []) or []}
        for el in feat.get('elements', []) or []:
            groups = [el.get('before', []), el.get('after', []), el.get('steps', [])]
            st = set()
            for grp in groups:
                for step in grp or []:
                    st.add(((step.get('result') or {}).get('status') or 'unknown'))
            statuses |= st
            if st & {'undefined', 'pending'}:
                tags = ftags | {t.get('name', '') for t in el.get('tags', []) or []}
                hit = {t[len('@concern:'):] for t in tags if t.startswith('@concern:')} & allowed
                if not hit:
                    sys.exit(1)
                concerns |= hit
    bad = statuses - {'passed', 'skipped'}
    if not bad or not bad <= {'undefined', 'pending'}:
        sys.exit(1)
    # Console witness. A formatter stream error (run_cucumber.js:122 fails the run on
    # it; create_stream.js:21-24 logs error.message, which is arbitrary errno text,
    # to stderr) has no distinctive marker, so refuse on any Error/formatter/errno
    # line outside the 'Undefined. Implement with the following snippet' blocks, and
    # require the summary line to name only undefined/pending/skipped/passed.
    import re
    ansi = re.compile(r'\x1b\[[0-9;]*[A-Za-z]')
    errpat = re.compile(r'^\s*Error\b|[Ff]ormatter|\bE(NOSPC|ACCES|PERM|ROFS|MFILE|NOENT|IO|PIPE|BADF|DQUOT|ISDIR)\b')
    summary = None
    in_snippet = False
    for line in open(sys.argv[4], errors='replace'):
        line = ansi.sub('', line.rstrip('\n'))
        if 'Undefined. Implement with the following snippet' in line:
            in_snippet = True
            continue
        if in_snippet:
            if line.strip() == '':
                in_snippet = False
            continue
        if errpat.search(line):
            sys.exit(1)
        m = re.match(r'^(\d+) scenarios? \((.*)\)\s*$', line)
        if m:
            summary = m.group(2)
    if summary is None or re.search(r'failed|ambiguous', summary):
        sys.exit(1)
    rep = json.load(open(sys.argv[2]))['summary']['scenarios']
    print(f\"{int(rep['pending'])}|\" + ', '.join(sorted(concerns)))
except Exception:
    sys.exit(1)
" "${CUCUMBER_OUT}" "${SPRINT_JSON}" "${BORN_RED_FILE}" "${CUCUMBER_LOG}" 2>/dev/null || true)"
  if [ -n "${PENDING_INFO}" ]; then
    echo ""
    echo "PENDING-ONLY (declared born-red concerns: ${PENDING_INFO#*|}): cucumber exited 1 with 0 failed and 0 ambiguous steps; ${PENDING_INFO%%|*} scenario(s) undefined/pending. The fleet verdict is the measured concerns'."
    VALIDATION_EXIT=0
  fi
fi

# Propagate cucumber failures and the measurement invariant above. The
# Jenkinsfile catchError(stageResult:'UNSTABLE') converts non-zero to UNSTABLE
# (advisory) rather than FAILURE.
exit "${VALIDATION_EXIT}"
