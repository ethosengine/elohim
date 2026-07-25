#!/bin/bash
# saga-sync.sh — pull the latest dataplane sprint-report down from CI, project +
# fulfill the resiliency-saga flow, then print the saga-status one-liner.
#
# The resiliency-saga (genesis/a2o/features/dataplane/resiliency-saga/NN-*.feature) rolls up
# per-concern into genesis/a2o/reports/sprint-report-dataplane.json (gitignored, CI-produced —
# absent on a fresh local checkout). This script closes that loop for local/session use:
#
#   1. Fetch the latest sprint-report-dataplane.json from the elohim-edge Jenkins job's
#      archived artifacts (the "Dataplane Validation" stage — elohim/holochain/Jenkinsfile —
#      archives it with allowEmptyArchive:true, so a 404 is a normal "no run yet / stage
#      didn't produce one" outcome, not a script failure).
#   2. `epr flow project` — (re-)mint the a2o:scenario-green commitments for every recipe in
#      .claude/epr-meta/recipes.yaml, including resiliency-saga's ten chapters.
#   3. `epr flow fulfill` the freshly-synced report — turn its byConcern verdicts into
#      Produce (fulfilling) / Dismiss (regression) FlowEvents in .eprfs/status/flows.jsonl.
#   4. Print the saga-status.py one-liner so the sync's effect is visible immediately.
#
# Idempotent: re-running with the same report is a no-op past step 1 (project/fulfill dedupe
# by content-addressed CID — see fulfill.rs's module doc). Safe to run repeatedly.
#
# Env overrides:
#   JENKINS_BASE   Jenkins base URL (default: https://jenkins.ethosengine.com)
#   JENKINS_JOB    Job name (default: elohim-edge — the job that runs elohim/holochain/Jenkinsfile;
#                  see .claude/skills/pipeline-diagnostics/SKILL.md "Pipeline names vs file paths")
#   JENKINS_BRANCH Branch leg of the job (default: dev)
#   JENKINS_BUILD  Build selector (default: lastSuccessfulBuild)

set -uo pipefail

WORKSPACE="${WORKSPACE:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
REPORTS_DIR="${WORKSPACE}/genesis/a2o/reports"
REPORT_PATH="${REPORTS_DIR}/sprint-report-dataplane.json"
RECIPES_MANIFEST="${WORKSPACE}/elohim/eprfs/Cargo.toml"
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/eprfs-gate-target}"

JENKINS_BASE="${JENKINS_BASE:-https://jenkins.ethosengine.com}"
JENKINS_JOB="${JENKINS_JOB:-elohim-edge}"
JENKINS_BRANCH="${JENKINS_BRANCH:-dev}"
JENKINS_BUILD="${JENKINS_BUILD:-lastSuccessfulBuild}"
ARTIFACT_PATH="genesis/a2o/reports/sprint-report-dataplane.json"
ARTIFACT_URL="${JENKINS_BASE}/job/${JENKINS_JOB}/job/${JENKINS_BRANCH}/${JENKINS_BUILD}/artifact/${ARTIFACT_PATH}"

echo "=== saga-sync ==="
mkdir -p "${REPORTS_DIR}"

# --- (a) fetch the latest sprint-report-dataplane.json --------------------------------
echo "  fetching ${ARTIFACT_URL}"
TMP_REPORT="$(mktemp)"
if curl -fsS --max-time 20 "${ARTIFACT_URL}" -o "${TMP_REPORT}" 2>/dev/null; then
    mv "${TMP_REPORT}" "${REPORT_PATH}"
    echo "  ok: wrote ${REPORT_PATH}"
else
    rm -f "${TMP_REPORT}"
    if [ -f "${REPORT_PATH}" ]; then
        echo "  no artifact at that URL (404/unreachable) — keeping existing local report: ${REPORT_PATH}"
    else
        echo "  no artifact at that URL (404/unreachable), and no existing local report — continuing without one"
    fi
fi

# --- (b) mint/refresh commitments for every recipe (including resiliency-saga) ---------
echo "  epr flow project"
RUSTFLAGS="" CARGO_TARGET_DIR="${CARGO_TARGET_DIR}" \
    cargo run --manifest-path "${RECIPES_MANIFEST}" -q -p elohim-epr-cli -- flow project --root "${WORKSPACE}"
PROJECT_STATUS=$?
if [ "${PROJECT_STATUS}" -ne 0 ]; then
    echo "  WARNING: epr flow project exited ${PROJECT_STATUS} — commitments may be stale"
fi

# --- (c) fulfill the synced report, if we have one -------------------------------------
if [ -f "${REPORT_PATH}" ]; then
    echo "  epr flow fulfill ${ARTIFACT_PATH}"
    RUSTFLAGS="" CARGO_TARGET_DIR="${CARGO_TARGET_DIR}" \
        cargo run --manifest-path "${RECIPES_MANIFEST}" -q -p elohim-epr-cli -- flow fulfill "${ARTIFACT_PATH}" --root "${WORKSPACE}"
    FULFILL_STATUS=$?
    if [ "${FULFILL_STATUS}" -ne 0 ]; then
        echo "  WARNING: epr flow fulfill exited ${FULFILL_STATUS}"
    fi
else
    echo "  skip: no ${ARTIFACT_PATH} to fulfill against"
fi

# --- (d) print the saga-status headline ------------------------------------------------
echo ""
python3 "${WORKSPACE}/.claude/scripts/saga-status.py"
