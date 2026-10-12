#!/bin/bash
# Regression coverage for run-dataplane-validation.sh's measurement invariant.
# Uses an exported pnpm function so no network, dependency install, or deployed
# doorway is required; the production wrapper still executes end-to-end.

set -euo pipefail

REPO_ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
SCRIPT="${REPO_ROOT}/scripts/ci/run-dataplane-validation.sh"
TEST_ROOT="$(mktemp -d)"
TEST_WORKSPACE="${TEST_ROOT}/workspace"

cleanup() {
  rm -rf "${TEST_ROOT}"
}
trap cleanup EXIT

mkdir -p "${TEST_WORKSPACE}/genesis/a2o" "${TEST_WORKSPACE}/scripts/ci"
cp "${REPO_ROOT}/scripts/ci/dataplane-born-red-concerns.txt" "${TEST_WORKSPACE}/scripts/ci/"

# Raw cucumber JSON fixture: one scenario with the given concern tag and step statuses.
cuke_json() {
  local concern="$1"; shift
  local steps="" st
  for st in "$@"; do steps="${steps}{\"result\":{\"status\":\"${st}\"}},"; done
  printf '[{"tags":[],"elements":[{"type":"scenario","tags":[{"name":"@concern:%s"}],"steps":[%s{"result":{"status":"passed"}}]}]}]' \
    "${concern}" "${steps}"
}

pnpm() {
  if [ "${1:-}" = "install" ]; then
    return 0
  fi

  # The a2o support code imports @elohim/storage-client, which resolves to a
  # built dist/. Record that the wrapper builds it before cucumber loads.
  if [ "${1:-}" = "--filter" ] && [ "${2:-}" = "@elohim/storage-client" ] && [ "${3:-}" = "build" ]; then
    : > "${WORKSPACE}/.storage-client-built"
    return 0
  fi

  if [ "${1:-}" = "exec" ] && [ "${2:-}" = "cucumber-js" ]; then
    local arg cucumber_out=""
    for arg in "$@"; do
      case "${arg}" in
        json:*) cucumber_out="${arg#json:}" ;;
      esac
    done
    mkdir -p "$(dirname "${cucumber_out}")"
    if [ -z "${TEST_NO_REPORT:-}" ]; then
      printf '%s\n' "${TEST_CUCUMBER_LOG:-1 scenario (1 undefined)}"
      printf '%s\n' "${TEST_CUCUMBER_JSON:-[]}" > "${cucumber_out}"
    fi
    return "${TEST_CUCUMBER_EXIT:-0}"
  fi

  if [ "${1:-}" = "exec" ] && [ "${2:-}" = "tsx" ]; then
    local out_json="" out_md=""
    shift 2
    while [ "$#" -gt 0 ]; do
      case "$1" in
        --out-json)
          out_json="$2"
          shift 2
          ;;
        --out-md)
          out_md="$2"
          shift 2
          ;;
        *) shift ;;
      esac
    done
    if [ -n "${TEST_NO_REPORT:-}" ]; then
      return 0
    fi
    printf '{\n  "summary": {\n    "scenarios": {\n      "total": %s,\n      "failed": %s,\n      "pending": %s\n    },\n    "byConcern": %s\n  }\n}\n' \
      "${TEST_SCENARIO_COUNT}" "${TEST_FAILED:-0}" "${TEST_PENDING:-0}" "${TEST_BYCONCERN:-{\}}" > "${out_json}"
    printf '# test report\n' > "${out_md}"
    return 0
  fi

  echo "Unexpected pnpm invocation: $*" >&2
  return 99
}
export -f pnpm

run_case() {
  local scenario_count="$1" cucumber_exit="$2" output_file="$3"
  set +e
  TEST_SCENARIO_COUNT="${scenario_count}" \
  TEST_CUCUMBER_EXIT="${cucumber_exit}" \
  QUIESCE_SKIP=1 \
  WORKSPACE="${TEST_WORKSPACE}" \
    bash "${SCRIPT}" > "${output_file}" 2>&1
  local status=$?
  set -e
  printf '%s\n' "${status}"
}

ZERO_OUTPUT="${TEST_ROOT}/zero.log"
ZERO_STATUS="$(run_case 0 0 "${ZERO_OUTPUT}")"
if [ "${ZERO_STATUS}" -ne 3 ]; then
  echo "Expected zero scenarios to exit 3; got ${ZERO_STATUS}" >&2
  sed -n '1,200p' "${ZERO_OUTPUT}" >&2
  exit 1
fi
if ! grep -Fq '0 dataplane scenarios ran' "${ZERO_OUTPUT}"; then
  echo "Zero-scenario failure did not explain the broken measurement" >&2
  sed -n '1,200p' "${ZERO_OUTPUT}" >&2
  exit 1
fi

PASS_OUTPUT="${TEST_ROOT}/pass.log"
PASS_STATUS="$(run_case 2 0 "${PASS_OUTPUT}")"
if [ "${PASS_STATUS}" -ne 0 ]; then
  echo "Expected a measured run to pass; got ${PASS_STATUS}" >&2
  sed -n '1,200p' "${PASS_OUTPUT}" >&2
  exit 1
fi

if [ ! -e "${TEST_WORKSPACE}/.storage-client-built" ]; then
  echo "Expected @elohim/storage-client to be built before the cucumber run (edge 1464: MODULE_NOT_FOUND -> 0 scenarios)" >&2
  sed -n '1,200p' "${PASS_OUTPUT}" >&2
  exit 1
fi

INVALID_OUTPUT="${TEST_ROOT}/invalid.log"
INVALID_STATUS="$(run_case invalid 0 "${INVALID_OUTPUT}")"
if [ "${INVALID_STATUS}" -ne 3 ]; then
  echo "Expected an unreadable scenario count to exit 3; got ${INVALID_STATUS}" >&2
  sed -n '1,200p' "${INVALID_OUTPUT}" >&2
  exit 1
fi
if ! grep -Fq 'scenario count is missing or unreadable' "${INVALID_OUTPUT}"; then
  echo "Invalid-count failure did not explain the broken report" >&2
  sed -n '1,200p' "${INVALID_OUTPUT}" >&2
  exit 1
fi

CUCUMBER_OUTPUT="${TEST_ROOT}/cucumber-failure.log"
CUCUMBER_STATUS="$(run_case 2 7 "${CUCUMBER_OUTPUT}")"
if [ "${CUCUMBER_STATUS}" -ne 7 ]; then
  echo "Expected cucumber exit 7 to be preserved; got ${CUCUMBER_STATUS}" >&2
  sed -n '1,200p' "${CUCUMBER_OUTPUT}" >&2
  exit 1
fi

expect_case() { # name expected_status banner(yes|no) output status
  if [ "$5" -ne "$2" ]; then
    echo "$1: expected exit $2; got $5" >&2; sed -n '1,200p' "$4" >&2; exit 1
  fi
  if [ "$3" = yes ] && ! grep -Fq 'PENDING-ONLY (declared born-red concerns: pain-is-answered): cucumber exited 1 with 0 failed and 0 ambiguous steps; 14 scenario(s)' "$4"; then
    echo "$1: expected the PENDING-ONLY banner" >&2; sed -n '1,200p' "$4" >&2; exit 1
  fi
  if [ "$3" = no ] && grep -Fq 'PENDING-ONLY' "$4"; then
    echo "$1: must not print the PENDING-ONLY banner" >&2; exit 1
  fi
}

O="${TEST_ROOT}/c-undef-listed.log"
S="$(TEST_PENDING=14 TEST_CUCUMBER_JSON="$(cuke_json pain-is-answered undefined)" run_case 20 1 "$O")"
expect_case "undefined under listed concern" 0 yes "$O" "$S"

O="${TEST_ROOT}/c-ambiguous.log"
S="$(TEST_PENDING=14 TEST_CUCUMBER_JSON="$(cuke_json pain-is-answered undefined ambiguous)" run_case 20 1 "$O")"
expect_case "ambiguous step" 1 no "$O" "$S"

O="${TEST_ROOT}/c-failed.log"
S="$(TEST_FAILED=1 TEST_PENDING=14 TEST_CUCUMBER_JSON="$(cuke_json pain-is-answered undefined failed)" run_case 20 1 "$O")"
expect_case "failed step" 1 no "$O" "$S"

O="${TEST_ROOT}/c-unlisted.log"
S="$(TEST_PENDING=14 TEST_CUCUMBER_JSON="$(cuke_json some-other-concern undefined)" run_case 20 1 "$O")"
expect_case "undefined under non-listed concern" 1 no "$O" "$S"

O="${TEST_ROOT}/c-fmt.log"
S="$(TEST_PENDING=14 TEST_CUCUMBER_LOG=$'ENOSPC: no space left on device, write\n1 scenario (1 undefined)' TEST_CUCUMBER_JSON="$(cuke_json pain-is-answered undefined)" run_case 20 1 "$O")"
expect_case "formatter error marker" 1 no "$O" "$S"

O="${TEST_ROOT}/c-noresult.log"
S="$(TEST_PENDING=14 TEST_CUCUMBER_JSON='[{"tags":[],"elements":[{"type":"scenario","tags":[{"name":"@concern:pain-is-answered"}],"steps":[{"result":{"status":"undefined"}},{}]}]}]' run_case 20 1 "$O")"
expect_case "step without result" 1 no "$O" "$S"

O="${TEST_ROOT}/c-nosummary.log"
S="$(TEST_PENDING=14 TEST_CUCUMBER_LOG='progress only' TEST_CUCUMBER_JSON="$(cuke_json pain-is-answered undefined)" run_case 20 1 "$O")"
expect_case "summary line absent" 1 no "$O" "$S"

O="${TEST_ROOT}/c-noreport.log"
S="$(TEST_NO_REPORT=1 run_case 20 1 "$O")"
expect_case "report absent" 3 no "$O" "$S"
if [ -e "${TEST_WORKSPACE}/genesis/a2o/reports/cucumber-report-dataplane.json" ]; then
  echo "stale cucumber report was not removed" >&2; exit 1
fi

echo "run-dataplane-validation: zero-scenario guard tests passed"
