#!/bin/bash
# jenkins-sync.sh — the Jenkins→fabric bridge script: pull the latest dataplane sprint-report down from CI, project +
# fulfill the resiliency-saga flow, then print the saga-status one-liner.
#
# WHY THIS LIVES IN genesis/scripts/ (bridge-seam provenance, 2026-07-25): the ONLY step
# that knows Jenkins exists is the artifact fetch — an external-protocol translation
# (bridge-shaped, per the seam map's what-do-you-ADD rule); everything downstream
# (`epr flow project|fulfill`) is substrate-native. It sits here with the rest of the
# manual/web2 pipeline orchestration because the whole class is deleted together at the
# rakia graduation: a rakia compute-commitment runner witnesses the verdict at execution
# (signed, on-chain, peer-validated) and there is nothing left to fetch or translate.
# A fetched report is an imported claim and never becomes stronger by being folded —
# the rakia-native verdict is admissible witness evidence instead.
#
# The resiliency-saga (genesis/a2o/features/dataplane/resiliency-saga/NN-*.feature) rolls up
# per-concern into genesis/a2o/reports/sprint-report-dataplane.json (gitignored, CI-produced —
# absent on a fresh local checkout). This script closes that loop for local/session use:
#
#   1. Fetch the latest sprint-report-dataplane.json from the elohim-edge Jenkins job's
#      archived artifacts (the "Dataplane Validation" stage — elohim/holochain/Jenkinsfile —
#      archives it with allowEmptyArchive:true, so "no build has one" is a normal outcome,
#      not a script failure). The build is the NEWEST of the last JENKINS_SCAN builds that
#      HAS the artifact, whatever its result: since the one-head-delivered sprint (Lane C3)
#      the deploy run skips the stage and a validate-only sibling measures, so
#      lastSuccessfulBuild is usually a deploy run with no report, and a red sibling
#      (FAILURE) is exactly the measurement to sync.
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
#   JENKINS_BUILD  Explicit build selector (a number or e.g. lastSuccessfulBuild); unset =
#                  the newest build carrying the artifact
#   JENKINS_SCAN   How many recent builds to search for the artifact (default: 10)

set -uo pipefail

WORKSPACE="${WORKSPACE:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
REPORTS_DIR="${WORKSPACE}/genesis/a2o/reports"
REPORT_PATH="${REPORTS_DIR}/sprint-report-dataplane.json"
RECIPES_MANIFEST="${WORKSPACE}/elohim/eprfs/Cargo.toml"
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/eprfs-gate-target}"

JENKINS_BASE="${JENKINS_BASE:-https://jenkins.ethosengine.com}"
JENKINS_JOB="${JENKINS_JOB:-elohim-edge}"
JENKINS_BRANCH="${JENKINS_BRANCH:-dev}"
JENKINS_BUILD="${JENKINS_BUILD:-}"
JENKINS_SCAN="${JENKINS_SCAN:-10}"
ARTIFACT_PATH="genesis/a2o/reports/sprint-report-dataplane.json"
JOB_URL="${JENKINS_BASE}/job/${JENKINS_JOB}/job/${JENKINS_BRANCH}"

# The newest of the last JENKINS_SCAN builds whose artifacts include ARTIFACT_PATH, as
# "<number> <result>" (empty when none does, or Jenkins is unreachable). The builds array
# is newest-first; each build object opens with the WorkflowRun _class (artifacts carry
# none), so one build per line after the split. -g: the tree's [] and {} are not globs.
newest_build_with_artifact() {
    local build
    build="$(curl -gfsS --max-time 20 \
        "${JOB_URL}/api/json?tree=builds[number,result,artifacts[relativePath]]{0,${JENKINS_SCAN}}" 2>/dev/null \
        | sed 's/{"_class":"org.jenkinsci.plugins.workflow.job.WorkflowRun"/\n/g' \
        | grep -F "\"relativePath\":\"${ARTIFACT_PATH}\"" \
        | head -n1)"
    [ -n "${build}" ] || return 0
    printf '%s %s\n' \
        "$(printf '%s' "${build}" | grep -o '"number":[0-9]*' | head -n1 | cut -d: -f2)" \
        "$(printf '%s' "${build}" | grep -o '"result":"[A-Z_]*"' | head -n1 | cut -d'"' -f4)"
}

echo "=== jenkins-sync ==="
mkdir -p "${REPORTS_DIR}"

if [ -z "${JENKINS_BUILD}" ]; then
    FOUND="$(newest_build_with_artifact)"
    JENKINS_BUILD="${FOUND%% *}"
    if [ -n "${JENKINS_BUILD}" ]; then
        echo "  newest ${JENKINS_JOB}/${JENKINS_BRANCH} build with the report: #${JENKINS_BUILD} (${FOUND#* })"
    else
        echo "  none of the last ${JENKINS_SCAN} ${JENKINS_JOB}/${JENKINS_BRANCH} builds archived ${ARTIFACT_PATH}"
    fi
fi
ARTIFACT_URL="${JOB_URL}/${JENKINS_BUILD:-lastSuccessfulBuild}/artifact/${ARTIFACT_PATH}"

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
