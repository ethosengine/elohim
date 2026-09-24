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

# --- --since-conductor: a SUCCESSFUL roll still yields a measured receipt row ---
# Every pod is Ready, so the default mode keeps no log at all; the mode reads
# each storage container's log since its conductor pod's creationTimestamp and
# keeps the conductor-app lines, and conductor-recovery-receipt.sh measures it.
SINCE_ROOT="${TEST_ROOT}/since"
SINCE_CALLS="${TEST_ROOT}/since.calls"
kubectl() {
  printf '%s\n' "$*" >> "${FAKE_KUBECTL_CALLS}"
  local args="$*" s='elohim-matthew-alpha' ns='elohim-alpha'
  case "${args}" in
    "get statefulset/${s}-conductor -n ${ns} -o yaml"|"get statefulset/${s} -n ${ns} -o yaml")
      printf 'kind: StatefulSet\n' ;;
    *"get statefulset/${s}-conductor -n ${ns} -o go-template="*) printf 'app=conductor,' ;;
    *"get statefulset/${s} -n ${ns} -o go-template="*) printf 'app=storage,' ;;
    'get nodes -o custom-columns='*) printf 'NAME\nnode-a\n' ;;
    "get pods -n ${ns} -l app=conductor -o wide"|"get pods -n ${ns} -l app=storage -o wide")
      printf 'NAME READY\n' ;;
    "get pods -n ${ns} -l app=conductor -o yaml")
      printf 'apiVersion: v1\nitems:\n- apiVersion: v1\n  kind: Pod\n  metadata:\n    creationTimestamp: "2026-09-25T10:00:00Z"\n    name: %s-conductor-0\n  spec:\n    containers:\n    - name: conductor\nkind: List\n' "${s}" ;;
    "get pods -n ${ns} -l app=storage -o yaml") printf 'kind: List\nitems: []\n' ;;
    *"get pods -n ${ns} -l app=conductor -o jsonpath="*) printf '%s-conductor-0\n' "${s}" ;;
    *"get pods -n ${ns} -l app=storage -o jsonpath="*) printf '%s-0\n' "${s}" ;;
    *"get pod ${s}-conductor-0 -n ${ns} -o jsonpath={.metadata.creationTimestamp}") printf '2026-09-25T10:00:00Z' ;;
    *"get pod ${s}-conductor-0 -n ${ns} -o jsonpath="*'.metadata.name'*)
      printf '%s-conductor-0|Running|True||node-a|conductor=true/;\n' "${s}" ;;
    *"get pod ${s}-0 -n ${ns} -o jsonpath="*'.metadata.name'*)
      printf '%s-0|Running|True||node-a|elohim-node=true/;\n' "${s}" ;;
    *"get pod ${s}-0 -n ${ns} -o jsonpath={range .spec.containers[*]}"*) printf 'elohim-node\n' ;;
    "logs ${s}-0 -n ${ns} -c elohim-node --since-time=2026-09-25T10:00:00Z")
      printf '%s\n' \
        '{"timestamp":"2026-09-25T10:00:05.000000Z","level":"INFO","fields":{"message":"sync sweep tick"},"target":"elohim_storage::sync"}' \
        '{"timestamp":"2026-09-25T10:01:00.000000Z","level":"WARN","fields":{"message":"conductor app is NOT RUNNING (CellDisabled)","role":"lamad"},"target":"elohim_storage::conductor_bridge_health"}' \
        '{"timestamp":"2026-09-25T10:07:12.000000Z","level":"INFO","fields":{"message":"conductor app is RUNNING again — a zome call on this role SUCCEEDED","role":"lamad"},"target":"elohim_storage::conductor_bridge_health"}' ;;
    *)
      printf 'unexpected kubectl invocation: %s\n' "${args}" >&2
      return 97 ;;
  esac
}
export -f kubectl

FAKE_KUBECTL_CALLS="${SINCE_CALLS}" bash "${SCRIPT}" --since-conductor \
  elohim-matthew-alpha elohim-alpha "${SINCE_ROOT}" > "${TEST_ROOT}/since.out"

SINCE_LOG="${SINCE_ROOT}/elohim-alpha--statefulset--elohim-matthew-alpha/elohim-matthew-alpha-0--elohim-node--since-conductor.log"
[ -f "${SINCE_LOG}" ] || { echo "--since-conductor wrote no ${SINCE_LOG}" >&2; exit 1; }
grep -Fq 'conductor app is RUNNING again' "${SINCE_LOG}"
grep -Fq 'conductor app is NOT RUNNING' "${SINCE_LOG}"
if grep -Fq 'sync sweep tick' "${SINCE_LOG}"; then
  echo '--since-conductor kept a line that is not a conductor-app line' >&2
  exit 1
fi
grep -Fq '[exit=0 scanned=3]' "${SINCE_LOG}"
grep -Fq '1 storage container log(s) captured' "${TEST_ROOT}/since.out"
[ -f "${SINCE_ROOT}/elohim-alpha--statefulset--elohim-matthew-alpha-conductor/pods.yaml" ]
if grep -Fq 'unexpected kubectl invocation' "${SINCE_ROOT}"/*/*.stderr 2>/dev/null; then
  echo '--since-conductor made a kubectl call the fake does not model' >&2
  exit 1
fi
if grep -Eq '^(describe|delete|apply|patch|rollout|scale) ' "${SINCE_CALLS}"; then
  echo '--since-conductor on an all-Ready roll described or mutated something' >&2
  exit 1
fi

receipt_out="$(bash "${REPO_ROOT}/scripts/ci/conductor-recovery-receipt.sh" --was 'pre-fix' "${SINCE_ROOT}")" || {
  echo "conductor-recovery-receipt did not measure the --since-conductor tree: ${receipt_out}" >&2
  exit 1
}
case "${receipt_out}" in
  *'elohim-matthew-alpha'*'7m12s'*'lamad'*) ;;
  *) echo "receipt row wrong: ${receipt_out}" >&2; exit 1 ;;
esac

echo 'capture-rollout-evidence: mocked capture and Jenkins wiring passed'
