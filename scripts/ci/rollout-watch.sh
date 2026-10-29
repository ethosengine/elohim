#!/usr/bin/env bash
# rollout-watch.sh — wait for one workload rollout, and give up on a
# crashlooping StatefulSet pod FAST instead of waiting out the whole timeout.
#
# WHY: an edge deploy waited the full `kubectl rollout status --timeout=600s`
# for every storage peer whose pod was in CrashLoopBackOff ("Disk quota
# exceeded"). A container that has crashed three times on the revision being
# rolled will not come Ready inside the remaining window, and every such peer
# cost the deploy ten minutes of wall-clock (45-60 min per deploy in all).
#
# Runtime contract: bash + coreutils + kubectl only (the edge deploy container
# has no python/jq — see capture-rollout-evidence.sh). Read-only: the only
# kubectl verbs are `rollout status` and `get`.
#
# Usage:
#   rollout-watch.sh <kind> <name> <namespace> <timeout> [output-root]
#
# Behaviour:
#   * `kubectl rollout status <kind>/<name> -n <ns> --timeout=<timeout>` runs
#     exactly as before, its output streaming to the log. A rollout that
#     completes returns the moment kubectl does — there is no polling delay on
#     the healthy path — and a rollout that fails returns kubectl's own exit.
#   * For a statefulset only, while that wait runs, the pods of the
#     statefulset's UPDATE revision (controller-revision-hash ==
#     .status.updateRevision, not terminating) are read every
#     ROLLOUT_FASTFAIL_POLL_SECS. A pod of an older revision is never judged:
#     replacing a crashing old pod is what a rollout is for.
#   * A container (init or regular) is judged crashlooping when
#       - it is waiting in CrashLoopBackOff with restartCount >= MIN_RESTARTS, or
#       - its restartCount rose on RISE_POLLS consecutive polls.
#     The rollout wait is then stopped and the script exits 4 after writing
#       <output-root>/<ns>--statefulset--<name>/fast-fail.reason  (one line)
#       <output-root>/<ns>--statefulset--<name>/fast-fail.txt     (detail)
#     into the same directory capture-rollout-evidence.sh fills, so the
#     archived rollout-evidence/** bundle carries the reason next to the pod
#     state, events and logs.
#   * Any other kind, ROLLOUT_FASTFAIL=0, or a malformed knob runs the plain
#     wait unchanged (exec). A pod read that fails skips that poll; it never
#     fails the rollout.
#
# Env knobs:
#   ROLLOUT_FASTFAIL                1 (default) watch for crashloops; 0 = off
#   ROLLOUT_FASTFAIL_POLL_SECS      seconds between pod reads (default 15)
#   ROLLOUT_FASTFAIL_MIN_RESTARTS   CrashLoopBackOff restart floor (default 3)
#   ROLLOUT_FASTFAIL_RISE_POLLS     consecutive rising polls (default 3)
#
# Exit codes:
#   0          rollout complete
#   4          fast-failed: a pod of the update revision is crashlooping
#   otherwise  kubectl rollout status's own exit (timeout, error, usage=2)

set -u

usage() {
  echo "Usage: $(basename "$0") <kind> <name> <namespace> <timeout> [output-root]" >&2
}

if [ "$#" -lt 4 ]; then
  usage
  exit 2
fi

KIND="$1"
NAME="$2"
NAMESPACE="$3"
TIMEOUT="$4"
OUTPUT_ROOT="${5:-rollout-evidence}"
FAST_FAIL_EXIT=4

ENABLED="${ROLLOUT_FASTFAIL:-1}"
POLL_SECS="${ROLLOUT_FASTFAIL_POLL_SECS:-15}"
MIN_RESTARTS="${ROLLOUT_FASTFAIL_MIN_RESTARTS:-3}"
RISE_POLLS="${ROLLOUT_FASTFAIL_RISE_POLLS:-3}"

plain_wait() {
  exec kubectl rollout status "${KIND}/${NAME}" -n "${NAMESPACE}" "--timeout=${TIMEOUT}"
}

if [ "${ENABLED}" = "0" ] || [ "${KIND}" != "statefulset" ]; then
  plain_wait
fi

for knob in "ROLLOUT_FASTFAIL_POLL_SECS=${POLL_SECS}" \
            "ROLLOUT_FASTFAIL_MIN_RESTARTS=${MIN_RESTARTS}" \
            "ROLLOUT_FASTFAIL_RISE_POLLS=${RISE_POLLS}"; do
  if ! [[ "${knob#*=}" =~ ^[1-9][0-9]*$ ]]; then
    echo "rollout-watch: ${knob%%=*} must be a positive integer, got '${knob#*=}' — crashloop watch OFF, plain rollout wait" >&2
    plain_wait
  fi
done

# Field and record separators the go-templates below emit: a termination
# message may hold newlines and pipes, so neither can delimit.
US=$'\x1f'
RS=$'\x1e'

# Every nested read sits under a `with`: kubectl's go-template errors with
# "nil pointer evaluating interface {}.<field>" when a PARENT field is absent
# (a pod with no labels, a container status without lastState), and an error
# here would only skip the poll — the guard keeps the poll measuring.
STS_TEMPLATE='{{with .status}}{{.updateRevision}}{{end}}{{"\x1f"}}{{with .spec}}{{with .selector}}{{range $key, $value := .matchLabels}}{{printf "%s=%s," $key $value}}{{end}}{{end}}{{end}}'
LAST_TERMINATED_NONE='{{"\x1f"}}{{"\x1f"}}'
CONTAINER_FIELDS='{{$p}}{{"\x1f"}}{{$r}}{{"\x1f"}}{{$d}}{{"\x1f"}}{{.name}}{{"\x1f"}}{{.restartCount}}{{"\x1f"}}{{with .state}}{{with .waiting}}{{.reason}}{{end}}{{end}}{{"\x1f"}}{{with .lastState}}{{with .terminated}}{{.reason}}{{"\x1f"}}{{.exitCode}}{{"\x1f"}}{{.message}}{{else}}'"${LAST_TERMINATED_NONE}"'{{end}}{{else}}'"${LAST_TERMINATED_NONE}"'{{end}}{{"\x1e"}}'
PODS_TEMPLATE='{{range .items}}{{$p := .metadata.name}}{{$r := ""}}{{with .metadata.labels}}{{$r = index . "controller-revision-hash"}}{{end}}{{$d := .metadata.deletionTimestamp}}{{with .status}}{{range .initContainerStatuses}}'"${CONTAINER_FIELDS}"'{{end}}{{range .containerStatuses}}'"${CONTAINER_FIELDS}"'{{end}}{{end}}{{end}}'

# go-template prints "<no value>" for an absent field.
present() {
  [ -n "$1" ] && [ "$1" != "<no value>" ]
}

# One line, bounded: termination messages can be up to 4 KiB of log tail.
one_line() {
  local s="${1//[$'\n\r\t']/ }"
  s="$(printf '%s' "${s}" | tr -s ' ')"
  s="${s# }"
  s="${s% }"
  if [ "${#s}" -gt 300 ]; then s="${s:0:300}…"; fi
  printf '%s' "${s}"
}

declare -A last_restarts=()
declare -A rises=()
selector=""
read_warned=0
verdict=""
verdict_table=""

warn_read() {
  if [ "${read_warned}" -eq 0 ]; then
    echo "rollout-watch: pod read for statefulset/${NAME} failed ($1) — skipping polls until it reads; the rollout wait itself is unaffected" >&2
    read_warned=1
  fi
}

# Read the update revision's pods once; set `verdict` when one crashloops.
probe() {
  local sts_line revision pods_raw record
  sts_line="$(kubectl get "statefulset/${NAME}" -n "${NAMESPACE}" -o "go-template=${STS_TEMPLATE}" 2>/dev/null)" \
    || { warn_read "statefulset get"; return 0; }
  revision="${sts_line%%"${US}"*}"
  if [ -z "${selector}" ]; then
    selector="${sts_line#*"${US}"}"
    selector="${selector%,}"
  fi
  if ! present "${revision}" || [ -z "${selector}" ]; then
    warn_read "update revision or selector unresolved"
    return 0
  fi
  pods_raw="$(kubectl get pods -n "${NAMESPACE}" -l "${selector}" -o "go-template=${PODS_TEMPLATE}" 2>/dev/null)" \
    || { warn_read "pods get"; return 0; }

  verdict_table=""
  while IFS= read -r -d "${RS}" record; do
    [ -n "${record}" ] || continue
    local pod rev deleted container restarts waiting last_reason last_exit last_message
    IFS="${US}" read -r -d '' pod rev deleted container restarts waiting last_reason last_exit last_message <<< "${record}"
    present "${last_message:-}" || last_message=""
    last_message="$(one_line "${last_message}")"
    [ -n "${pod}" ] && [ -n "${container}" ] || continue
    [[ "${restarts}" =~ ^[0-9]+$ ]] || restarts=0
    present "${waiting}" || waiting=""
    present "${last_reason}" || last_reason=""
    present "${last_exit}" || last_exit=""
    verdict_table+="${pod} ${container} revision=${rev} restarts=${restarts} waiting=${waiting:--} lastTerminated=${last_reason:--}/${last_exit:--} ${last_message}"$'\n'

    # Only the revision being rolled, and never a pod on its way out.
    [ "${rev}" = "${revision}" ] || continue
    present "${deleted}" && continue

    local key="${pod}/${container}"
    local prev="${last_restarts[${key}]:-}"
    if [ -n "${prev}" ] && [ "${restarts}" -gt "${prev}" ]; then
      rises[${key}]=$(( ${rises[${key}]:-0} + 1 ))
    else
      rises[${key}]=0
    fi
    last_restarts[${key}]="${restarts}"

    [ -z "${verdict}" ] || continue
    local why=""
    if [ "${waiting}" = "CrashLoopBackOff" ] && [ "${restarts}" -ge "${MIN_RESTARTS}" ]; then
      why="CrashLoopBackOff after ${restarts} restarts"
    elif [ "${rises[${key}]}" -ge "${RISE_POLLS}" ]; then
      why="restartCount rose on ${rises[${key}]} consecutive polls (now ${restarts}${waiting:+, waiting ${waiting}})"
    fi
    if [ -n "${why}" ]; then
      verdict="pod ${pod} container ${container}: ${why}"
      if [ -n "${last_reason}${last_exit}${last_message}" ]; then
        verdict+="; last terminated ${last_reason:-unknown}${last_exit:+ (exit ${last_exit})}${last_message:+: ${last_message}}"
      fi
    fi
  done <<< "${pods_raw}"
}

echo "rollout-watch: waiting for statefulset/${NAME} in ${NAMESPACE} (timeout ${TIMEOUT}); fast-fail on a crashlooping update-revision pod (CrashLoopBackOff with >=${MIN_RESTARTS} restarts, or restarts rising on ${RISE_POLLS} consecutive ${POLL_SECS}s polls)"

kubectl rollout status "statefulset/${NAME}" -n "${NAMESPACE}" "--timeout=${TIMEOUT}" &
rollout_pid=$!
sleeper=""
cleanup() {
  if [ -n "${sleeper}" ]; then kill "${sleeper}" 2>/dev/null; fi
}
trap cleanup EXIT

while :; do
  sleep "${POLL_SECS}" &
  sleeper=$!
  # Returns on whichever ends first: the rollout (no added latency) or a poll tick.
  wait -n
  if ! kill -0 "${rollout_pid}" 2>/dev/null; then
    wait "${rollout_pid}"
    exit $?
  fi
  probe
  if [ -n "${verdict}" ]; then
    kill "${rollout_pid}" 2>/dev/null
    wait "${rollout_pid}" 2>/dev/null
    artifact_dir="${OUTPUT_ROOT}/${NAMESPACE}--statefulset--${NAME}"
    mkdir -p "${artifact_dir}"
    printf '%s\n' "${verdict}" > "${artifact_dir}/fast-fail.reason"
    {
      printf 'fast_failed_at=%s\nworkload=statefulset/%s\nnamespace=%s\nwaited_secs=%s\ntimeout=%s\n' \
        "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "${NAME}" "${NAMESPACE}" "${SECONDS}" "${TIMEOUT}"
      printf 'reason=%s\n\n# last poll (pod container revision restarts waiting lastTerminated message)\n%s' \
        "${verdict}" "${verdict_table}"
    } > "${artifact_dir}/fast-fail.txt"
    echo "rollout-watch: FAST-FAIL statefulset/${NAME} in ${NAMESPACE} after ${SECONDS}s (not waiting out ${TIMEOUT}): ${verdict}"
    exit "${FAST_FAIL_EXIT}"
  fi
done
