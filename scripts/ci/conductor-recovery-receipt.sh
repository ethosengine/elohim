#!/usr/bin/env bash
# conductor-recovery-receipt.sh — per-pod conductor recovery time after a roll,
# read from rollout evidence: the arc's cycle-time row for a conductor roll.
#
# Runtime contract: bash + coreutils (date) only, and NEVER kubectl. It reads
# an evidence tree in the layout scripts/ci/capture-rollout-evidence.sh writes;
# collecting that tree is the operator's (or the edge pipeline's) act.
#
# Usage:
#   conductor-recovery-receipt.sh [--was <text>] <evidence-root>
#
#   <evidence-root>  the collector's output root (default name rollout-evidence/),
#                    holding <ns>--statefulset--<prefix>-conductor/ and
#                    <ns>--statefulset--<prefix>/ directories.
#   --was <text>     the "Was" cell (default: the pre-fix window, DELTA 09-22).
#
# What it measures, per conductor pod <prefix>-conductor-<n>:
#   restart   = that pod's metadata.creationTimestamp in pods.yaml (a roll
#               recreates the pod, so this is when the conductor restarted);
#   recovered = for each role, the FIRST storage log line
#               "conductor app is RUNNING again" at or after the restart, read
#               from <prefix>-<n>--*.log in the storage workload's directory
#               (the collector's <pod>--<container>--{current,previous}.log, or
#               an operator's Loki export dropped in as <pod>--loki.log); the
#               pod's recovery is its SLOWEST role. That line is emitted only
#               when a zome call on the role succeeded, so it is the recovery
#               proof storage itself accepts (conductor_bridge_health.rs).
#
# Output: one markdown row per conductor pod, in the shape of the cycle-time
# table in genesis/data/timeline/backlog/upgrade-propagation-p2p-design-arc.md
# (| Change class | Was | Now (measured) |). A pod without both halves of the
# evidence prints an UNMEASURED row naming what is missing — never a guess.
#
# Exit: 0 every row measured · 3 any row UNMEASURED (or no conductor evidence)
#       · 64 usage.

set -u

usage() {
  printf 'usage: conductor-recovery-receipt.sh [--was <text>] <evidence-root>\n' >&2
  exit 64
}

WAS='56 min–6 h (pin 25dd2d0be, backlog fleet-standing-celldisabled DELTA 09-22)'
ROOT=''
while [ "$#" -gt 0 ]; do
  case "$1" in
    --was) [ "$#" -ge 2 ] || usage; WAS="$2"; shift 2 ;;
    --was=*) WAS="${1#--was=}"; shift ;;
    -h|--help) usage ;;
    -*) usage ;;
    *) [ -z "${ROOT}" ] || usage; ROOT="$1"; shift ;;
  esac
done
[ -n "${ROOT}" ] && [ -d "${ROOT}" ] || usage
ROOT="${ROOT%/}"

RECOVERY_LINE='conductor app is RUNNING again'
unmeasured=0

to_epoch() { date -u -d "$1" +%s 2>/dev/null; }

human_duration() {
  local secs="$1" h m s
  h=$((secs / 3600)); m=$(((secs % 3600) / 60)); s=$((secs % 60))
  if [ "${h}" -gt 0 ]; then
    printf '%dh%02dm%02ds' "${h}" "${m}" "${s}"
  else
    printf '%dm%02ds' "${m}" "${s}"
  fi
}

row() { # <label> <now-cell>
  printf '| conductor restart → cells RUNNING again · %s | %s | %s |\n' "$1" "${WAS}" "$2"
}

unmeasured_row() { # <label> <reason>
  unmeasured=1
  row "$1" "UNMEASURED — $2"
}

# pods.yaml (a pod List) → "name|creationTimestamp" per pod, from each item's
# top-level metadata only (ownerReferences / labels / container names sit at
# deeper indents and are never read as the pod's name).
conductor_pods() { # <pods.yaml>
  local line in_meta=0 name='' created=''
  local re_name='^    name: "?([^"]+)"?$'
  local re_created='^    creationTimestamp: "?([^"]+)"?$'
  while IFS= read -r line || [ -n "${line}" ]; do
    case "${line}" in
      '- '*)
        [ -n "${name}" ] && printf '%s|%s\n' "${name}" "${created}"
        name=''; created=''; in_meta=0 ;;
      '  metadata:') in_meta=1 ;;
      '  '[!\ ]*) in_meta=0 ;;
      *)
        if [ "${in_meta}" -eq 1 ]; then
          if [[ "${line}" =~ ${re_name} ]]; then
            name="${BASH_REMATCH[1]}"
          elif [[ "${line}" =~ ${re_created} ]]; then
            created="${BASH_REMATCH[1]}"
          fi
        fi ;;
    esac
  done < "$1"
  [ -n "${name}" ] && printf '%s|%s\n' "${name}" "${created}"
  return 0
}

measure_pod() { # <label> <restart-iso> <storage-dir> <storage-pod>
  local label="$1" restart_iso="$2" storage_dir="$3" storage_pod="$4"
  local restart_epoch
  if [ -z "${restart_iso}" ] || ! restart_epoch="$(to_epoch "${restart_iso}")"; then
    unmeasured_row "${label}" "no readable creationTimestamp for the conductor pod in pods.yaml"
    return
  fi

  local logs=() f
  if [ -d "${storage_dir}" ]; then
    for f in "${storage_dir}/${storage_pod}"--*.log; do
      [ -f "${f}" ] && logs+=("${f}")
    done
  fi
  if [ "${#logs[@]}" -eq 0 ]; then
    unmeasured_row "${label}" "no storage log for ${storage_pod} under ${storage_dir##*/}/ (the collector keeps logs only for non-Ready pods, last 200 lines; drop a Loki export in as ${storage_pod}--loki.log)"
    return
  fi

  local -A first_epoch=() first_iso=()
  local line ts role epoch
  local re_json_ts='"timestamp":"([^"]+)"'
  local re_text_ts='^([0-9]{4}-[0-9]{2}-[0-9]{2}T[^[:space:]]+)'
  local re_json_role='"role":"([^"]+)"'
  local re_text_role='role="?([A-Za-z0-9_.-]+)"?'
  for f in "${logs[@]}"; do
    while IFS= read -r line || [ -n "${line}" ]; do
      [[ "${line}" == *"${RECOVERY_LINE}"* ]] || continue
      if [[ "${line}" =~ ${re_json_ts} ]] || [[ "${line}" =~ ${re_text_ts} ]]; then
        ts="${BASH_REMATCH[1]}"
      else
        continue
      fi
      if [[ "${line}" =~ ${re_json_role} ]] || [[ "${line}" =~ ${re_text_role} ]]; then
        role="${BASH_REMATCH[1]}"
      else
        role='(unnamed)'
      fi
      epoch="$(to_epoch "${ts}")" || continue
      [ "${epoch}" -ge "${restart_epoch}" ] || continue
      if [ -z "${first_epoch[${role}]:-}" ] || [ "${epoch}" -lt "${first_epoch[${role}]}" ]; then
        first_epoch["${role}"]="${epoch}"
        first_iso["${role}"]="${ts}"
      fi
    done < "${f}"
  done

  local n="${#first_epoch[@]}"
  if [ "${n}" -eq 0 ]; then
    unmeasured_row "${label}" "no \"${RECOVERY_LINE}\" line at or after restart ${restart_iso} in ${#logs[@]} log file(s) for ${storage_pod}"
    return
  fi

  local slowest='' slowest_epoch=-1 r
  for r in "${!first_epoch[@]}"; do
    if [ "${first_epoch[${r}]}" -gt "${slowest_epoch}" ]; then
      slowest="${r}"; slowest_epoch="${first_epoch[${r}]}"
    fi
  done
  local roles_word='roles'
  [ "${n}" -eq 1 ] && roles_word='role'
  row "${label}" "$(human_duration $((slowest_epoch - restart_epoch))) (restart ${restart_iso} → RUNNING again ${first_iso[${slowest}]}, last role ${slowest}, ${n} ${roles_word})"
}

found=0
for dir in "${ROOT}"/*--statefulset--*-conductor; do
  [ -d "${dir}" ] || continue
  found=1
  base="${dir##*/}"
  ns="${base%%--statefulset--*}"
  sts="${base#*--statefulset--}"
  prefix="${sts%-conductor}"
  storage_dir="${ROOT}/${ns}--statefulset--${prefix}"

  if [ ! -f "${dir}/pods.yaml" ]; then
    unmeasured_row "${prefix}" "no pods.yaml in ${base}/"
    continue
  fi
  pods="$(conductor_pods "${dir}/pods.yaml")"
  if [ -z "${pods}" ]; then
    unmeasured_row "${prefix}" "no conductor pod listed in ${base}/pods.yaml"
    continue
  fi
  while IFS='|' read -r pod created; do
    ordinal="${pod##*-}"
    label="${prefix}"
    [ "${ordinal}" = "0" ] || label="${prefix} (ordinal ${ordinal})"
    measure_pod "${label}" "${created}" "${storage_dir}" "${prefix}-${ordinal}"
  done <<< "${pods}"
done

if [ "${found}" -eq 0 ]; then
  printf 'conductor-recovery-receipt: no <ns>--statefulset--<prefix>-conductor evidence under %s\n' "${ROOT}" >&2
  exit 3
fi
[ "${unmeasured}" -eq 0 ] || exit 3
exit 0
