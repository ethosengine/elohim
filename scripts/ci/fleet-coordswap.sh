#!/usr/bin/env bash
# fleet-coordswap.sh — rolling coordinator-zome hot-swap driver ("rung 1" of
# the upgrade-velocity program, backlog upgrade-propagation-p2p-design-arc).
#
# WHY THIS EXISTS
#   A DNA change whose diff is coordinator-zome-only never moves the DNA
#   hash, so it can be pushed to a running conductor via
#   happ_manager::sync_coordinators's update_coordinators hot-swap — no
#   restart, no re-key, no DHT churn (see root CLAUDE.md "DNA changes don't
#   redeploy by default"). This script is the FLEET-LEVEL vehicle for that:
#   it fans the hot-swap across a list of storage peers ONE AT A TIME
#   (rolling semantics, like `kubectl rollout`), verifying each peer before
#   moving to the next, so a bad bundle or a wedged peer stops the rollout
#   instead of racing every peer at once.
#
# THE SERVER CONTRACT (elohim-storage admin route, implemented in parallel —
# this script codes to the contract, not to any particular server revision):
#
#   POST {peer_base_url}/admin/coordinators/sync?apply=true|false&app_id={app_id}
#     Body: raw .happ bundle bytes, Content-Type: application/octet-stream.
#     apply=false (default) -> dry run: reports coordinator-zome drift
#       between the bundle and what the peer's conductor is running;
#       changes nothing.
#     apply=true -> performs the update_coordinators hot-swap. The server
#       returns 403 if its ALLOW_COORDINATOR_UPDATE env is not enabled on
#       that peer, 503 if no conductor is attached.
#     200 response body (camelCase JSON):
#       {
#         "appId": "elohim",
#         "apply": true,
#         "driftedCount": 1,
#         "appliedCount": 1,
#         "roles": [
#           {
#             "role": "lamad",
#             "drifted": true,
#             "applied": true,
#             "installedCoordinators": {"content_store": "<hash>"},
#             "bundledCoordinators": {"content_store": "<hash>"},
#             "error": null
#           }
#         ]
#       }
#
# USAGE
#   fleet-coordswap.sh --happ <path.happ> --peers <peer-list> [--apply]
#                       [--app-id elohim] [--timeout 120] [--json]
#
#   --peers is a comma-separated list of base URLs. Each entry may be
#   `name=url` (the name is used in output) or a bare URL (a short name is
#   derived from it).
#
# EXAMPLES
#   # Local mesh, status sweep (dry-run every peer, no changes):
#   scripts/ci/fleet-coordswap.sh --happ ./elohim.happ \
#     --peers matthew=http://localhost:8090,adam=http://localhost:8091,eve=http://localhost:8092
#
#   # Local mesh, apply the rollout:
#   scripts/ci/fleet-coordswap.sh --happ ./elohim.happ --apply \
#     --peers matthew=http://localhost:8090,adam=http://localhost:8091,eve=http://localhost:8092
#
#   # In-cluster (Jenkins), service URLs, JSON output for a downstream step:
#   scripts/ci/fleet-coordswap.sh --happ ./elohim.happ --apply --json \
#     --peers matthew=http://elohim-matthew-alpha.elohim-alpha.svc.cluster.local:8090,adam=http://elohim-adam-alpha.elohim-alpha.svc.cluster.local:8090
#
# --apply requires ALLOW_COORDINATOR_UPDATE=true to be set on every target
# storage node ahead of time (see root CLAUDE.md "DNA changes don't redeploy
# by default"). A peer without it returns 403 and STOPS the rollout there.
#
# MODES
#   status sweep (default, no --apply): dry-runs every peer and prints a
#     per-peer drift table. Exit 0 if every peer answered (even if drift
#     exists — drift is information in status mode, not failure). Exit 2 if
#     any peer was unreachable or returned an error.
#   rolling apply (--apply): for each peer IN ORDER —
#     1. dry-run; if driftedCount == 0, mark up-to-date, move on;
#     2. otherwise POST apply=true;
#     3. re-dry-run to confirm driftedCount == 0 (the per-peer verification);
#     4. only then continue to the next peer.
#     The first peer that fails (HTTP/network error, apply error, remaining
#     drift after apply, 403/503) STOPS the rollout immediately. Exit 1,
#     reporting which peers were updated and which were not reached.
#
# EXIT CODES
#   0  status sweep: every peer reachable (drift may still exist)
#      apply:        every peer updated or already current
#   1  apply: rollout stopped partway (a peer failed)
#   2  status sweep: at least one peer unreachable/errored
#   64 usage error (bad/missing args)
#
# Implementation notes: bash + curl + jq only. No heredocs feeding remote
# shells, no eval. curl responses are split into body/http_code via a
# trailing `\n%{http_code}` marker so a non-2xx response is always handled
# as structured error data, never a silent script death.
set -euo pipefail

SCRIPT_NAME="$(basename "$0")"

usage() {
  cat >&2 <<EOF
Usage: ${SCRIPT_NAME} --happ <path.happ> --peers <peer-list> [--apply] [--app-id elohim] [--timeout 120] [--json]

  --happ PATH        path to the .happ bundle to sync (required)
  --peers LIST       comma-separated peer list: name=url or url (required)
  --apply            perform the hot-swap (default: dry-run status sweep only)
  --app-id ID        app_id query param (default: elohim)
  --timeout SECS     per-request curl timeout in seconds (default: 120)
  --json             emit the collected per-peer reports as one JSON array
                      on stdout, suppressing the human-readable table

See the top-of-file comment block for the server contract and examples.
EOF
  exit 64
}

HAPP_PATH=""
PEERS_RAW=""
APPLY=0
APP_ID="elohim"
TIMEOUT=120
JSON_OUT=0

while [ "$#" -gt 0 ]; do
  case "$1" in
    --happ)
      [ "$#" -ge 2 ] || usage
      HAPP_PATH="$2"
      shift 2
      ;;
    --peers)
      [ "$#" -ge 2 ] || usage
      PEERS_RAW="$2"
      shift 2
      ;;
    --apply)
      APPLY=1
      shift
      ;;
    --app-id)
      [ "$#" -ge 2 ] || usage
      APP_ID="$2"
      shift 2
      ;;
    --timeout)
      [ "$#" -ge 2 ] || usage
      TIMEOUT="$2"
      shift 2
      ;;
    --json)
      JSON_OUT=1
      shift
      ;;
    -h|--help)
      usage
      ;;
    *)
      echo "${SCRIPT_NAME}: unknown argument: $1" >&2
      usage
      ;;
  esac
done

[ -n "${HAPP_PATH}" ] || { echo "${SCRIPT_NAME}: --happ is required" >&2; usage; }
[ -n "${PEERS_RAW}" ] || { echo "${SCRIPT_NAME}: --peers is required" >&2; usage; }

command -v curl >/dev/null 2>&1 || { echo "${SCRIPT_NAME}: curl is required" >&2; exit 64; }
command -v jq >/dev/null 2>&1 || { echo "${SCRIPT_NAME}: jq is required" >&2; exit 64; }

[ -f "${HAPP_PATH}" ] || { echo "${SCRIPT_NAME}: happ file not found: ${HAPP_PATH}" >&2; exit 64; }
[ -s "${HAPP_PATH}" ] || { echo "${SCRIPT_NAME}: happ file is empty: ${HAPP_PATH}" >&2; exit 64; }

case "${TIMEOUT}" in
  ''|*[!0-9]*) echo "${SCRIPT_NAME}: --timeout must be a positive integer" >&2; exit 64 ;;
esac

# ---------------------------------------------------------------------------
# Peer list parsing: "name=url,name=url" or bare "url,url" (name derived).
# Populates two parallel arrays: PEER_NAMES, PEER_URLS.
# ---------------------------------------------------------------------------

derive_name() {
  # Strip scheme, strip trailing slash, take the host[:port] portion, and
  # sanitize into a short token safe for table columns.
  local url="$1" stripped
  stripped="${url#*://}"
  stripped="${stripped%%/*}"
  printf '%s' "${stripped}"
}

PEER_NAMES=()
PEER_URLS=()

IFS=',' read -r -a _peer_entries <<< "${PEERS_RAW}"
for entry in "${_peer_entries[@]}"; do
  entry="$(printf '%s' "${entry}" | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//')"
  [ -n "${entry}" ] || continue
  if [[ "${entry}" == *"="* ]]; then
    name="${entry%%=*}"
    url="${entry#*=}"
  else
    name="$(derive_name "${entry}")"
    url="${entry}"
  fi
  url="${url%/}"
  [ -n "${url}" ] || { echo "${SCRIPT_NAME}: empty URL in peer entry: ${entry}" >&2; exit 64; }
  PEER_NAMES+=("${name}")
  PEER_URLS+=("${url}")
done

[ "${#PEER_URLS[@]}" -gt 0 ] || { echo "${SCRIPT_NAME}: --peers produced no entries" >&2; exit 64; }

# ---------------------------------------------------------------------------
# HTTP helper: POST the happ bundle to one peer's sync endpoint.
#
# Writes the parsed HTTP status code to stdout on line 1 and the response
# body (or a synthetic error JSON on connection failure) on the remaining
# lines. Never `exit`s and never relies on `set -e` propagating through a
# failed curl — a network failure is data, not a script fault.
# ---------------------------------------------------------------------------
post_sync() {
  local base_url="$1" apply_flag="$2" raw http_code body curl_status
  local url="${base_url}/admin/coordinators/sync?apply=${apply_flag}&app_id=${APP_ID}"

  # stderr is captured to its own temp file rather than merged with stdout
  # (2>&1): curl's `-w` trailer writes to stdout even on a connection
  # failure (e.g. a bare "000" for an unreached host), and merging streams
  # would let that leak into the human-readable error text below.
  local stderr_file
  stderr_file="$(mktemp)"

  set +e
  raw="$(curl -sS -m "${TIMEOUT}" -w '\n%{http_code}' \
    -X POST \
    -H 'Content-Type: application/octet-stream' \
    --data-binary "@${HAPP_PATH}" \
    "${url}" 2>"${stderr_file}")"
  curl_status=$?
  set -e

  if [ "${curl_status}" -ne 0 ]; then
    local err_text
    err_text="$(tr '\n' ' ' < "${stderr_file}" | sed -e 's/[[:space:]]*$//')"
    rm -f "${stderr_file}"
    [ -n "${err_text}" ] || err_text="curl exit ${curl_status} (no diagnostic output)"
    printf '000\n'
    jq -nc --arg err "${err_text}" '{"error": $err}'
    return 0
  fi
  rm -f "${stderr_file}"

  http_code="${raw##*$'\n'}"
  body="${raw%$'\n'*}"

  if [ -z "${http_code}" ] || [ "${http_code}" = "${raw}" ]; then
    printf '000\n'
    jq -nc --arg err "malformed curl response (no status code split)" '{"error": $err}'
    return 0
  fi

  printf '%s\n' "${http_code}"
  if echo "${body}" | jq -e . >/dev/null 2>&1; then
    printf '%s\n' "${body}"
  else
    jq -nc --arg err "non-JSON response body" --arg body "${body}" '{"error": $err, "rawBody": $body}'
  fi
}

# Runs post_sync and splits its two-part stdout into the globals
# LAST_HTTP_CODE / LAST_BODY.
LAST_HTTP_CODE=""
LAST_BODY=""
call_sync() {
  local base_url="$1" apply_flag="$2" out
  out="$(post_sync "${base_url}" "${apply_flag}")"
  LAST_HTTP_CODE="$(printf '%s' "${out}" | head -n1)"
  LAST_BODY="$(printf '%s' "${out}" | tail -n +2)"
}

drifted_count() { printf '%s' "$1" | jq -r '.driftedCount // "?"' 2>/dev/null || printf '?'; }
applied_count()  { printf '%s' "$1" | jq -r '.appliedCount // "?"' 2>/dev/null || printf '?'; }
err_message()    { printf '%s' "$1" | jq -r '.error // empty' 2>/dev/null || true; }

# ---------------------------------------------------------------------------
# Per-peer report accumulator (for the final table and --json output).
# Each report is one compact JSON object appended to REPORTS[].
# ---------------------------------------------------------------------------
REPORTS=()

add_report() {
  local name="$1" url="$2" verdict="$3" http_code="$4" drifted="$5" applied="$6" note="$7"
  REPORTS+=("$(jq -nc \
    --arg name "${name}" \
    --arg url "${url}" \
    --arg verdict "${verdict}" \
    --arg httpCode "${http_code}" \
    --arg drifted "${drifted}" \
    --arg applied "${applied}" \
    --arg note "${note}" \
    '{peer: $name, url: $url, verdict: $verdict, httpCode: $httpCode, driftedRoles: $drifted, appliedRoles: $applied, note: $note}')")
}

print_human_line() {
  local name="$1" drifted="$2" applied="$3" verdict="$4"
  printf '  %-16s drifted=%-4s applied=%-4s %s\n' "${name}" "${drifted}" "${applied}" "${verdict}"
}

print_final_table() {
  echo ""
  echo "=== fleet-coordswap summary ==="
  printf '  %-16s %-10s %-10s %-8s %s\n' "PEER" "DRIFTED" "APPLIED" "HTTP" "VERDICT"
  for r in "${REPORTS[@]}"; do
    printf '  %-16s %-10s %-10s %-8s %s\n' \
      "$(echo "${r}" | jq -r '.peer')" \
      "$(echo "${r}" | jq -r '.driftedRoles')" \
      "$(echo "${r}" | jq -r '.appliedRoles')" \
      "$(echo "${r}" | jq -r '.httpCode')" \
      "$(echo "${r}" | jq -r '.verdict')"
  done
}

emit_json() {
  local joined
  joined="$(printf '%s\n' "${REPORTS[@]}" | jq -s '.')"
  printf '%s\n' "${joined}"
}

# ---------------------------------------------------------------------------
# Status sweep (default mode): dry-run every peer, no ordering constraints,
# no early exit on drift (drift is information, not failure here).
# ---------------------------------------------------------------------------
run_status_sweep() {
  local any_unreachable=0
  [ "${JSON_OUT}" -eq 1 ] || { echo "=== fleet-coordswap: status sweep (dry-run) ==="; echo "happ: ${HAPP_PATH}  app-id: ${APP_ID}"; echo ""; }

  local i
  for i in "${!PEER_URLS[@]}"; do
    local name="${PEER_NAMES[$i]}" url="${PEER_URLS[$i]}"
    call_sync "${url}" "false"

    if [ "${LAST_HTTP_CODE}" = "200" ]; then
      local drifted applied
      drifted="$(drifted_count "${LAST_BODY}")"
      applied="$(applied_count "${LAST_BODY}")"
      if [ "${drifted}" = "0" ]; then
        add_report "${name}" "${url}" "up-to-date" "${LAST_HTTP_CODE}" "${drifted}" "${applied}" ""
        [ "${JSON_OUT}" -eq 1 ] || print_human_line "${name}" "${drifted}" "${applied}" "up-to-date"
      else
        add_report "${name}" "${url}" "drift-detected" "${LAST_HTTP_CODE}" "${drifted}" "${applied}" ""
        [ "${JSON_OUT}" -eq 1 ] || print_human_line "${name}" "${drifted}" "${applied}" "drift-detected"
      fi
    else
      any_unreachable=1
      local note
      note="$(err_message "${LAST_BODY}")"
      [ -n "${note}" ] || note="HTTP ${LAST_HTTP_CODE}"
      add_report "${name}" "${url}" "error" "${LAST_HTTP_CODE}" "?" "?" "${note}"
      [ "${JSON_OUT}" -eq 1 ] || print_human_line "${name}" "?" "?" "ERROR: ${note}"
    fi
  done

  if [ "${JSON_OUT}" -eq 1 ]; then
    emit_json
  else
    print_final_table
    echo ""
    if [ "${any_unreachable}" -eq 1 ]; then
      echo "STATUS SWEEP: one or more peers unreachable/errored"
    else
      echo "STATUS SWEEP: all peers reachable"
    fi
  fi

  [ "${any_unreachable}" -eq 0 ] || return 2
  return 0
}

# ---------------------------------------------------------------------------
# Rolling apply: peer by peer, sequential, stop on first failure.
# ---------------------------------------------------------------------------
run_rolling_apply() {
  local updated_count=0 current_count=0
  [ "${JSON_OUT}" -eq 1 ] || { echo "=== fleet-coordswap: rolling apply ==="; echo "happ: ${HAPP_PATH}  app-id: ${APP_ID}"; echo ""; }

  local total="${#PEER_URLS[@]}"
  local i
  for i in "${!PEER_URLS[@]}"; do
    local name="${PEER_NAMES[$i]}" url="${PEER_URLS[$i]}"

    # Step 1: dry-run.
    call_sync "${url}" "false"
    if [ "${LAST_HTTP_CODE}" != "200" ]; then
      local note
      note="$(err_message "${LAST_BODY}")"
      [ -n "${note}" ] || note="HTTP ${LAST_HTTP_CODE}"
      add_report "${name}" "${url}" "failed-pre-check" "${LAST_HTTP_CODE}" "?" "?" "${note}"
      [ "${JSON_OUT}" -eq 1 ] || print_human_line "${name}" "?" "?" "FAILED (pre-check): ${note}"
      report_rollout_failure "${updated_count}" "${total}" "${i}"
      return 1
    fi

    local pre_drifted
    pre_drifted="$(drifted_count "${LAST_BODY}")"

    if [ "${pre_drifted}" = "0" ]; then
      current_count=$((current_count + 1))
      add_report "${name}" "${url}" "up-to-date" "${LAST_HTTP_CODE}" "0" "0" ""
      [ "${JSON_OUT}" -eq 1 ] || print_human_line "${name}" "0" "0" "up-to-date"
      continue
    fi

    # Step 2: apply.
    call_sync "${url}" "true"
    if [ "${LAST_HTTP_CODE}" != "200" ]; then
      local note
      note="$(err_message "${LAST_BODY}")"
      if [ -z "${note}" ]; then
        case "${LAST_HTTP_CODE}" in
          403) note="ALLOW_COORDINATOR_UPDATE not enabled on peer" ;;
          503) note="no conductor attached on peer" ;;
          *) note="HTTP ${LAST_HTTP_CODE}" ;;
        esac
      fi
      add_report "${name}" "${url}" "failed-apply" "${LAST_HTTP_CODE}" "${pre_drifted}" "?" "${note}"
      [ "${JSON_OUT}" -eq 1 ] || print_human_line "${name}" "${pre_drifted}" "?" "FAILED (apply): ${note}"
      report_rollout_failure "${updated_count}" "${total}" "${i}"
      return 1
    fi

    local applied
    applied="$(applied_count "${LAST_BODY}")"

    # Step 3: re-dry-run to verify drift is now 0.
    call_sync "${url}" "false"
    if [ "${LAST_HTTP_CODE}" != "200" ]; then
      local note
      note="$(err_message "${LAST_BODY}")"
      [ -n "${note}" ] || note="HTTP ${LAST_HTTP_CODE} on post-apply verification"
      add_report "${name}" "${url}" "failed-verify" "${LAST_HTTP_CODE}" "?" "${applied}" "${note}"
      [ "${JSON_OUT}" -eq 1 ] || print_human_line "${name}" "?" "${applied}" "FAILED (verify): ${note}"
      report_rollout_failure "${updated_count}" "${total}" "${i}"
      return 1
    fi

    local post_drifted
    post_drifted="$(drifted_count "${LAST_BODY}")"
    if [ "${post_drifted}" != "0" ]; then
      add_report "${name}" "${url}" "failed-verify" "${LAST_HTTP_CODE}" "${post_drifted}" "${applied}" "drift remained after apply"
      [ "${JSON_OUT}" -eq 1 ] || print_human_line "${name}" "${post_drifted}" "${applied}" "FAILED (verify): drift remained after apply"
      report_rollout_failure "${updated_count}" "${total}" "${i}"
      return 1
    fi

    updated_count=$((updated_count + 1))
    add_report "${name}" "${url}" "updated" "${LAST_HTTP_CODE}" "0" "${applied}" ""
    [ "${JSON_OUT}" -eq 1 ] || print_human_line "${name}" "0" "${applied}" "updated"
  done

  if [ "${JSON_OUT}" -eq 1 ]; then
    emit_json
  else
    print_final_table
    echo ""
    echo "ROLLOUT COMPLETE: ${updated_count}/${total} peers updated, ${current_count} already current"
  fi
  return 0
}

report_rollout_failure() {
  # updated_count (unused positional kept for call-site clarity) is not
  # relied on here — every peer at index < failed_index was successfully
  # processed (updated OR already up-to-date), since the loop is strictly
  # sequential and stops at the first failure.
  local total="$2" failed_index="$3"
  [ "${JSON_OUT}" -eq 1 ] && { emit_json; return; }

  print_final_table
  echo ""
  echo "ROLLOUT STOPPED: failed at peer $((failed_index + 1))/${total} (${PEER_NAMES[$failed_index]})"
  if [ "${failed_index}" -gt 0 ]; then
    local processed=("${PEER_NAMES[@]:0:${failed_index}}")
    echo "  processed before failure (updated or already current): ${processed[*]}"
  else
    echo "  processed before failure: none"
  fi
  if [ "$((failed_index + 1))" -lt "${total}" ]; then
    local remaining=("${PEER_NAMES[@]:$((failed_index + 1))}")
    echo "  not yet reached: ${remaining[*]}"
  fi
}

if [ "${APPLY}" -eq 1 ]; then
  run_rolling_apply
  exit $?
else
  run_status_sweep
  exit $?
fi
