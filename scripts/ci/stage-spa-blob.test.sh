#!/bin/bash
# stage-spa-blob.test.sh — regression coverage for the head-PATCH ladder's
# classification of a CellDisabled answer (2026-09-21).
#
# run: bash scripts/ci/stage-spa-blob.test.sh
#
# Fake `curl`, `node` and `sleep` on PATH stand in for the doorway, the SDK
# packaging step, and the clock, so no network, no dist, and no deployed host
# are required. The fake `sleep` RECORDS each call and then performs it for
# real: budgets here are 1-3s, so the file sleeps only a few seconds total, and
# every timing assertion reads the recorded calls rather than wall-clock
# (a loaded runner must not be able to flake this file).
#
# KIND=server is used throughout so the deliverability gate (browser-only) is
# skipped — the leg under test is the serverBlobHash PATCH, which takes the
# identical ladder.
set -uo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
# STAGE_SPA_BLOB_SCRIPT lets a red-first run point the same cases at a
# pre-fix copy of the script; unset, the checked-in one is under test.
script="${STAGE_SPA_BLOB_SCRIPT:-${here}/stage-spa-blob.sh}"

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
trap 'rm -rf "${root}"' EXIT

bin="${root}/bin"
dist="${root}/app/pkg/dist/server"
mkdir -p "${bin}" "${dist}" || { echo "FATAL: could not create fixture dirs under ${root}" >&2; exit 1; }
# Every fixture path must resolve INSIDE the root before anything is written.
for p in "${bin}/curl" "${bin}/node" "${bin}/sleep"; do
  case "${p}" in
    "${root}"/*) : ;;
    *) echo "FATAL: fixture path '${p}' escapes the test root '${root}'" >&2; exit 1 ;;
  esac
done

# --- fake curl -------------------------------------------------------------
# Answers the calls this script makes: the blob PUT, the head PATCH, the
# read-back GET, the advisory head GET, and the canonical-head POST. The PATCH
# answer is driven by FAKE_PATCH_MODE + FAKE_CELL_TIMES and counted in
# FAKE_STATE. FAKE_PATCH_DELAY makes an attempt take real time (the transport
# budget must not be charged for a readiness attempt, however slow it is); it
# calls the REAL sleep so it never pollutes the recorded retry cadence.
cat > "${bin}/curl" <<'FAKE'
#!/bin/bash
method=GET
url=""
headers_out=""
data=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    -X) method="$2"; shift 2 ;;
    -D) headers_out="$2"; shift 2 ;;
    -d) data="$2"; shift 2 ;;
    -H|-w|-o|--data-binary|--max-time) shift 2 ;;
    -*) shift ;;
    *) url="$1"; shift ;;
  esac
done

real_sleep() { [ -x /bin/sleep ] && /bin/sleep "$1" || /usr/bin/sleep "$1"; }

case "${url}" in
  */admin/seed/blob)
    printf '{"ok":true,"forwarded_to_storage":true}'
    exit 0
    ;;
  */head-record)
    exit 22
    ;;
  */head)
    # Advisory canonical-head leg: unless a case asks for a resolvable head,
    # answer "no route" so the stage logs a warning and returns, keeping the
    # test to the leg under test.
    [ "${FAKE_HEAD_MODE:-missing}" = "resolvable" ] || exit 22
    printf '{"headActionHash":"uhCkkFAKEHEAD"}'
    exit 0
    ;;
  */canonical-head)
    if [ "${FAKE_DECLARE_MODE:-ok}" = "cell-forever" ]; then
      printf '{"error":"Conductor returned an error while using a ConductorApi: CellDisabled(CellId(uhC0kFAKE))"}\n503'
    else
      printf '{"ok":true}\n200'
    fi
    exit 0
    ;;
esac

if [ "${method}" = "PATCH" ]; then
  count=1
  if [ -f "${FAKE_STATE}/patches" ]; then
    count=$(( $(cat "${FAKE_STATE}/patches") + 1 ))
  fi
  printf '%s' "${count}" > "${FAKE_STATE}/patches"
  printf '%s' "${data}" | sed -n 's/.*:"\([^"]*\)".*/\1/p' > "${FAKE_STATE}/hash"

  disabled='{"error":"Conductor returned an error while using a ConductorApi: CellDisabled(CellId(uhC0kFAKE))"}'
  shed_body='{"status":"catching-up","retryAfter":1}'
  shed_headers='HTTP/1.1 503\r\nRetry-After: 1\r\n\r\n'
  case "${FAKE_PATCH_MODE}" in
    cell-then-ok)
      if [ "${count}" -le "${FAKE_CELL_TIMES}" ]; then
        [ -n "${FAKE_PATCH_DELAY:-}" ] && real_sleep "${FAKE_PATCH_DELAY}"
        printf '%s\n503' "${disabled}"
      else
        printf '{"dhtAnchorHash":"uhCkkFAKEANCHOR"}\n200'
      fi
      ;;
    cell-then-shed-then-ok)
      # The reviewer's mixed case: slow readiness answers, then ONE ordinary
      # shed (which DOES spend the transport budget), then success.
      if [ "${count}" -le "${FAKE_CELL_TIMES}" ]; then
        [ -n "${FAKE_PATCH_DELAY:-}" ] && real_sleep "${FAKE_PATCH_DELAY}"
        printf '%s\n503' "${disabled}"
      elif [ "${count}" -eq $(( FAKE_CELL_TIMES + 1 )) ]; then
        [ -n "${headers_out}" ] && printf "${shed_headers}" > "${headers_out}"
        printf '%s\n503' "${shed_body}"
      else
        printf '{"dhtAnchorHash":"uhCkkFAKEANCHOR"}\n200'
      fi
      ;;
    cell-forever)
      printf '%s\n503' "${disabled}"
      ;;
    shed)
      [ -n "${headers_out}" ] && printf "${shed_headers}" > "${headers_out}"
      printf '%s\n503' "${shed_body}"
      ;;
    structural)
      printf '{"error":"content row conflict"}\n409'
      ;;
  esac
  exit 0
fi

# Read-back of the row the PATCH just wrote.
case "${url}" in
  */db/content/*)
    printf '{"serverBlobHash":"%s"}' "$(cat "${FAKE_STATE}/hash" 2>/dev/null)"
    exit 0
    ;;
esac
exit 22
FAKE

# --- fake node (SDK packaging) --------------------------------------------
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

# --- fake sleep (records, then really sleeps) ------------------------------
# Recording is what the timing assertions read. The sleep still happens, so the
# script's own wall-clock budgets behave exactly as in production.
cat > "${bin}/sleep" <<'FAKE'
#!/bin/bash
printf '%s\n' "${1:-0}" >> "${FAKE_STATE}/sleeps" 2>/dev/null
[ -x /bin/sleep ] && exec /bin/sleep "$@"
exec /usr/bin/sleep "$@"
FAKE

chmod +x "${bin}/curl" "${bin}/node" "${bin}/sleep" \
  || { echo "FATAL: could not make the fixtures executable" >&2; exit 1; }
export PATH="${bin}:${PATH}"

fail=0
check() {
  if eval "$2"; then
    echo "ok   $1"
  else
    echo "FAIL $1"
    fail=1
  fi
}

# Recorded-sleep helpers — every timing claim in this file reads these.
naps() { cat "${1}/sleeps" 2>/dev/null; }
no_naps() { [ ! -s "${1}/sleeps" ]; }
nap_count() { naps "$1" | grep -c . ; }
longest_nap() { naps "$1" | sort -n | tail -1; }

# Run one stage against a doorway URL. Echoes the exit status; the log lands in
# $log. Every case is DO_PATCH=1 (the authoring leg) and KIND=server.
run_stage() {
  local url="$1" log="$2"
  shift 2
  FAKE_STATE="${state}" \
  DO_PATCH=1 \
  STORAGE_API_KEY_ADMIN=test-key \
  STAGE_CELL_READY_STATE_DIR="${cell_state}" \
  STAGE_CELL_READY_POLL_SECS=1 \
  env "$@" bash "${script}" "${dist}" "test-slug" "${url}" server > "${log}" 2>&1
  printf '%s' "$?"
}

# --- (a) CellDisabled x3 then 200 -----------------------------------------
# STAGE_BLOB_ATTEMPTS=1 and a 2s transport budget are the proof that the
# readiness wait charges neither: three 1s polls would blow both if it did.
state="${root}/a"; cell_state="${root}/a-cells"; mkdir -p "${state}"
log="${root}/a.log"
status="$(run_stage "http://doorway-a" "${log}" \
  FAKE_PATCH_MODE=cell-then-ok FAKE_CELL_TIMES=3 \
  STAGE_CELL_READY_BUDGET_SECS=30 STAGE_BLOB_ATTEMPTS=1 STAGE_BLOB_BUDGET_SECS=2)"
check "(a) CellDisabled x3 then 200 exits 0" "[ '${status}' = '0' ]"
check "(a) prints the recovery line" "grep -q 'started running after' '${log}'"
check "(a) the transport attempt counter is not consumed" "! grep -q 'stage failed after' '${log}'"
check "(a) names the readiness window, not a permanent answer" \
  "grep -q 'post-restart readiness window' '${log}'"
check "(a) the waiting was the readiness cadence, three polls" \
  "[ \"\$(nap_count '${state}')\" = '3' ]"

# --- (g) mixed: slow readiness answers, then an ordinary shed, then 200 ----
# The reviewer's regression. Each CellDisabled attempt takes ~1s of real time;
# only the SHED attempt and its advertised 1s wait may be charged to the 2s
# transport budget. Charging the readiness attempts too (the first cut of this
# fix) exhausted the budget at "1 attempt" despite an attempt cap of 5.
state="${root}/g"; cell_state="${root}/g-cells"; mkdir -p "${state}"
log="${root}/g.log"
status="$(run_stage "http://doorway-g" "${log}" \
  FAKE_PATCH_MODE=cell-then-shed-then-ok FAKE_CELL_TIMES=3 FAKE_PATCH_DELAY=1 \
  STAGE_CELL_READY_BUDGET_SECS=60 STAGE_BLOB_ATTEMPTS=5 STAGE_BLOB_BUDGET_SECS=2)"
check "(g) slow readiness attempts do not spend the transport budget" \
  "[ '${status}' = '0' ] && ! grep -q 'stage failed after' '${log}'"
check "(g) the ordinary shed still rode the transport ladder" \
  "grep -q 'retryAfter=1s' '${log}'"

# --- (h) a recovered wait clears this host's clock -------------------------
state="${root}/h1"; cell_state="${root}/h-cells"; mkdir -p "${state}"
log="${root}/h1.log"
status="$(run_stage "http://doorway-h" "${log}" \
  FAKE_PATCH_MODE=cell-then-ok FAKE_CELL_TIMES=2 \
  STAGE_CELL_READY_BUDGET_SECS=30 STAGE_BLOB_BUDGET_SECS=30)"
check "(h) the recovering run exits 0" "[ '${status}' = '0' ]"
check "(h) recovery removes this host's readiness record" \
  "[ ! -f '${cell_state}/http___doorway-h' ]"
# A LATER CellDisabled against the same host is a NEW window: it must get a
# full budget, not inherit the one this run just watched clear.
state="${root}/h2"; mkdir -p "${state}"
log="${root}/h2.log"
status="$(run_stage "http://doorway-h" "${log}" \
  FAKE_PATCH_MODE=cell-forever STAGE_CELL_READY_BUDGET_SECS=2)"
check "(h) the next CellDisabled gets a fresh full budget" \
  "[ '${status}' = '1' ] && ! grep -q 'already spent' '${log}' && grep -q 'post-restart readiness window' '${log}'"

# --- (b) CellDisabled forever, shared per-host budget ----------------------
state="${root}/b"; cell_state="${root}/b-cells"; mkdir -p "${state}"
log="${root}/b1.log"
status="$(run_stage "http://doorway-b" "${log}" \
  FAKE_PATCH_MODE=cell-forever STAGE_CELL_READY_BUDGET_SECS=3)"
check "(b) an exhausted readiness budget exits 1" "[ '${status}' = '1' ]"
check "(b) reports the measurement, not a diagnosis" \
  "grep -q 'readiness budget exhausted: the cell was still unavailable after' '${log}'"
check "(b) points at the conductor's own state instead of prescribing a cure" \
  "grep -q \"Read that conductor's app/cell state\" '${log}' && ! grep -qi 'genuinely disabled' '${log}'"
check "(b) leaves the host STALE by name" "grep -q 'Host left STALE' '${log}'"

# A SECOND invocation against the SAME host inherits the spent clock and must
# fail without a single retry — this is what keeps the 63-minute per-leg
# re-spend cured.
state="${root}/b2"; mkdir -p "${state}"
log="${root}/b2.log"
status="$(run_stage "http://doorway-b" "${log}" \
  FAKE_PATCH_MODE=cell-forever STAGE_CELL_READY_BUDGET_SECS=3)"
check "(b) a second leg against the same host exits 1" "[ '${status}' = '1' ]"
check "(b) it never waits — no retry storm" "no_naps '${state}'"
check "(b) it names when the wait began and what was spent" \
  "grep -q 'readiness budget was already spent earlier in this run' '${log}'"

# A DIFFERENT host is unaffected: its own clock starts fresh and it waits.
state="${root}/b3"; mkdir -p "${state}"
log="${root}/b3.log"
status="$(run_stage "http://doorway-other" "${log}" \
  FAKE_PATCH_MODE=cell-forever STAGE_CELL_READY_BUDGET_SECS=1)"
check "(b) a different host is unaffected — it waits on its own clock" \
  "grep -q 'post-restart readiness window' '${log}' && ! grep -q 'already spent' '${log}'"

# --- (i) the budget is an enforced deadline --------------------------------
# A poll interval longer than what is left must be capped to the remainder:
# budget 1 / poll 3 ends at ~1s, and the ONE recorded nap is 1, never 3.
state="${root}/i"; cell_state="${root}/i-cells"; mkdir -p "${state}"
log="${root}/i.log"
status="$(run_stage "http://doorway-i" "${log}" \
  FAKE_PATCH_MODE=cell-forever \
  STAGE_CELL_READY_BUDGET_SECS=1 STAGE_CELL_READY_POLL_SECS=3)"
check "(i) a poll longer than the remaining budget is capped to it" \
  "[ '${status}' = '1' ] && [ \"\$(longest_nap '${state}')\" = '1' ]"

# --- (c) budget 0 restores the old immediate-structural behaviour ----------
state="${root}/c"; cell_state="${root}/c-cells"; mkdir -p "${state}"
log="${root}/c.log"
status="$(run_stage "http://doorway-c" "${log}" \
  FAKE_PATCH_MODE=cell-forever STAGE_CELL_READY_BUDGET_SECS=0)"
check "(c) STAGE_CELL_READY_BUDGET_SECS=0 exits 1" "[ '${status}' = '1' ]"
check "(c) it does not wait" "no_naps '${state}'"
check "(c) it names the switch that disabled waiting" \
  "grep -q 'STAGE_CELL_READY_BUDGET_SECS=0' '${log}'"

# --- (d) no regression for the ordinary shed / structural classes ----------
state="${root}/d1"; cell_state="${root}/d1-cells"; mkdir -p "${state}"
log="${root}/d1.log"
status="$(run_stage "http://doorway-d1" "${log}" \
  FAKE_PATCH_MODE=shed STAGE_BLOB_ATTEMPTS=2 STAGE_BLOB_BUDGET_SECS=30)"
check "(d) an ordinary 503 shed still spends the TRANSPORT budget" \
  "[ '${status}' = '1' ] && grep -q 'stage failed after 2 attempt(s)' '${log}'"
check "(d) it honours the advertised retryAfter" \
  "grep -q 'retryAfter=1s' '${log}' && [ \"\$(longest_nap '${state}')\" = '1' ]"
check "(d) a shed never enters the readiness wait" \
  "! grep -q 'readiness window' '${log}'"

state="${root}/d2"; cell_state="${root}/d2-cells"; mkdir -p "${state}"
log="${root}/d2.log"
status="$(run_stage "http://doorway-d2" "${log}" \
  FAKE_PATCH_MODE=structural STAGE_BLOB_ATTEMPTS=5 STAGE_BLOB_BUDGET_SECS=30)"
check "(d) a structural 4xx is still immediate" \
  "[ '${status}' = '1' ] && no_naps '${state}' && grep -q 'failed structurally (HTTP 409)' '${log}'"

# --- (e) a state file older than 6h is ignored -----------------------------
state="${root}/e"; cell_state="${root}/e-cells"; mkdir -p "${state}" "${cell_state}"
printf 'first_seen=%s\nexhausted=%s\n' "$(( $(date +%s) - 25000 ))" "$(( $(date +%s) - 24000 ))" \
  > "${cell_state}/http___doorway-e"
log="${root}/e.log"
status="$(run_stage "http://doorway-e" "${log}" \
  FAKE_PATCH_MODE=cell-forever STAGE_CELL_READY_BUDGET_SECS=1)"
check "(e) a stale record is ignored, not inherited" \
  "grep -q 'ignoring a cell-readiness record' '${log}' && ! grep -q 'already spent' '${log}'"
check "(e) and the fresh clock is actually used" \
  "[ '${status}' = '1' ] && grep -q 'post-restart readiness window' '${log}'"

# --- (f) the DECLARE_ONLY ladder shares the same per-host clock ------------
# A CellDisabled answer reaches that ladder too (503 from storage's
# conductor_write_error, 502 from a pre-fix binary). DECLARE_MAX_ATTEMPTS=1
# proves the wait does not consume that ladder's own attempt counter either.
state="${root}/f"; cell_state="${root}/f-cells"; mkdir -p "${state}"
log="${root}/f.log"
FAKE_STATE="${state}" \
STORAGE_API_KEY_ADMIN=test-key \
STAGE_CELL_READY_STATE_DIR="${cell_state}" \
STAGE_CELL_READY_POLL_SECS=1 \
FAKE_HEAD_MODE=resolvable FAKE_DECLARE_MODE=cell-forever \
DECLARE_ONLY=1 DECLARE_MAX_ATTEMPTS=1 SOURCE_DOORWAY_URL=http://doorway-src \
STAGE_CELL_READY_BUDGET_SECS=2 \
  bash "${script}" "-" "test-slug" "http://doorway-f" server > "${log}" 2>&1
status=$?
check "(f) DECLARE_ONLY waits on the readiness budget, then exits 1" \
  "[ '${status}' = '1' ] && grep -q 'post-restart readiness window' '${log}'"
check "(f) the declare ladder's own attempt counter is not consumed" \
  "grep -q 'readiness budget exhausted' '${log}'"

# --- (j) BUILD_TAG scopes the default state dir to one build ---------------
# Without an explicit STAGE_CELL_READY_STATE_DIR, two builds sharing a
# workspace must not share a clock: build 2 gets its own budget.
ws="${root}/j-ws"; mkdir -p "${ws}"
for tag in build-1 build-2; do
  state="${root}/j-${tag}"; mkdir -p "${state}"
  log="${root}/j-${tag}.log"
  FAKE_STATE="${state}" \
  DO_PATCH=1 \
  STORAGE_API_KEY_ADMIN=test-key \
  STAGE_CELL_READY_POLL_SECS=1 \
  WORKSPACE="${ws}" BUILD_TAG="${tag}" \
  FAKE_PATCH_MODE=cell-forever STAGE_CELL_READY_BUDGET_SECS=1 \
    bash "${script}" "${dist}" "test-slug" "http://doorway-j" server > "${log}" 2>&1
done
check "(j) a second BUILD_TAG does not inherit the first build's spent clock" \
  "! grep -q 'already spent' '${root}/j-build-2.log' && grep -q 'post-restart readiness window' '${root}/j-build-2.log'"
check "(j) each build's clock is filed under its own tag" \
  "[ -f '${ws}/.stage-cell-ready/build-1/http___doorway-j' ] && [ -f '${ws}/.stage-cell-ready/build-2/http___doorway-j' ]"

if [ "${fail}" -eq 0 ]; then
  echo "ALL PASS"
fi
exit "${fail}"
