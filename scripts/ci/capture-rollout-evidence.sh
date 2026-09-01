#!/usr/bin/env bash
# capture-rollout-evidence.sh — preserve pod-level evidence when a rollout fails.
#
# Runtime contract: bash + coreutils + kubectl only. The edge deploy container
# deliberately has no python/pip/PyYAML (edge #1183).
#
# Usage:
#   capture-rollout-evidence.sh <kind> <name> <namespace> [output-root]
#
# The collector is diagnostic and best-effort: every failed read is recorded in
# its artifact, but the script exits zero so it can never replace the rollout's
# original failure with an observability failure.

set -u

KIND="${1:?usage: capture-rollout-evidence.sh <kind> <name> <namespace> [output-root]}"
NAME="${2:?usage: capture-rollout-evidence.sh <kind> <name> <namespace> [output-root]}"
NAMESPACE="${3:?usage: capture-rollout-evidence.sh <kind> <name> <namespace> [output-root]}"
OUTPUT_ROOT="${4:-rollout-evidence}"

safe_kind="${KIND//\//-}"
artifact_dir="${OUTPUT_ROOT}/${NAMESPACE}--${safe_kind}--${NAME}"
mkdir -p "${artifact_dir}"

run_to_file() {
  local output_file="$1"
  shift
  {
    printf '$'
    printf ' %q' "$@"
    printf '\n'
    "$@"
    local command_status=$?
    printf '\n[exit=%s]\n' "${command_status}"
    return "${command_status}"
  } > "${output_file}" 2>&1
}

printf 'captured_at=%s\nworkload=%s/%s\nnamespace=%s\n' \
  "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "${KIND}" "${NAME}" "${NAMESPACE}" \
  > "${artifact_dir}/capture.meta"

run_to_file "${artifact_dir}/workload.yaml" \
  kubectl get "${KIND}/${NAME}" -n "${NAMESPACE}" -o yaml || true

selector_stderr="${artifact_dir}/selector.stderr"
selector="$(kubectl get "${KIND}/${NAME}" -n "${NAMESPACE}" \
  -o go-template='{{range $key, $value := .spec.selector.matchLabels}}{{printf "%s=%s," $key $value}}{{end}}' \
  2> "${selector_stderr}")"
selector_status=$?
selector="${selector%,}"
printf 'selector=%s\nexit=%s\n' "${selector}" "${selector_status}" \
  > "${artifact_dir}/selector.txt"

run_to_file "${artifact_dir}/node-pressure.txt" kubectl get nodes \
  -o 'custom-columns=NAME:.metadata.name,UNSCHEDULABLE:.spec.unschedulable,READY:.status.conditions[?(@.type=="Ready")].status,MEMORY_PRESSURE:.status.conditions[?(@.type=="MemoryPressure")].status,DISK_PRESSURE:.status.conditions[?(@.type=="DiskPressure")].status,PID_PRESSURE:.status.conditions[?(@.type=="PIDPressure")].status,ALLOCATABLE_CPU:.status.allocatable.cpu,ALLOCATABLE_MEMORY:.status.allocatable.memory' \
  || true

if [ "${selector_status}" -ne 0 ] || [ -z "${selector}" ]; then
  summary="Actual pod readiness for ${KIND}/${NAME} in ${NAMESPACE}: unavailable — workload selector could not be resolved"
  printf '%s\n' "${summary}" | tee "${artifact_dir}/summary.txt"
  exit 0
fi

run_to_file "${artifact_dir}/pods-wide.txt" kubectl get pods -n "${NAMESPACE}" \
  -l "${selector}" -o wide || true
run_to_file "${artifact_dir}/pods.yaml" kubectl get pods -n "${NAMESPACE}" \
  -l "${selector}" -o yaml || true

pod_names_stderr="${artifact_dir}/pod-names.stderr"
pod_names="$(kubectl get pods -n "${NAMESPACE}" -l "${selector}" \
  -o jsonpath='{range .items[*]}{.metadata.name}{"\n"}{end}' \
  2> "${pod_names_stderr}")"
pod_names_status=$?

ready_count=0
pod_count=0
pod_summaries=""
printf 'pod\tphase\tready\tdeletionTimestamp\tnode\tcontainers\n' \
  > "${artifact_dir}/pod-state.tsv"

if [ "${pod_names_status}" -eq 0 ]; then
  while IFS= read -r pod; do
    [ -n "${pod}" ] || continue
    pod_count=$((pod_count + 1))

    state_stderr="${artifact_dir}/${pod}--state.stderr"
    state_line="$(kubectl get pod "${pod}" -n "${NAMESPACE}" \
      -o jsonpath='{.metadata.name}{"|"}{.status.phase}{"|"}{.status.conditions[?(@.type=="Ready")].status}{"|"}{.metadata.deletionTimestamp}{"|"}{.spec.nodeName}{"|"}{range .status.initContainerStatuses[*]}{.name}{"="}{.ready}{"/"}{.state.waiting.reason}{.state.terminated.reason}{";"}{end}{range .status.containerStatuses[*]}{.name}{"="}{.ready}{"/"}{.state.waiting.reason}{.state.terminated.reason}{";"}{end}{"\n"}' \
      2> "${state_stderr}")"
    state_status=$?

    if [ "${state_status}" -ne 0 ]; then
      printf '%s\tunavailable\tUnknown\t\t\t\n' "${pod}" >> "${artifact_dir}/pod-state.tsv"
      state_summary="${pod}=Unavailable"
      ready="Unknown"
      deletion_timestamp=""
    else
      IFS='|' read -r state_pod phase ready deletion_timestamp node containers <<< "${state_line}"
      printf '%s\t%s\t%s\t%s\t%s\t%s\n' \
        "${state_pod}" "${phase}" "${ready}" "${deletion_timestamp}" "${node}" "${containers}" \
        >> "${artifact_dir}/pod-state.tsv"
      if [ "${ready}" = "True" ] && [ -z "${deletion_timestamp}" ]; then
        ready_count=$((ready_count + 1))
        readiness="Ready"
      elif [ -n "${deletion_timestamp}" ]; then
        readiness="Terminating(Ready=${ready:-Unknown})"
      else
        readiness="NotReady"
      fi
      state_summary="${state_pod}=${phase}/${readiness}[node=${node:-unassigned};containers=${containers:-none}]"
    fi

    if [ -n "${pod_summaries}" ]; then
      pod_summaries="${pod_summaries}; ${state_summary}"
    else
      pod_summaries="${state_summary}"
    fi

    if [ "${ready}" != "True" ] || [ -n "${deletion_timestamp}" ]; then
      run_to_file "${artifact_dir}/${pod}--describe.txt" \
        kubectl describe pod "${pod}" -n "${NAMESPACE}" || true
      run_to_file "${artifact_dir}/${pod}--events.txt" kubectl get events \
        -n "${NAMESPACE}" \
        --field-selector "involvedObject.kind=Pod,involvedObject.name=${pod}" \
        --sort-by=.lastTimestamp -o wide || true

      containers_stderr="${artifact_dir}/${pod}--containers.stderr"
      containers="$(kubectl get pod "${pod}" -n "${NAMESPACE}" \
        -o jsonpath='{range .spec.initContainers[*]}{.name}{"\n"}{end}{range .spec.containers[*]}{.name}{"\n"}{end}' \
        2> "${containers_stderr}")"
      while IFS= read -r container; do
        [ -n "${container}" ] || continue
        run_to_file "${artifact_dir}/${pod}--${container}--current.log" \
          kubectl logs "${pod}" -n "${NAMESPACE}" -c "${container}" --tail=200 || true
        run_to_file "${artifact_dir}/${pod}--${container}--previous.log" \
          kubectl logs "${pod}" -n "${NAMESPACE}" -c "${container}" --previous --tail=200 || true
      done <<< "${containers}"
    fi
  done <<< "${pod_names}"
fi

if [ "${pod_names_status}" -ne 0 ]; then
  summary="Actual pod readiness for ${KIND}/${NAME} in ${NAMESPACE}: unavailable — pod state query failed"
elif [ "${pod_count}" -eq 0 ]; then
  summary="Actual pod readiness for ${KIND}/${NAME} in ${NAMESPACE}: 0/0 pods Ready — no matching pods"
else
  summary="Actual pod readiness for ${KIND}/${NAME} in ${NAMESPACE}: ${ready_count}/${pod_count} pods Ready — ${pod_summaries}"
fi

printf '%s\n' "${summary}" | tee "${artifact_dir}/summary.txt"
exit 0
