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

mkdir -p "${TEST_WORKSPACE}/genesis/a2o"

pnpm() {
  if [ "${1:-}" = "install" ]; then
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
    printf '[]\n' > "${cucumber_out}"
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
    printf '{\n  "summary": {\n    "scenarios": {\n      "total": %s\n    },\n    "byConcern": {}\n  }\n}\n' \
      "${TEST_SCENARIO_COUNT}" > "${out_json}"
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

echo "run-dataplane-validation: zero-scenario guard tests passed"
