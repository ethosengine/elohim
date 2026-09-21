#!/bin/bash
# stage-spa-blob.test.sh — regression coverage for the NOT-READY class (three
# faces), the ONE run-level readiness deadline, and the fail-closed blob-forward
# evidence rule (2026-09-21).
#
# run: bash scripts/ci/stage-spa-blob.test.sh
#
# DETERMINISM. Fake `curl`, `node`, `date` and `sleep` on PATH stand in for the
# doorway, the SDK packaging step and the clock. The clock is a FILE: fake `date`
# reads it, fake `sleep` records its argument and ADVANCES it without sleeping,
# and fake `curl` may advance it per call (FAKE_CURL_SECS). Nothing in this file
# reads wall-clock time, so a loaded runner cannot flake it and the whole suite
# runs in well under a second.
#
# The JSON classifier deliberately runs against a REAL node (STAGE_JSON_NODE),
# because the parsing is the thing under test; only the packaging `node` is fake.
#
# KIND=server is used for most cases so the deliverability gate (browser-only) is
# skipped — the leg under test is the serverBlobHash PATCH, which takes the
# identical ladder. Case (j) runs one browser leg against a gate shim to prove
# the gate is still invoked exactly as it was.
set -uo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
# STAGE_SPA_BLOB_SCRIPT lets a red-first run point the same cases at a
# pre-fix copy of the script; unset, the checked-in one is under test.
script="${STAGE_SPA_BLOB_SCRIPT:-${here}/stage-spa-blob.sh}"

# The REAL node, resolved BEFORE the fake one is put on PATH.
REAL_NODE="$(command -v node 2>/dev/null)"
if [ -z "${REAL_NODE}" ]; then
  echo "FATAL: no node on PATH — the JSON classifier under test needs a real one" >&2
  exit 1
fi
export STAGE_JSON_NODE="${REAL_NODE}"

# Fixture setup is fail-closed: an empty or non-directory root would make the
# fixture destinations /bin/curl, /bin/node and /app, which a privileged runner
# would happily overwrite. Nothing is written before these checks pass.
root="$(mktemp -d 2>/dev/null)" || root=""
if [ -z "${root}" ] || [ ! -d "${root}" ]; then
  echo "FATAL: could not create a temporary root (mktemp -d failed) — refusing to write fixtures" >&2
  exit 1
fi
case "${root}" in
  /|/bin|/usr|/etc|/sbin) echo "FATAL: refusing to use '${root}' as a fixture root" >&2; exit 1 ;;
esac
# STAGE_TEST_KEEP=1 leaves the fixture root (and every case's log) behind.
if [ -z "${STAGE_TEST_KEEP:-}" ]; then
  trap 'rm -rf "${root}"' EXIT
else
  echo "fixture root: ${root}"
fi

bin="${root}/bin"
dist="${root}/app/pkg/dist/server"
dist_browser="${root}/app/pkg/dist/browser"
mkdir -p "${bin}" "${dist}" "${dist_browser}" \
  || { echo "FATAL: could not create fixture dirs under ${root}" >&2; exit 1; }
for p in "${bin}/curl" "${bin}/node" "${bin}/sleep" "${bin}/date"; do
  case "${p}" in
    "${root}"/*) : ;;
    *) echo "FATAL: fixture path '${p}' escapes the test root '${root}'" >&2; exit 1 ;;
  esac
done

# --- fake curl -------------------------------------------------------------
# Every leg is driven by a SCRIPTED SEQUENCE: FAKE_PUT_SEQ, FAKE_PROBE_SEQ,
# FAKE_PATCH_SEQ, FAKE_DECLARE_SEQ — newline-separated records of
#     status|body|curl-exit|Retry-After-header
# consumed one per call, the LAST record repeating forever. An empty status means
# "this curl did not honour -w", so nothing is written to stdout for it. The fake
# honours `-o <file>` (and `-o -`), because the script under test now depends on
# it: response bodies must reach the classifier as their ORIGINAL BYTES.
#
# Body tokens, all of which need bytes a shell variable cannot carry:
#   @@BIG@@      >1 MiB catching-up envelope (over the limit in BOTH chars and bytes)
#   @@UTF8BIG@@  600k `é` = 1,200,041 BYTES in 600k CHARACTERS, plus a true forward
#   @@NUL@@      `{"forwarded_to_storage":tr<NUL>ue}` — invalid JSON on the wire
#   @@BADUTF8@@  a valid-looking object carrying bytes that are not UTF-8
cat > "${bin}/curl" <<'FAKE'
#!/bin/bash
method=GET
url=""
headers_out=""
out=""
wfmt=""
data=""
databin="unset"
while [ "$#" -gt 0 ]; do
  case "$1" in
    -X) method="$2"; shift 2 ;;
    -D) headers_out="$2"; shift 2 ;;
    -d) data="$2"; shift 2 ;;
    -o) out="$2"; shift 2 ;;
    -w) wfmt="$2"; shift 2 ;;
    --data-binary) databin="$2"; shift 2 ;;
    -H|--max-time) shift 2 ;;
    -*) shift ;;
    *) url="$1"; shift ;;
  esac
done

# The ONLY wall-clock delay in this harness, used by the one real-time case (x):
# a signal cannot be delivered on a fake clock.
[ -n "${FAKE_CURL_REAL_SLEEP:-}" ] && /bin/sleep "${FAKE_CURL_REAL_SLEEP}"

clock_now() { local n=0; [ -f "${FAKE_CLOCK}" ] && n="$(cat "${FAKE_CLOCK}")"; printf '%s' "${n}"; }
advance() {
  local by="${FAKE_CURL_SECS:-0}"
  [ "${by}" -gt 0 ] 2>/dev/null || return 0
  printf '%s' "$(( $(clock_now) + by ))" > "${FAKE_CLOCK}"
}
bump() {
  local f="${FAKE_STATE}/$1" n=1
  [ -f "${f}" ] && n=$(( $(cat "${f}") + 1 ))
  printf '%s' "${n}" > "${f}"
  printf '%s' "${n}"
}
pick() {
  local seq="$1" want="$2" n=0 line last=""
  while IFS= read -r line; do
    [ -n "${line}" ] || continue
    n=$(( n + 1 ))
    last="${line}"
    if [ "${n}" -eq "${want}" ]; then printf '%s' "${line}"; return 0; fi
  done <<< "${seq}"
  printf '%s' "${last}"
}
emit() {
  local rec="$1" st body rc ra tmp
  IFS='|' read -r st body rc ra <<< "${rec}"
  tmp="$(mktemp)"
  case "${body}" in
    @@BIG@@)
      { printf '{"status":"catching-up","pad":"'
        head -c 1100000 /dev/zero | tr '\0' 'x'
        printf '"}'; } > "${tmp}" ;;
    @@UTF8BIG@@)
      { printf '{"forwarded_to_storage":true,"pad":"'
        yes 'é' | head -n 600000 | tr -d '\n'
        printf '"}'; } > "${tmp}" ;;
    @@LONGCELL@@)
      { printf '{"error":"Conductor returned an error while using a ConductorApi: CellDisabled(CellId(uhC0kFAKE))"}'
        printf '\n'
        head -c 300000 /dev/zero | tr '\0' 'z'; } > "${tmp}" ;;
    @@NUL@@)
      { printf '{"forwarded_to_storage":tr'; printf '\000'; printf 'ue}'; } > "${tmp}" ;;
    @@BADUTF8@@)
      { printf '{"forwarded_to_storage":true,"x":"'; printf '\377\376'; printf '"}'; } > "${tmp}" ;;
    *)
      printf '%s' "${body}" > "${tmp}" ;;
  esac
  if [ -n "${ra}" ] && [ -n "${headers_out}" ]; then
    printf 'HTTP/1.1 %s\r\nRetry-After: %s\r\n\r\n' "${st:-503}" "${ra}" > "${headers_out}"
  fi
  if [ -n "${out}" ] && [ "${out}" != "-" ]; then cat "${tmp}" > "${out}"; else cat "${tmp}"; fi
  rm -f "${tmp}"
  # An empty scripted status models a curl that did not honour -w.
  if [ -n "${wfmt}" ] && [ -n "${st}" ]; then
    printf '%b' "$(printf '%s' "${wfmt}" | sed "s/%{http_code}/${st}/")"
  fi
  exit "${rc:-0}"
}

advance

# Defaults live in variables, never inline in ${VAR:-...}: a `}` inside the
# default text would close the expansion and silently truncate the body.
DEF_PUT='200|{"success":true,"hash":"x","already_cached":false,"forwarded_to_storage":true,"size":13}|0|'
DEF_PROBE='200|{"success":true,"hash":"x","already_cached":true,"forwarded_to_storage":true,"size":13}|0|'
DEF_PATCH='200|{"dhtAnchorHash":"uhCkkFAKEANCHOR"}|0|'
DEF_DECLARE='200|{"ok":true}|0|'

case "${url}" in
  */admin/seed/blob)
    if [ -z "${databin}" ]; then
      printf 'EMPTY-BODY-PUT\n' >> "${FAKE_STATE}/probes"
      emit "$(pick "${FAKE_PROBE_SEQ:-$DEF_PROBE}" "$(bump probe_calls)")"
    fi
    printf '%s\n' "$(clock_now)" >> "${FAKE_STATE}/put_times"
    emit "$(pick "${FAKE_PUT_SEQ:-$DEF_PUT}" "$(bump put_calls)")"
    ;;
  */head-record)
    exit 22
    ;;
  */head)
    [ "${FAKE_HEAD_MODE:-missing}" = "resolvable" ] || exit 22
    printf '{"headActionHash":"uhCkkFAKEHEAD"}'
    exit 0
    ;;
  */canonical-head)
    emit "$(pick "${FAKE_DECLARE_SEQ:-$DEF_DECLARE}" "$(bump declare_calls)")"
    ;;
esac

if [ "${method}" = "PATCH" ]; then
  printf '%s\n' "$(clock_now)" >> "${FAKE_STATE}/patch_times"
  printf '%s' "${data}" | sed -n 's/.*:"\([^"]*\)".*/\1/p' > "${FAKE_STATE}/hash"
  emit "$(pick "${FAKE_PATCH_SEQ:-$DEF_PATCH}" "$(bump patch_calls)")"
fi

# Read-back of the row the PATCH just wrote.
case "${url}" in
  */db/content/*)
    printf '{"serverBlobHash":"%s","blobHash":"%s"}' \
      "$(cat "${FAKE_STATE}/hash" 2>/dev/null)" "$(cat "${FAKE_STATE}/hash" 2>/dev/null)"
    exit 0
    ;;
esac
exit 22
FAKE

# --- fake node (SDK packaging only) ----------------------------------------
cat > "${bin}/node" <<'FAKE'
#!/bin/bash
out=""
kind="browser"
while [ "$#" -gt 0 ]; do
  case "$1" in
    --out) out="$2"; shift 2 ;;
    --kind) kind="$2"; shift 2 ;;
    *) shift ;;
  esac
done
mkdir -p "${out}"
printf 'fake-package' > "${out}/${kind}.zip"
exit 0
FAKE

# --- fake date (reads the clock file) --------------------------------------
cat > "${bin}/date" <<'FAKE'
#!/bin/bash
now=0
[ -f "${FAKE_CLOCK}" ] && now="$(cat "${FAKE_CLOCK}")"
want=""
prev=""
for a in "$@"; do
  [ "${prev}" = "-d" ] && want="${a}"
  prev="${a}"
done
if [ -n "${want}" ]; then printf 'T+%s\n' "${want#@}"; exit 0; fi
printf '%s\n' "${now}"
FAKE

# --- fake sleep (records, then ADVANCES the clock; never really sleeps) -----
cat > "${bin}/sleep" <<'FAKE'
#!/bin/bash
n="${1:-0}"
printf '%s\n' "${n}" >> "${FAKE_STATE}/sleeps" 2>/dev/null
now=0
[ -f "${FAKE_CLOCK}" ] && now="$(cat "${FAKE_CLOCK}")"
if [ "${n}" -ge 0 ] 2>/dev/null; then printf '%s' "$(( now + n ))" > "${FAKE_CLOCK}"; fi
exit 0
FAKE

chmod +x "${bin}/curl" "${bin}/node" "${bin}/sleep" "${bin}/date" \
  || { echo "FATAL: could not make the fixtures executable" >&2; exit 1; }
export PATH="${bin}:${PATH}"

fail=0
checks=0
check() {
  checks=$(( checks + 1 ))
  if eval "$2"; then
    echo "ok   $1"
  else
    echo "FAIL $1"
    fail=1
  fi
}

naps() { cat "${1}/sleeps" 2>/dev/null; }
no_naps() { [ ! -s "${1}/sleeps" ]; }
nap_count() { naps "$1" | grep -c . ; }
longest_nap() { naps "$1" | sort -n | tail -1; }
call_count() { cat "${1}/$2" 2>/dev/null || printf '0'; }
clock_base=1000000

# Start a fresh case: its own FAKE_STATE, its own clock, its own log.
new_case() {
  state="${root}/$1"; log="${root}/$1.log"
  mkdir -p "${state}"
  printf '%s' "${clock_base}" > "${state}/clock"
}

# Run one stage. Echoes the exit status; the log lands in $log.
run_stage() {
  local url="$1" kind="${2:-server}" d="${dist}"
  shift 2
  [ "${kind}" = "browser" ] && d="${dist_browser}"
  FAKE_STATE="${state}" \
  FAKE_CLOCK="${state}/clock" \
  DO_PATCH="${DO_PATCH_OVERRIDE:-1}" \
  STORAGE_API_KEY_ADMIN=test-key \
  STAGE_CELL_READY_STATE_DIR="${cell_state}" \
  STAGE_CELL_READY_POLL_SECS=10 \
  STAGE_HARD_TIMEOUT_SECS="${STAGE_HARD_TIMEOUT_OVERRIDE:-0}" \
  env "$@" bash "${stage_script:-${script}}" "${d}" "test-slug" "${url}" "${kind}" \
    > "${log}" 2>&1
  printf '%s' "$?"
}

OK_PUT='200|{"success":true,"hash":"x","already_cached":false,"forwarded_to_storage":true,"size":13}|0|'
OK_PATCH='200|{"dhtAnchorHash":"uhCkkFAKEANCHOR"}|0|'
CELL_BODY='{"error":"Conductor returned an error while using a ConductorApi: CellDisabled(CellId(uhC0kFAKE))"}'
SHED_CATCHUP='503|{"status":"catching-up","retryAfter":30,"cause":"upstream","circuit":"closed","errorStreak":1}|0|'
FWD_TIMEOUT='200|{"success":true,"hash":"x","already_cached":false,"forwarded_to_storage":false,"size":13,"error":"could not reach storage to forward the blob: error sending request for url (http://elohim-matthew-alpha:8090/blob/sha256-abc): operation timed out"}|0|'

# --- (a) catching-up x3 then 200 on the PATCH leg --------------------------
# STAGE_BLOB_ATTEMPTS=1 and a 2s transport budget are the proof that the
# readiness wait charges neither: three polls would blow both if it did.
new_case a; cell_state="${root}/a-cells"
status="$(run_stage "http://doorway-a" server \
  FAKE_PATCH_SEQ="${SHED_CATCHUP}
${SHED_CATCHUP}
${SHED_CATCHUP}
${OK_PATCH}" \
  STAGE_CELL_READY_BUDGET_SECS=300 STAGE_BLOB_ATTEMPTS=1 STAGE_BLOB_BUDGET_SECS=2)"
check "(a) catching-up x3 then 200 exits 0" "[ '${status}' = '0' ]"
check "(a) it is classified catching-up, not a transport shed" \
  "grep -q 'face=catching-up' '${log}'"
check "(a) the transport attempt counter is not consumed" "! grep -q 'stage failed after' '${log}'"
check "(a) exactly three readiness naps" "[ \"\$(nap_count '${state}')\" = '3' ]"
check "(a) four PATCHes: the three shed ones and the one that landed" \
  "[ \"\$(call_count '${state}' patch_calls)\" = '4' ]"

# --- (b) catching-up on the PUT leg ---------------------------------------
# curl -f used to discard this body entirely, so the leg spent its transport
# budget on a readiness condition (#1715).
new_case b; cell_state="${root}/b-cells"
status="$(run_stage "http://doorway-b" server \
  FAKE_PUT_SEQ="${SHED_CATCHUP}
${SHED_CATCHUP}
${OK_PUT}" \
  STAGE_CELL_READY_BUDGET_SECS=300 STAGE_BLOB_ATTEMPTS=1 STAGE_BLOB_BUDGET_SECS=2)"
check "(b) a catching-up shed on the PUT leg waits and then succeeds" \
  "[ '${status}' = '0' ] && grep -q 'face=catching-up' '${log}'"
check "(b) it did not spend the transport ladder" "! grep -q 'stage failed after' '${log}'"

# --- (c) the storage-forward-timeout face, and its two near-misses ---------
new_case c1; cell_state="${root}/c1-cells"
status="$(run_stage "http://doorway-c1" server \
  FAKE_PUT_SEQ="${FWD_TIMEOUT}
${FWD_TIMEOUT}
${OK_PUT}" \
  STAGE_CELL_READY_BUDGET_SECS=300 STAGE_BLOB_ATTEMPTS=1 STAGE_BLOB_BUDGET_SECS=2)"
check "(c) a forward that timed out x2 then confirmed exits 0" \
  "[ '${status}' = '0' ] && grep -q 'face=storage-forward-timeout' '${log}'"
check "(c) it is described as availability-unknown, never as storage being up" \
  "grep -q 'availability unknown' '${log}' && ! grep -qi 'storage is up' '${log}'"

new_case c2; cell_state="${root}/c2-cells"
status="$(run_stage "http://doorway-c2" server \
  FAKE_PUT_SEQ='200|{"success":true,"hash":"x","already_cached":false,"forwarded_to_storage":false,"size":13,"error":"could not reach storage to forward the blob: error sending request for url (http://elohim-matthew-alpha:8090/blob/sha256-abc): tcp connect error: Connection refused (os error 111)"}|0|' \
  STAGE_CELL_READY_BUDGET_SECS=300 STAGE_BLOB_ATTEMPTS=2 STAGE_BLOB_BUDGET_SECS=30)"
check "(c) connection-refused rides the transport ladder, never readiness" \
  "[ '${status}' = '1' ] && ! grep -q 'face=' '${log}' && grep -q 'stage failed after 2 attempt' '${log}'"

new_case c3; cell_state="${root}/c3-cells"
status="$(run_stage "http://doorway-c3" server \
  FAKE_PUT_SEQ='200|{"success":true,"hash":"x","already_cached":false,"forwarded_to_storage":false,"size":13,"error":"could not reach storage to forward the blob: error sending request for url (http://timeout-storage:8090/blob/sha256-abc): dns error: failed to lookup address information: Name or service not known"}|0|' \
  STAGE_CELL_READY_BUDGET_SECS=300 STAGE_BLOB_ATTEMPTS=2 STAGE_BLOB_BUDGET_SECS=30)"
check "(c) a DNS failure against a host NAMED timeout-storage is not the timeout face" \
  "[ '${status}' = '1' ] && ! grep -q 'face=' '${log}'"

# --- (d) two faces, ONE deadline ------------------------------------------
new_case d; cell_state="${root}/d-cells"
status="$(run_stage "http://doorway-d" server \
  FAKE_PATCH_SEQ="503|${CELL_BODY}|0|
503|${CELL_BODY}|0|
503|{\"status\":\"catching-up\"}|0|
503|{\"status\":\"catching-up\"}|0|
${OK_PATCH}" \
  STAGE_CELL_READY_BUDGET_SECS=100 STAGE_BLOB_ATTEMPTS=1 STAGE_BLOB_BUDGET_SECS=2)"
check "(d) a run that meets both faces still exits 0" "[ '${status}' = '0' ]"
check "(d) both faces are named" \
  "grep -q 'face=cell-not-running' '${log}' && grep -q 'face=catching-up' '${log}'"
check "(d) the persisted stamp is the run's FIRST not-ready answer, unchanged" \
  "grep -qx 'first_not_ready=${clock_base}' '${cell_state}/run-deadline'"
check "(d) the logged remaining is strictly decreasing across the face change" \
  "[ \"\$(grep -o '[0-9]*s left of' '${log}' | tr -dc '0-9\n' | tr '\n' ' ')\" = '100 90 80 70 ' ]"

# --- (e) a recovery does NOT replenish the deadline ------------------------
new_case e; cell_state="${root}/e-cells"
status="$(run_stage "http://doorway-e" server \
  FAKE_PATCH_SEQ="503|${CELL_BODY}|0|
503|${CELL_BODY}|0|
${OK_PATCH}" \
  STAGE_CELL_READY_BUDGET_SECS=100 STAGE_BLOB_BUDGET_SECS=30)"
check "(e) the recovering leg exits 0" "[ '${status}' = '0' ]"
check "(e) recovery KEEPS the run deadline — the record is not removed" \
  "[ -f '${cell_state}/run-deadline' ] && grep -qx 'first_not_ready=${clock_base}' '${cell_state}/run-deadline'"
check "(e) and it says how much of the run's deadline is left" \
  "grep -q \"of this run's readiness deadline left\" '${log}'"
# A SECOND window later in the same run inherits what is left (20s were spent).
state2="${root}/e2"; log2="${root}/e2.log"; mkdir -p "${state2}"
cp "${state}/clock" "${state2}/clock"
FAKE_STATE="${state2}" FAKE_CLOCK="${state2}/clock" DO_PATCH=1 \
STORAGE_API_KEY_ADMIN=test-key STAGE_CELL_READY_STATE_DIR="${cell_state}" \
STAGE_HARD_TIMEOUT_SECS=0 \
STAGE_CELL_READY_POLL_SECS=10 \
FAKE_PATCH_SEQ="503|${CELL_BODY}|0|" \
STAGE_CELL_READY_BUDGET_SECS=100 STAGE_BLOB_BUDGET_SECS=30 \
  bash "${script}" "${dist}" "test-slug" "http://doorway-e" server > "${log2}" 2>&1
status=$?
check "(e) a second window in the same run inherits the deadline (80s, not 100s)" \
  "[ '${status}' = '1' ] && grep -q '20s waited, 80s left of 100s' '${log2}'"
check "(e) and the whole run still stops at the one deadline" \
  "grep -q 'still not ready after 100s' '${log2}'"
# A different BUILD_TAG is a different run and gets its own deadline.
ws="${root}/e-ws"; mkdir -p "${ws}"
for tag in build-1 build-2; do
  st="${root}/e-${tag}"; mkdir -p "${st}"
  printf '%s' "${clock_base}" > "${st}/clock"
  FAKE_STATE="${st}" FAKE_CLOCK="${st}/clock" DO_PATCH=1 \
  STORAGE_API_KEY_ADMIN=test-key STAGE_CELL_READY_POLL_SECS=10 STAGE_HARD_TIMEOUT_SECS=0 \
  WORKSPACE="${ws}" BUILD_TAG="${tag}" \
  FAKE_PATCH_SEQ="503|${CELL_BODY}|0|" STAGE_CELL_READY_BUDGET_SECS=10 \
    bash "${script}" "${dist}" "test-slug" "http://doorway-e2" server \
    > "${root}/e-${tag}.log" 2>&1
done
check "(e) each BUILD_TAG files its own run-deadline" \
  "[ -f '${ws}/.stage-cell-ready/build-1/run-deadline' ] && [ -f '${ws}/.stage-cell-ready/build-2/run-deadline' ]"
check "(e) the second build does not inherit the first build's spent deadline" \
  "grep -q '0s waited, 10s left of 10s' '${root}/e-build-2.log'"

# --- (f) forever not-ready: the deadline governs when a RE-OFFER may start --
# budget 30, poll 10 -> PATCHes at +0, +10, +20 and NOTHING at or after +30.
new_case f; cell_state="${root}/f-cells"
status="$(run_stage "http://doorway-f" server \
  FAKE_PATCH_SEQ="503|${CELL_BODY}|0|" \
  STAGE_CELL_READY_BUDGET_SECS=30 STAGE_CELL_READY_POLL_SECS_UNUSED=1 \
  STAGE_BLOB_BUDGET_SECS=600)"
check "(f) a deadline that runs out exits 1" "[ '${status}' = '1' ]"
check "(f) exactly three PATCHes were dispatched" \
  "[ \"\$(call_count '${state}' patch_calls)\" = '3' ]"
check "(f) none of them started at or after expiry" \
  "[ \"\$(sort -n '${state}/patch_times' | tail -1)\" -lt \"\$(( clock_base + 30 ))\" ]"
check "(f) the exhaustion line is a measurement with the last face" \
  "grep -q 'readiness deadline reached; the holder was still not ready after 30s (last face=cell-not-running' '${log}'"
check "(f) it admits an attempt in flight may outlive the deadline" \
  "grep -q 'an attempt in flight may have finished past it' '${log}'"
check "(f) it points at the doorway and the storage peer, not at a cause" \
  "grep -q \"read the doorway's /health/serving and the storage peer's state\" '${log}'"
# WHICH gate stopped it is observable, and both gates matter: the nap here ends
# exactly at expiry, so the post-nap check inside cell_ready_wait is what should
# refuse — the loop-top gate is the backstop for the ORDINARY-sleep path and
# prints a different line. Without this the post-nap check is untested, because
# the backstop silently covers for it.
check "(f) the post-nap check refuses, not the loop-top backstop" \
  "grep -q 'never became ready (see above)' '${log}' \
   && ! grep -q 'before another attempt could be offered' '${log}'"
# A SECOND invocation of the SAME run fails at once, with no nap at all.
state2="${root}/f2"; log2="${root}/f2.log"; mkdir -p "${state2}"
cp "${state}/clock" "${state2}/clock"
FAKE_STATE="${state2}" FAKE_CLOCK="${state2}/clock" DO_PATCH=1 \
STORAGE_API_KEY_ADMIN=test-key STAGE_CELL_READY_STATE_DIR="${cell_state}" \
STAGE_HARD_TIMEOUT_SECS=0 \
FAKE_PATCH_SEQ="503|${CELL_BODY}|0|" STAGE_CELL_READY_BUDGET_SECS=30 \
  bash "${script}" "${dist}" "test-slug" "http://doorway-f" server > "${log2}" 2>&1
status=$?
check "(f) a later leg of the same run fails at once, with no nap" \
  "[ '${status}' = '1' ] && no_naps '${state2}' && grep -q 'readiness deadline reached' '${log2}'"

# --- (g) advertised retryAfter, normalised ONCE ----------------------------
new_case g1; cell_state="${root}/g1-cells"
status="$(run_stage "http://doorway-g1" server \
  FAKE_PATCH_SEQ='503|{"status":"catching-up","retryAfter":0}|0|
503|{"status":"catching-up","retryAfter":0}|0|
200|{"dhtAnchorHash":"uhCkkFAKEANCHOR"}|0|' \
  STAGE_CELL_READY_BUDGET_SECS=300 STAGE_CELL_READY_POLL_SECS=60 \
  STAGE_BLOB_BUDGET_SECS=30)"
check "(g) retryAfter=0 naps the 5s floor, not zero and not the full poll" \
  "[ '${status}' = '0' ] && [ \"\$(naps '${state}' | sort -u | tr '\n' ' ')\" = '5 ' ]"

new_case g2; cell_state="${root}/g2-cells"
status="$(run_stage "http://doorway-g2" server \
  FAKE_PATCH_SEQ="503|{\"status\":\"catching-up\"}|0|999999999999999999999999999999999999999" \
  STAGE_CELL_READY_BUDGET_SECS=30 STAGE_CELL_READY_POLL_SECS=10 \
  STAGE_BLOB_BUDGET_SECS=600)"
check "(g) a 39-digit Retry-After is ignored by the READINESS nap (poll wins)" \
  "[ '${status}' = '1' ] && [ \"\$(longest_nap '${state}')\" = '10' ]"

new_case g3; cell_state="${root}/g3-cells"
status="$(run_stage "http://doorway-g3" server \
  FAKE_PATCH_SEQ="503|{\"error\":\"shedding\"}|0|999999999999999999999999999999999999999" \
  STAGE_CELL_READY_BUDGET_SECS=300 STAGE_BLOB_ATTEMPTS=2 STAGE_BLOB_BUDGET_SECS=600)"
check "(g) a 39-digit Retry-After is ignored by the TRANSPORT sleep too" \
  "[ '${status}' = '1' ] && [ \"\$(naps '${state}' | tr '\n' ' ')\" = '5 ' ]"

new_case g4; cell_state="${root}/g4-cells"
status="$(run_stage "http://doorway-g4" server \
  FAKE_PATCH_SEQ="503|${CELL_BODY}|0|" \
  STAGE_CELL_READY_BUDGET_SECS=3 STAGE_CELL_READY_POLL_SECS=10 \
  STAGE_BLOB_BUDGET_SECS=600)"
check "(g) with 3s left the nap is 3s — the floor never overruns the deadline" \
  "[ '${status}' = '1' ] && [ \"\$(naps '${state}' | tr '\n' ' ')\" = '3 ' ]"

# --- (h) JSON negatives: none of these may classify or confirm -------------
h_case() {
  local name="$1" body="$2"
  new_case "${name}"; cell_state="${root}/${name}-cells"
  status="$(run_stage "http://doorway-${name}" server \
    FAKE_PATCH_SEQ="503|${body}|0|" \
    STAGE_CELL_READY_BUDGET_SECS=300 STAGE_BLOB_ATTEMPTS=1 STAGE_BLOB_BUDGET_SECS=600)"
  check "(h) ${name} does not classify as not-ready" \
    "[ '${status}' = '1' ] && ! grep -q 'face=' '${log}'"
}
h_case h-nested '{"error":{"status":"catching-up"}}'
h_case h-array '[{"status":"catching-up"}]'
h_case h-nonstring '{"status":123}'
h_case h-missing '{"error":"something else"}'
h_case h-invalid 'catching-up'
h_case h-oversize '@@BIG@@'

new_case h-fwd-string; cell_state="${root}/h-fwd-string-cells"
status="$(run_stage "http://doorway-h-fwd-string" server \
  FAKE_PUT_SEQ='200|{"forwarded_to_storage":"true"}|0|' \
  STAGE_CELL_READY_BUDGET_SECS=300 STAGE_BLOB_ATTEMPTS=1 STAGE_BLOB_BUDGET_SECS=600)"
check "(h) the STRING \"true\" never confirms a storage forward" \
  "[ '${status}' = '1' ] && grep -q 'did NOT confirm the storage forward' '${log}'"

new_case h-fwd-spaced; cell_state="${root}/h-fwd-spaced-cells"
status="$(run_stage "http://doorway-h-fwd-spaced" server \
  FAKE_PUT_SEQ='200|{"forwarded_to_storage": false, "error":"could not reach storage to forward the blob: error sending request for url (http://s:8090/b): tcp connect error: Connection refused (os error 111)"}|0|' \
  STAGE_CELL_READY_BUDGET_SECS=300 STAGE_BLOB_ATTEMPTS=1 STAGE_BLOB_BUDGET_SECS=600)"
check "(h) a SPACED forwarded_to_storage:false with connection-refused never exits 0" \
  "[ '${status}' != '0' ] && ! grep -q 'face=' '${log}'"

new_case h-fwd-missing; cell_state="${root}/h-fwd-missing-cells"
status="$(run_stage "http://doorway-h-fwd-missing" server \
  FAKE_PUT_SEQ='200|{}|0|' \
  STAGE_CELL_READY_BUDGET_SECS=300 STAGE_BLOB_ATTEMPTS=1 STAGE_BLOB_BUDGET_SECS=600)"
check "(h) a missing forwarded_to_storage is a contract violation, not a success" \
  "[ '${status}' = '1' ] && grep -q 'did NOT confirm the storage forward' '${log}'"

# --- (i) curl 55/56: affirmative storage evidence only ---------------------
new_case i1; cell_state="${root}/i1-cells"
status="$(run_stage "http://doorway-i1" server \
  FAKE_PUT_SEQ='200|{"success":true,"hash":"x","already_cached":true,"forwarded_to_storage":true,"size":13}|55|' \
  STAGE_CELL_READY_BUDGET_SECS=300 STAGE_BLOB_ATTEMPTS=1 STAGE_BLOB_BUDGET_SECS=600)"
check "(i) a broken PUT whose captured body confirms the forward is done" \
  "[ '${status}' = '0' ] && [ ! -f '${state}/probes' ]"

new_case i2; cell_state="${root}/i2-cells"
status="$(run_stage "http://doorway-i2" server \
  FAKE_PUT_SEQ='|not-json|56|' \
  FAKE_PROBE_SEQ='200|{"success":true,"hash":"x","already_cached":true,"forwarded_to_storage":true,"size":13}|0|' \
  STAGE_CELL_READY_BUDGET_SECS=300 STAGE_BLOB_ATTEMPTS=1 STAGE_BLOB_BUDGET_SECS=600)"
check "(i) an unparsable broken PUT sends a ZERO-BYTE confirmation probe" \
  "grep -qx 'EMPTY-BODY-PUT' '${state}/probes'"
check "(i) and the probe's answer decides it" "[ '${status}' = '0' ]"

new_case i3; cell_state="${root}/i3-cells"
status="$(run_stage "http://doorway-i3" server \
  FAKE_PUT_SEQ='|not-json|56|' \
  FAKE_PROBE_SEQ='200|{"success":true,"hash":"x","already_cached":true,"forwarded_to_storage":false,"size":13,"error":"storage did not answer"}|0|' \
  STAGE_CELL_READY_BUDGET_SECS=300 STAGE_BLOB_ATTEMPTS=1 STAGE_BLOB_BUDGET_SECS=600)"
check "(i) cache-only evidence never exits 0" \
  "[ '${status}' != '0' ] && grep -q 'a doorway cache hit is not that proof' '${log}'"

# --- (j) with NO window open, behaviour equals HEAD's ----------------------
new_case j1; cell_state="${root}/j1-cells"
status="$(run_stage "http://doorway-j1" server \
  STAGE_CELL_READY_BUDGET_SECS=300 STAGE_BLOB_BUDGET_SECS=30)"
check "(j) an ordinary success exits 0 with no readiness machinery at all" \
  "[ '${status}' = '0' ] && no_naps '${state}' && ! grep -q 'face=' '${log}' \
   && ! grep -q 'not-ready window' '${log}' && [ ! -f '${cell_state}/run-deadline' ]"
check "(j) it verifies the row it just wrote" "grep -q 'verified test-slug' '${log}'"

new_case j2; cell_state="${root}/j2-cells"
status="$(run_stage "http://doorway-j2" server \
  FAKE_PATCH_SEQ='503|{"error":"shedding"}|0|1' \
  STAGE_CELL_READY_BUDGET_SECS=300 STAGE_BLOB_ATTEMPTS=2 STAGE_BLOB_BUDGET_SECS=30)"
check "(j) an ordinary shed still spends the TRANSPORT budget" \
  "[ '${status}' = '1' ] && grep -q 'stage failed after 2 attempt(s)' '${log}'"
check "(j) and still honours its advertised Retry-After" \
  "grep -q 'retryAfter=1s' '${log}' && [ \"\$(naps '${state}' | tr '\n' ' ')\" = '1 ' ]"

new_case j3; cell_state="${root}/j3-cells"
status="$(run_stage "http://doorway-j3" server \
  FAKE_PATCH_SEQ='409|{"error":"content row conflict"}|0|' \
  STAGE_CELL_READY_BUDGET_SECS=300 STAGE_BLOB_ATTEMPTS=5 STAGE_BLOB_BUDGET_SECS=30)"
check "(j) a structural 4xx is still immediate" \
  "[ '${status}' = '1' ] && no_naps '${state}' && grep -q 'failed structurally (HTTP 409)' '${log}'"

# The deliverability gate must still be invoked exactly as it was: once per
# attempt, with its two arguments and no wrapper of any kind.
shim="${root}/shim"; mkdir -p "${shim}"
cp "${script}" "${shim}/stage-spa-blob.sh"
cat > "${shim}/deliverability-gate.sh" <<'GATE'
#!/bin/bash
printf '%s\n' "$*" >> "${FAKE_STATE}/gate-calls"
exit 0
GATE
chmod +x "${shim}/deliverability-gate.sh"
new_case j4; cell_state="${root}/j4-cells"
stage_script="${shim}/stage-spa-blob.sh"
status="$(run_stage "http://doorway-j4" browser \
  STAGE_CELL_READY_BUDGET_SECS=300 STAGE_BLOB_BUDGET_SECS=30)"
stage_script=""
check "(j) a browser leg still invokes the deliverability gate, once, unwrapped" \
  "[ '${status}' = '0' ] && [ \"\$(grep -c . '${state}/gate-calls')\" = '1' ] \
   && grep -qx 'http://doorway-j4 sha256-.*' '${state}/gate-calls'"

# --- (k) an unusable deadline record degrades to THIS invocation -----------
# The state dir is a regular FILE, so mkdir -p cannot create it (root included).
new_case k1
cell_state="${root}/k1-not-a-dir"; printf 'x' > "${cell_state}"
status="$(run_stage "http://doorway-k1" server \
  FAKE_PATCH_SEQ="503|${CELL_BODY}|0|" \
  STAGE_CELL_READY_BUDGET_SECS=30 STAGE_BLOB_BUDGET_SECS=600)"
check "(k) an unwritable state dir still enforces the budget in this invocation" \
  "[ '${status}' = '1' ] && grep -q 'still not ready after 30s' '${log}'"
check "(k) and says so exactly once" \
  "[ \"\$(grep -c '⊘ WARN' '${log}')\" = '1' ]"

new_case k2; cell_state="${root}/k2-cells"; mkdir -p "${cell_state}"
printf 'this is not a stamp\n' > "${cell_state}/run-deadline"
status="$(run_stage "http://doorway-k2" server \
  FAKE_PATCH_SEQ="503|${CELL_BODY}|0|" \
  STAGE_CELL_READY_BUDGET_SECS=30 STAGE_BLOB_BUDGET_SECS=600)"
check "(k) a garbage record is LEFT ALONE, warned about once, and the budget holds" \
  "[ '${status}' = '1' ] && grep -q 'still not ready after 30s' '${log}' \
   && [ \"\$(grep -c '⊘ WARN' '${log}')\" = '1' ] \
   && grep -qx 'this is not a stamp' '${cell_state}/run-deadline'"

# --- (l) budget 0, and the age guard's one legitimate home -----------------
new_case l1; cell_state="${root}/l1-cells"
status="$(run_stage "http://doorway-l1" server \
  FAKE_PATCH_SEQ="503|${CELL_BODY}|0|" \
  STAGE_CELL_READY_BUDGET_SECS=0 STAGE_BLOB_BUDGET_SECS=600)"
check "(l) STAGE_CELL_READY_BUDGET_SECS=0 reports without waiting" \
  "[ '${status}' = '1' ] && no_naps '${state}' && grep -q 'STAGE_CELL_READY_BUDGET_SECS=0' '${log}'"

# A 7h-old record in a SCOPED dir belongs to this run and is honoured.
new_case l2; cell_state="${root}/l2-cells"; mkdir -p "${cell_state}"
printf 'first_not_ready=%s\n' "$(( clock_base - 25200 ))" > "${cell_state}/run-deadline"
status="$(run_stage "http://doorway-l2" server \
  FAKE_PATCH_SEQ="503|${CELL_BODY}|0|" \
  STAGE_CELL_READY_BUDGET_SECS=30 STAGE_BLOB_BUDGET_SECS=600)"
check "(l) in a SCOPED dir a 7h-old record is honoured, not aged out" \
  "[ '${status}' = '1' ] && no_naps '${state}' && ! grep -q 'ignoring a run-deadline record' '${log}'"

# The same record in the UNSCOPED fallback dir (no BUILD_TAG, no explicit dir)
# is another run's and is ignored.
state="${root}/l3"; log="${root}/l3.log"; mkdir -p "${state}"
printf '%s' "${clock_base}" > "${state}/clock"
ws3="${root}/l3-ws"; mkdir -p "${ws3}/.stage-cell-ready"
printf 'first_not_ready=%s\n' "$(( clock_base - 25200 ))" > "${ws3}/.stage-cell-ready/run-deadline"
FAKE_STATE="${state}" FAKE_CLOCK="${state}/clock" DO_PATCH=1 \
STORAGE_API_KEY_ADMIN=test-key STAGE_CELL_READY_POLL_SECS=10 WORKSPACE="${ws3}" STAGE_HARD_TIMEOUT_SECS=0 \
FAKE_PATCH_SEQ="503|${CELL_BODY}|0|" STAGE_CELL_READY_BUDGET_SECS=30 \
  bash "${script}" "${dist}" "test-slug" "http://doorway-l3" server > "${log}" 2>&1
status=$?
check "(l) in the UNSCOPED fallback dir a 7h-old record is ignored" \
  "[ '${status}' = '1' ] && grep -q 'ignoring a run-deadline record' '${log}' \
   && grep -q '0s waited, 30s left of 30s' '${log}'"

# --- (m) DECLARE_ONLY takes the same classifier and the same deadline ------
new_case m; cell_state="${root}/m-cells"
FAKE_STATE="${state}" FAKE_CLOCK="${state}/clock" \
STORAGE_API_KEY_ADMIN=test-key STAGE_CELL_READY_STATE_DIR="${cell_state}" \
STAGE_HARD_TIMEOUT_SECS=0 \
STAGE_CELL_READY_POLL_SECS=10 \
FAKE_HEAD_MODE=resolvable FAKE_DECLARE_SEQ="503|{\"status\":\"catching-up\"}|0|" \
DECLARE_ONLY=1 DECLARE_MAX_ATTEMPTS=1 SOURCE_DOORWAY_URL=http://doorway-src \
STAGE_CELL_READY_BUDGET_SECS=30 \
  bash "${script}" "-" "test-slug" "http://doorway-m" server > "${log}" 2>&1
status=$?
check "(m) DECLARE_ONLY waits on the run deadline, then exits 1" \
  "[ '${status}' = '1' ] && grep -q 'face=catching-up' '${log}' && grep -q 'readiness deadline reached' '${log}'"
check "(m) the declare ladder's own attempt counter is not consumed" \
  "[ \"\$(call_count '${state}' declare_calls)\" = '3' ]"

# --- (n) NO NEW ATTEMPT STARTS AT OR AFTER THE DEADLINE, BY ANY PATH -------
# The reviewer's fixture: an open window, then an ORDINARY transport shed whose
# own ladder sleeps past the deadline. Before the pre-attempt gate this
# dispatched at 0, 10 and 40s and returned 0.
new_case n1; cell_state="${root}/n1-cells"
status="$(run_stage "http://doorway-n1" server \
  FAKE_PATCH_SEQ='503|{"status":"catching-up"}|0|
503|{"error":"shedding"}|0|30
200|{"dhtAnchorHash":"uhCkkFAKEANCHOR"}|0|' \
  STAGE_CELL_READY_BUDGET_SECS=12 STAGE_BLOB_BUDGET_SECS=600)"
check "(n) catching-up then an ordinary shed cannot reach a third dispatch" \
  "[ '${status}' = '1' ] && [ \"\$(call_count '${state}' patch_calls)\" = '2' ]"
check "(n) both dispatches started strictly before the deadline" \
  "[ \"\$(sort -n '${state}/patch_times' | tail -1)\" -lt \"\$(( clock_base + 12 ))\" ]"
check "(n) the ordinary retry sleep was capped by the remaining deadline" \
  "[ \"\$(naps '${state}' | tr '\n' ' ')\" = '10 2 ' ]"
check "(n) and it stops with the readiness-deadline line, not a transport one" \
  "grep -q 'readiness deadline reached' '${log}' && grep -q 'before another attempt could be offered' '${log}'"

# The same shape on the DECLARE_ONLY ladder: it dispatched at 0, 10 and 12.
new_case n2; cell_state="${root}/n2-cells"
FAKE_STATE="${state}" FAKE_CLOCK="${state}/clock" \
STORAGE_API_KEY_ADMIN=test-key STAGE_CELL_READY_STATE_DIR="${cell_state}" \
STAGE_HARD_TIMEOUT_SECS=0 STAGE_CELL_READY_POLL_SECS=10 \
FAKE_HEAD_MODE=resolvable \
FAKE_DECLARE_SEQ='503|{"status":"catching-up"}|0|
503|{"error":"shedding"}|0|' \
DECLARE_ONLY=1 DECLARE_MAX_ATTEMPTS=12 SOURCE_DOORWAY_URL=http://doorway-src \
STAGE_CELL_READY_BUDGET_SECS=12 \
  bash "${script}" "-" "test-slug" "http://doorway-n2" server > "${log}" 2>&1
status=$?
check "(n) DECLARE_ONLY cannot POST at the deadline either" \
  "[ '${status}' = '1' ] && [ \"\$(call_count '${state}' declare_calls)\" = '2' ]"
check "(n) its capped ladder sleep stops short of the deadline" \
  "[ \"\$(naps '${state}' | tr '\n' ' ')\" = '10 2 ' ] && grep -q 'readiness deadline was reached before another declare' '${log}'"

# --- (o) the ORIGINAL BYTES are what gets parsed ---------------------------
o_put_case() {
  local name="$1" body="$2" desc="$3"
  new_case "${name}"; cell_state="${root}/${name}-cells"
  status="$(run_stage "http://doorway-${name}" server \
    FAKE_PUT_SEQ="200|${body}|0|" \
    STAGE_CELL_READY_BUDGET_SECS=300 STAGE_BLOB_ATTEMPTS=1 STAGE_BLOB_BUDGET_SECS=600)"
  check "(o) ${desc}" \
    "[ '${status}' != '0' ] && grep -q 'did NOT confirm the storage forward' '${log}'"
}
o_put_case o-nul '@@NUL@@' 'a NUL inside the literal true is invalid JSON and confirms nothing'
o_put_case o-badutf8 '@@BADUTF8@@' 'a body that is not valid UTF-8 confirms nothing'
o_put_case o-utf8big '@@UTF8BIG@@' '1.2 MB in 600k characters is over the BYTE limit and confirms nothing'

new_case o-nl; cell_state="${root}/o-nl-cells"
status="$(run_stage "http://doorway-o-nl" server \
  FAKE_PATCH_SEQ='503|{"status":"catching-up\n"}|0|' \
  STAGE_CELL_READY_BUDGET_SECS=300 STAGE_BLOB_ATTEMPTS=1 STAGE_BLOB_BUDGET_SECS=600)"
check "(o) a status of \"catching-up\\n\" is not \"catching-up\"" \
  "[ '${status}' = '1' ] && ! grep -q 'face=' '${log}'"

new_case o-nulstr; cell_state="${root}/o-nulstr-cells"
status="$(run_stage "http://doorway-o-nulstr" server \
  FAKE_PATCH_SEQ='503|{"status":"catching-\u0000up"}|0|' \
  STAGE_CELL_READY_BUDGET_SECS=300 STAGE_BLOB_ATTEMPTS=1 STAGE_BLOB_BUDGET_SECS=600)"
check "(o) a status carrying an escaped NUL is not \"catching-up\" either" \
  "[ '${status}' = '1' ] && ! grep -q 'face=' '${log}'"

# --- (p) an unusable deadline stamp takes the declared fallback ------------
p_stamp_case() {
  local name="$1" stamp="$2" desc="$3"
  new_case "${name}"; cell_state="${root}/${name}-cells"; mkdir -p "${cell_state}"
  printf 'first_not_ready=%s\n' "${stamp}" > "${cell_state}/run-deadline"
  status="$(run_stage "http://doorway-${name}" server \
    FAKE_PATCH_SEQ="503|${CELL_BODY}|0|" \
    STAGE_CELL_READY_BUDGET_SECS=30 STAGE_BLOB_BUDGET_SECS=600)"
  check "(p) ${desc}" \
    "[ '${status}' = '1' ] && grep -q 'still not ready after 30s' '${log}' \
     && [ \"\$(grep -c '⊘ WARN' '${log}')\" = '1' ]"
}
p_stamp_case p-huge '99999999999999999999' 'a 20-digit stamp is garbage, not 7.7e18 seconds of budget'
p_stamp_case p-future "$(( clock_base + 1000 ))" 'a future stamp does not extend the budget'

# --- (q) the first stamp is created atomically and the winner is adopted ---
# A record that appears between this leg's look and its write must be ADOPTED,
# never overwritten — otherwise two first-observers keep two different deadlines.
new_case q; cell_state="${root}/q-cells"; mkdir -p "${cell_state}"
printf 'first_not_ready=%s\n' "$(( clock_base - 20 ))" > "${cell_state}/run-deadline"
status="$(run_stage "http://doorway-q" server \
  FAKE_PATCH_SEQ="503|${CELL_BODY}|0|" \
  STAGE_CELL_READY_BUDGET_SECS=30 STAGE_BLOB_BUDGET_SECS=600)"
check "(q) an existing valid record is adopted, not overwritten" \
  "grep -qx 'first_not_ready=$(( clock_base - 20 ))' '${cell_state}/run-deadline' \
   && grep -q '20s waited, 10s left of 30s' '${log}'"

# --- (r) a long body after the CellDisabled line still classifies ----------
# `grep -q` exits at the first match, the producer takes SIGPIPE, and under
# pipefail the whole pipeline failed — so a long body made the face vanish.
new_case r; cell_state="${root}/r-cells"
status="$(run_stage "http://doorway-r" server \
  FAKE_PATCH_SEQ='503|@@LONGCELL@@|0|' \
  STAGE_CELL_READY_BUDGET_SECS=10 STAGE_BLOB_BUDGET_SECS=600)"
check "(r) CellDisabled followed by 300k characters still classifies" \
  "[ '${status}' = '1' ] && grep -q 'face=cell-not-running' '${log}'"

# --- (s) nested parentheses do not smuggle the word timeout through --------
new_case s; cell_state="${root}/s-cells"
status="$(run_stage "http://doorway-s" server \
  FAKE_PUT_SEQ='200|{"forwarded_to_storage":false,"error":"could not reach storage to forward the blob: connect error (timeout diagnostic (retry disabled)): connection refused"}|0|' \
  STAGE_CELL_READY_BUDGET_SECS=300 STAGE_BLOB_ATTEMPTS=1 STAGE_BLOB_BUDGET_SECS=600)"
check "(s) a refused connection described inside nested parens is not the timeout face" \
  "[ '${status}' = '1' ] && ! grep -q 'face=' '${log}'"

# --- (t) the HARD per-invocation bound is applied, and can be switched off --
# A fake `timeout` on its OWN PATH entry records the arguments and then runs the
# real command, so the wrapper is exercised rather than merely asserted.
tbin="${root}/tbin"; mkdir -p "${tbin}"
cat > "${tbin}/timeout" <<'FAKET'
#!/bin/bash
printf '%s\n' "$*" >> "${FAKE_STATE}/timeout-calls" 2>/dev/null
while [ "$#" -gt 0 ]; do
  case "$1" in --kill-after=*) shift ;; *) break ;; esac
done
shift
exec "$@"
FAKET
chmod +x "${tbin}/timeout"

# Run in a SUBSHELL so the fake `timeout` reaches only these two cases, and
# leave STAGE_HARD_TIMEOUT_SECS unset so the script computes its own default.
new_case t1; cell_state="${root}/t1-cells"
(
  PATH="${tbin}:${PATH}"
  FAKE_STATE="${state}" FAKE_CLOCK="${state}/clock" DO_PATCH=1 \
  STORAGE_API_KEY_ADMIN=test-key STAGE_CELL_READY_STATE_DIR="${cell_state}" \
  STAGE_CELL_READY_POLL_SECS=10 STAGE_CELL_READY_BUDGET_SECS=300 \
  STAGE_BLOB_BUDGET_SECS=30 \
    bash "${script}" "${dist}" "test-slug" "http://doorway-t1" server > "${log}" 2>&1
)
status=$?
check "(t) the stage wraps itself in the hard bound exactly once" \
  "[ '${status}' = '0' ] && [ \"\$(grep -cF '${script}' '${state}/timeout-calls')\" = '1' ]"
check "(t) with the computed default = readiness + transport + 1500" \
  "grep -q -- '--kill-after=30 1830 ' '${state}/timeout-calls'"

new_case t2; cell_state="${root}/t2-cells"
(
  PATH="${tbin}:${PATH}"
  FAKE_STATE="${state}" FAKE_CLOCK="${state}/clock" DO_PATCH=1 \
  STORAGE_API_KEY_ADMIN=test-key STAGE_CELL_READY_STATE_DIR="${cell_state}" \
  STAGE_CELL_READY_POLL_SECS=10 STAGE_CELL_READY_BUDGET_SECS=300 \
  STAGE_BLOB_BUDGET_SECS=30 STAGE_HARD_TIMEOUT_SECS=0 \
    bash "${script}" "${dist}" "test-slug" "http://doorway-t2" server > "${log}" 2>&1
)
status=$?
check "(t) STAGE_HARD_TIMEOUT_SECS=0 disables the wrapper" \
  "[ '${status}' = '0' ] && ! grep -qF '${script}' '${state}/timeout-calls'"

# --- (u) a runner WITHOUT `timeout` must still deliver a healthy upload ----
# One availability decision serves the wrapper and the parser; when it is absent
# both run unbounded under one WARN, rather than the parser failing every check.
# A complete PATH with exactly one thing missing — anything less would test the
# absence of coreutils, not the absence of `timeout`.
nobin="${root}/no-timeout-bin"; mkdir -p "${nobin}"
for d in /usr/local/bin /usr/bin /bin /usr/sbin /sbin; do
  [ -d "${d}" ] || continue
  for f in "${d}"/*; do
    b="$(basename "${f}")"
    [ "${b}" = "timeout" ] && continue
    [ -e "${nobin}/${b}" ] || ln -s "${f}" "${nobin}/${b}" 2>/dev/null
  done
done
for c in curl node date sleep; do ln -sf "${bin}/${c}" "${nobin}/${c}"; done
new_case u1; cell_state="${root}/u1-cells"
(
  PATH="${nobin}"
  command -v timeout >/dev/null 2>&1 && { echo "SETUP: timeout still on PATH" >&2; exit 99; }
  FAKE_STATE="${state}" FAKE_CLOCK="${state}/clock" DO_PATCH=1 \
  STORAGE_API_KEY_ADMIN=test-key STAGE_CELL_READY_STATE_DIR="${cell_state}" \
  STAGE_CELL_READY_POLL_SECS=10 STAGE_CELL_READY_BUDGET_SECS=300 \
  STAGE_BLOB_BUDGET_SECS=30 STAGE_JSON_NODE="${REAL_NODE}" \
    bash "${script}" "${dist}" "test-slug" "http://doorway-u1" server > "${log}" 2>&1
)
status=$?
check "(u) with no 'timeout' on PATH a healthy 200 + forwarded:true still exits 0" \
  "[ '${status}' = '0' ] && grep -q 'storage forward CONFIRMED' '${log}'"
check "(u) and it says so exactly once, as a WARN, not per parse" \
  "[ \"\$(grep -c 'is not on PATH' '${log}')\" = '1' ]"

# --- (v) a parser that could not RUN is named, not silently negative -------
vbin="${root}/v-bin"; mkdir -p "${vbin}"
# What `timeout` leaves behind when it kills the parser: 124, no output.
printf '#!/bin/bash\nexit 124\n' > "${vbin}/killed-node"
# What node itself leaves behind when it crashes: exit 1 — the SAME code as a
# clean negative. Only the missing liveness token separates them.
printf '#!/bin/bash\nexit 1\n' > "${vbin}/crashed-node"
chmod +x "${vbin}/killed-node" "${vbin}/crashed-node"

new_case v1; cell_state="${root}/v1-cells"
status="$(run_stage "http://doorway-v1" server \
  STAGE_JSON_NODE="${vbin}/killed-node" \
  STAGE_CELL_READY_BUDGET_SECS=300 STAGE_BLOB_ATTEMPTS=1 STAGE_BLOB_BUDGET_SECS=600)"
check "(v) a parser killed at its timeout is diagnosed as a runner problem" \
  "[ '${status}' != '0' ] && grep -q 'the local JSON parser did not complete (exit 124)' '${log}' \
   && grep -q 'not the doorway' '${log}'"

new_case v1b; cell_state="${root}/v1b-cells"
status="$(run_stage "http://doorway-v1b" server \
  STAGE_JSON_NODE="${vbin}/crashed-node" \
  STAGE_CELL_READY_BUDGET_SECS=300 STAGE_BLOB_ATTEMPTS=1 STAGE_BLOB_BUDGET_SECS=600)"
check "(v) a node that crashes with exit 1 is NOT read as a clean negative" \
  "[ '${status}' != '0' ] && grep -q 'the local JSON parser did not complete (exit 1)' '${log}'"

new_case v2; cell_state="${root}/v2-cells"
status="$(run_stage "http://doorway-v2" server \
  FAKE_PUT_SEQ='200|{"forwarded_to_storage":false}|0|' \
  STAGE_CELL_READY_BUDGET_SECS=300 STAGE_BLOB_ATTEMPTS=1 STAGE_BLOB_BUDGET_SECS=600)"
check "(v) a CLEAN negative carries no parser diagnostic" \
  "[ '${status}' = '1' ] && grep -q 'did NOT confirm the storage forward' '${log}' \
   && ! grep -q 'the local JSON parser did not complete' '${log}'"

# --- (w) an incomplete record published by an overlapping writer ------------
# An EMPTY run-deadline is exactly what `set -C` + printf exposed between create
# and fill. It must NOT be replaced by this leg's later stamp.
new_case w; cell_state="${root}/w-cells"; mkdir -p "${cell_state}"
: > "${cell_state}/run-deadline"
status="$(run_stage "http://doorway-w" server \
  FAKE_PATCH_SEQ="503|${CELL_BODY}|0|" \
  STAGE_CELL_READY_BUDGET_SECS=30 STAGE_BLOB_BUDGET_SECS=600)"
check "(w) an empty record is never overwritten with a later stamp" \
  "[ ! -s '${cell_state}/run-deadline' ]"
check "(w) and the invocation falls back in-process, warning once" \
  "[ '${status}' = '1' ] && grep -q 'still not ready after 30s' '${log}' \
   && [ \"\$(grep -c '⊘ WARN' '${log}')\" = '1' ]"
check "(w) no half-written temp record is left behind" \
  "[ -z \"\$(ls -A '${cell_state}' | grep -v '^run-deadline$')\" ]"

# --- (x) REAL-TIME test (~3s): TERM on the launched PID stops the staging ---
# MARKED: this is the one case in this file that uses wall-clock sleeps. Signal
# delivery cannot be simulated on the fake clock, and the defect is precisely
# that the wrapped inner script outlived the PID the caller signalled.
new_case x; cell_state="${root}/x-cells"
FAKE_STATE="${state}" FAKE_CLOCK="${state}/clock" DO_PATCH=1 \
FAKE_CURL_REAL_SLEEP=2 \
STORAGE_API_KEY_ADMIN=test-key STAGE_CELL_READY_STATE_DIR="${cell_state}" \
STAGE_CELL_READY_POLL_SECS=10 STAGE_CELL_READY_BUDGET_SECS=300 \
STAGE_BLOB_BUDGET_SECS=30 \
  bash "${script}" "${dist}" "test-slug" "http://doorway-x" server > "${log}" 2>&1 &
outer=$!
/bin/sleep 0.5
kill -TERM "${outer}" 2>/dev/null
wait "${outer}" 2>/dev/null
/bin/sleep 2.5
check "(x) TERM on the launched PID stops the run before the head PATCH" \
  "[ ! -f '${state}/patch_calls' ]"

echo "--- ${checks} checks"
if [ "${fail}" -eq 0 ]; then
  echo "ALL PASS"
fi
exit "${fail}"
