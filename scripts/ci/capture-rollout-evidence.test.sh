#!/usr/bin/env bash
# Hermetic regression test for capture-rollout-evidence.sh.

set -euo pipefail

REPO_ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
SCRIPT="${REPO_ROOT}/scripts/ci/capture-rollout-evidence.sh"
JENKINSFILE="${REPO_ROOT}/elohim/holochain/Jenkinsfile"
TEST_ROOT="$(mktemp -d)"
OUTPUT_ROOT="${TEST_ROOT}/evidence"
CALLS="${TEST_ROOT}/kubectl.calls"

cleanup() {
  rm -rf "${TEST_ROOT}"
}
trap cleanup EXIT

kubectl() {
  printf '%s\n' "$*" >> "${FAKE_KUBECTL_CALLS}"
  local args="$*"

  case "${args}" in
    'get deployment/demo -n test-ns -o yaml')
      printf 'kind: Deployment\nmetadata:\n  name: demo\n'
      ;;
    *'get deployment/demo -n test-ns -o go-template='*)
      printf 'app=demo,track=stable,'
      ;;
    'get nodes -o custom-columns='*)
      printf 'Error from server (Forbidden): nodes is forbidden\n' >&2
      return 1
      ;;
    'get pods -n test-ns -l app=demo,track=stable -o wide')
      printf 'NAME               READY   STATUS\ndemo-old           1/1     Running\ndemo-new           0/2     Pending\ndemo-terminating   1/1     Terminating\n'
      ;;
    'get pods -n test-ns -l app=demo,track=stable -o yaml')
      printf 'kind: List\nitems: []\n'
      ;;
    *'get pods -n test-ns -l app=demo,track=stable -o jsonpath='*)
      printf 'demo-old\ndemo-new\ndemo-terminating\n'
      ;;
    *'get pod demo-old -n test-ns -o jsonpath='*'.metadata.name'*)
      printf 'demo-old|Running|True||node-a|web=true/;\n'
      ;;
    *'get pod demo-new -n test-ns -o jsonpath='*'.metadata.name'*)
      printf 'demo-new|Pending|False||node-b|web=false/CrashLoopBackOff;sidecar=false/ContainerCreating;\n'
      ;;
    *'get pod demo-terminating -n test-ns -o jsonpath='*'.metadata.name'*)
      printf 'demo-terminating|Running|True|2026-09-01T20:00:00Z|node-c|web=true/;\n'
      ;;
    *'get pod demo-new -n test-ns -o jsonpath='*'.spec.initContainers'*)
      printf 'prepare\nweb\nsidecar\n'
      ;;
    *'get pod demo-terminating -n test-ns -o jsonpath='*'.spec.initContainers'*)
      printf 'web\n'
      ;;
    'describe pod demo-new -n test-ns'|'describe pod demo-terminating -n test-ns')
      printf 'Conditions:\n  Ready False\nEvents:\n  Warning FailedScheduling\n'
      ;;
    'get events -n test-ns --field-selector involvedObject.kind=Pod,involvedObject.name=demo-new --sort-by=.lastTimestamp -o wide'|'get events -n test-ns --field-selector involvedObject.kind=Pod,involvedObject.name=demo-terminating --sort-by=.lastTimestamp -o wide')
      printf 'Warning FailedScheduling insufficient memory\n'
      ;;
    'logs demo-new -n test-ns -c prepare --tail=200'|'logs demo-new -n test-ns -c web --tail=200'|'logs demo-new -n test-ns -c sidecar --tail=200'|'logs demo-terminating -n test-ns -c web --tail=200')
      printf 'current log\n'
      ;;
    'logs demo-new -n test-ns -c prepare --previous --tail=200'|'logs demo-new -n test-ns -c web --previous --tail=200'|'logs demo-new -n test-ns -c sidecar --previous --tail=200'|'logs demo-terminating -n test-ns -c web --previous --tail=200')
      printf 'previous log\n'
      ;;
    *)
      printf 'unexpected kubectl invocation: %s\n' "${args}" >&2
      return 97
      ;;
  esac
}
export -f kubectl

FAKE_KUBECTL_CALLS="${CALLS}" bash "${SCRIPT}" \
  deployment demo test-ns "${OUTPUT_ROOT}"

ARTIFACT_DIR="${OUTPUT_ROOT}/test-ns--deployment--demo"
grep -Fq '1/3 pods Ready' "${ARTIFACT_DIR}/summary.txt"
grep -Fq 'demo-old=Running/Ready' "${ARTIFACT_DIR}/summary.txt"
grep -Fq 'demo-new=Pending/NotReady' "${ARTIFACT_DIR}/summary.txt"
grep -Fq 'demo-terminating=Running/Terminating(Ready=True)' "${ARTIFACT_DIR}/summary.txt"
grep -Fq 'FailedScheduling' "${ARTIFACT_DIR}/demo-new--describe.txt"
grep -Fq 'insufficient memory' "${ARTIFACT_DIR}/demo-new--events.txt"
grep -Fq 'current log' "${ARTIFACT_DIR}/demo-new--web--current.log"
grep -Fq 'previous log' "${ARTIFACT_DIR}/demo-new--sidecar--previous.log"
grep -Fq 'current log' "${ARTIFACT_DIR}/demo-new--prepare--current.log"
grep -Fq 'previous log' "${ARTIFACT_DIR}/demo-terminating--web--previous.log"
grep -Fq 'Forbidden' "${ARTIFACT_DIR}/node-pressure.txt"
grep -Fq '[exit=1]' "${ARTIFACT_DIR}/node-pressure.txt"

if grep -Fq 'describe pod demo-old' "${CALLS}"; then
  echo 'Ready pod was described; only non-Ready pods should receive deep capture' >&2
  exit 1
fi

grep -Fq "archiveArtifacts artifacts: 'rollout-evidence/**', allowEmptyArchive: true" "${JENKINSFILE}"
if grep -Fq 'peers Ready' "${JENKINSFILE}"; then
  echo 'Jenkins summary still labels rollout testcase outcomes as pod readiness' >&2
  exit 1
fi
if [ "$(grep -c 'waitForRolloutWithEvidence(' "${JENKINSFILE}")" -ne 5 ]; then
  echo 'Expected the helper definition plus all four rollout wait call sites' >&2
  exit 1
fi
if grep -Eq 'kubectl (apply|delete|patch|replace|rollout|scale)' "${SCRIPT}"; then
  echo 'Evidence collector contains a mutating kubectl verb' >&2
  exit 1
fi

echo 'capture-rollout-evidence: mocked capture and Jenkins wiring passed'
