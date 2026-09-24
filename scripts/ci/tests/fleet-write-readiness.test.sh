#!/bin/bash
# scripts/ci/tests/fleet-write-readiness.test.sh — the readiness PRECONDITION
# (native-delivery sprint Lane A1): ready, each not-ready face, mixed hosts,
# unparseable answers, and the deploy intent it writes on refusal.
#
# run: bash scripts/ci/tests/fleet-write-readiness.test.sh
#
# DETERMINISM (the stage-spa-blob.test.sh harness shape). A fake `curl` on PATH
# answers from per-host fixture files, a fake `node` stands in for the SDK
# packager, and a fake `sleep` records that it was called — the script under
# test must NEVER sleep, so any record fails the suite. The JSON classifier runs
# against a REAL node (READINESS_JSON_NODE), because the parsing is under test.
set -uo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
script="${FLEET_READINESS_SCRIPT:-${here}/../fleet-write-readiness.sh}"

REAL_NODE="$(command -v node 2>/dev/null)"
if [ -z "${REAL_NODE}" ]; then
  echo "FATAL: no node on PATH — the JSON classifier under test needs a real one" >&2
  exit 1
fi
export READINESS_JSON_NODE="${REAL_NODE}"

root="$(mktemp -d 2>/dev/null)" || root=""
if [ -z "${root}" ] || [ ! -d "${root}" ]; then
  echo "FATAL: could not create a temporary root" >&2
  exit 1
fi
case "${root}" in /|/bin|/usr|/etc|/sbin) echo "FATAL: refusing root '${root}'" >&2; exit 1 ;; esac
if [ -z "${READINESS_TEST_KEEP:-}" ]; then trap 'rm -rf "${root}"' EXIT; else echo "fixture root: ${root}"; fi

bin="${root}/bin"
mkdir -p "${bin}" "${root}/app/pkg/dist/browser" "${root}/app/pkg/dist/server" \
  || { echo "FATAL: could not create fixture dirs" >&2; exit 1; }

# --- fake curl -------------------------------------------------------------
# Answers from ${FAKE_DIR}/<host>.<leg>, one record `status|body|curl-exit|Retry-After`.
# leg = health (GET /health/serving) or put (PUT /admin/seed/blob). A missing
# fixture answers curl exit 7 (connection refused). Every call's argv is logged.
cat > "${bin}/curl" <<'FAKE'
#!/bin/bash
url="" headers_out="" out="" wfmt="" method=GET
printf '%s\n' "$*" >> "${FAKE_DIR}/calls"
while [ "$#" -gt 0 ]; do
  case "$1" in
    -X) method="$2"; shift 2 ;;
    -D) headers_out="$2"; shift 2 ;;
    -o) out="$2"; shift 2 ;;
    -w) wfmt="$2"; shift 2 ;;
    -H|--max-time|--data-binary|-d) shift 2 ;;
    -*) shift ;;
    *) url="$1"; shift ;;
  esac
done
host="${url#*://}"; host="${host%%/*}"
case "${url}" in
  */health/serving) leg=health ;;
  */admin/seed/blob) leg=put ;;
  *) exit 22 ;;
esac
fixture="${FAKE_DIR}/${host}.${leg}"
[ -f "${fixture}" ] || exit 7
IFS='|' read -r st body rc ra < "${fixture}"
if [ -n "${headers_out}" ]; then
  { printf 'HTTP/1.1 %s\r\n' "${st}"; [ -n "${ra}" ] && printf 'Retry-After: %s\r\n' "${ra}"; printf '\r\n'; } > "${headers_out}"
fi
if [ -n "${out}" ] && [ "${out}" != "-" ]; then printf '%s' "${body}" > "${out}"; else printf '%s' "${body}"; fi
[ "${rc:-0}" = "0" ] && [ -n "${wfmt}" ] && printf '%s' "${st}"
exit "${rc:-0}"
FAKE

# --- fake node (SDK packager only; the classifier uses the REAL node) -------
cat > "${bin}/node" <<'FAKE'
#!/bin/bash
out="" kind=browser
while [ "$#" -gt 0 ]; do
  case "$1" in --out) out="$2"; shift 2 ;; --kind) kind="$2"; shift 2 ;; *) shift ;; esac
done
mkdir -p "${out}"
printf 'fake-%s-package' "${kind}" > "${out}/${kind}.zip"
FAKE

# --- fake sleep: the script must never call it -----------------------------
cat > "${bin}/sleep" <<'FAKE'
#!/bin/bash
printf 'slept %s\n' "$*" >> "${FAKE_DIR}/sleeps"
exit 0
FAKE
chmod +x "${bin}/curl" "${bin}/node" "${bin}/sleep"
export PATH="${bin}:${PATH}"

fail=0 checks=0
check() {
  checks=$(( checks + 1 ))
  if eval "$2"; then echo "ok   $1"; else echo "FAIL $1"; fail=1; fi
}

OK_HEALTH='200|{"shedding":false,"degrading":false,"upstreams":[],"rolesDiscovered":5,"warmupEmpty":false,"storageServing":{"status":"serving","httpStatus":200}}|0|'
OK_PUT='409|{"success":false,"hash":"sha256-0000","already_cached":false,"forwarded_to_storage":false,"size":0,"error":"Hash mismatch"}|0|'

new_case() {
  FAKE_DIR="${root}/$1"; log="${root}/$1.log"; out="${root}/$1.out"
  mkdir -p "${FAKE_DIR}"
  export FAKE_DIR
}
fixture() { printf '%s' "$3" > "${FAKE_DIR}/$1.$2"; }
run() {
  env STORAGE_API_KEY_ADMIN=test-key READINESS_OUT="${out}" "$@" \
    bash "${script}" ${HOSTS} > "${log}" 2>&1
  printf '%s' "$?"
}
never_slept() { [ ! -s "${FAKE_DIR}/sleeps" ]; }
calls_to() { local c; c="$(grep -c -- "$1" "${FAKE_DIR}/calls" 2>/dev/null)"; printf '%s' "${c:-0}"; }

# --- ready --------------------------------------------------------------------
new_case ready; HOSTS="https://doorway-a"
fixture doorway-a health "${OK_HEALTH}"; fixture doorway-a put "${OK_PUT}"
status="$(run)"
check "ready: exits 0" "[ '${status}' = '0' ]"
check "ready: names the host READY" "grep -qx 'FLEET-READY doorway-a' '${log}'"
check "ready: asks both questions exactly once" \
  "[ \"\$(calls_to /health/serving)\" = '1' ] && [ \"\$(calls_to /admin/seed/blob)\" = '1' ]"
check "ready: the write probe is a ZERO-byte PUT on the never-held sentinel, with the admin key" \
  "grep -q -- '--data-binary  https://doorway-a/admin/seed/blob' '${FAKE_DIR}/calls' && grep -q 'X-Blob-Hash: sha256-0000000000000000000000000000000000000000000000000000000000000000' '${FAKE_DIR}/calls' && grep -q 'X-API-Key: test-key' '${FAKE_DIR}/calls'"
check "ready: every request carries a --max-time bound" \
  "[ \"\$(grep -c -- '--max-time' '${FAKE_DIR}/calls')\" = '2' ]"
check "ready: the result file carries the line" "grep -qx 'FLEET-READY doorway-a' '${out}'"
check "ready: never sleeps" "never_slept"

# --- face: shedding (health 503, Retry-After header) -------------------------
new_case shed; HOSTS="https://doorway-a"
fixture doorway-a health '503|{"shedding":true,"degrading":true,"upstreams":[],"storageServing":{"status":"serving"}}|0|45'
fixture doorway-a put "${OK_PUT}"
status="$(run)"
check "shedding: exits 3" "[ '${status}' = '3' ]"
check "shedding: names face and the advertised retryAfter" \
  "grep -qx 'FLEET-NOT-READY doorway-a face=shedding retryAfter=45' '${log}'"
check "shedding: refuses on the first answer — no write probe" "[ \"\$(calls_to /admin/seed/blob)\" = '0' ]"
check "shedding: never sleeps" "never_slept"

# --- face: degrading ----------------------------------------------------------
new_case degr; HOSTS="https://doorway-a"
fixture doorway-a health '503|{"shedding":false,"degrading":true,"upstreams":[],"storageServing":{"status":"serving"}}|0|30'
status="$(run)"
check "degrading: exits 3 with face=degrading" \
  "[ '${status}' = '3' ] && grep -qx 'FLEET-NOT-READY doorway-a face=degrading retryAfter=30' '${log}'"

# --- face: storage refused / unreachable -------------------------------------
new_case sref; HOSTS="https://doorway-a"
fixture doorway-a health '503|{"shedding":false,"degrading":false,"upstreams":[],"storageServing":{"status":"refused","httpStatus":503}}|0|20'
status="$(run)"
check "storage refused: exits 3 with face=storage-refused" \
  "[ '${status}' = '3' ] && grep -qx 'FLEET-NOT-READY doorway-a face=storage-refused retryAfter=20' '${log}'"
new_case sunr; HOSTS="https://doorway-a"
fixture doorway-a health '503|{"shedding":false,"degrading":false,"upstreams":[],"storageServing":{"status":"unreachable"}}|0|'
status="$(run)"
check "storage unreachable: exits 3, face named, default retryAfter when none advertised" \
  "[ '${status}' = '3' ] && grep -qx 'FLEET-NOT-READY doorway-a face=storage-unreachable retryAfter=60' '${log}'"

# --- the other two serving regimes the doorway 503s on ------------------------
new_case blind; HOSTS="https://doorway-a"
fixture doorway-a health '503|{"shedding":false,"degrading":false,"upstreams":[],"rolesDiscovered":0,"storageServing":{"status":"serving"}}|0|30'
status="$(run)"
check "conductor-blind: exits 3 with face=conductor-blind" \
  "[ '${status}' = '3' ] && grep -q 'face=conductor-blind' '${log}'"

# --- face: catching-up (write probe, body retryAfter) -------------------------
new_case catch; HOSTS="https://doorway-a"
fixture doorway-a health "${OK_HEALTH}"
fixture doorway-a put '503|{"status":"catching-up","retryAfter":30,"cause":"upstream"}|0|'
status="$(run)"
check "catching-up: exits 3 with face=catching-up and the body's retryAfter" \
  "[ '${status}' = '3' ] && grep -qx 'FLEET-NOT-READY doorway-a face=catching-up retryAfter=30' '${log}'"
check "catching-up: never sleeps" "never_slept"

# A 503 whose status is only NESTED is not the doorway declaring itself.
new_case nested; HOSTS="https://doorway-a"
fixture doorway-a health "${OK_HEALTH}"
fixture doorway-a put '503|{"error":{"status":"catching-up"}}|0|'
status="$(run)"
check "a nested catching-up is a plain write shed, not the catching-up face" \
  "[ '${status}' = '3' ] && grep -q 'face=write-shed' '${log}'"

# --- face: storage-forward-timeout (2xx, forward not confirmed, timed out) ----
new_case fwd; HOSTS="https://doorway-a"
fixture doorway-a health "${OK_HEALTH}"
fixture doorway-a put '200|{"success":true,"hash":"x","already_cached":true,"forwarded_to_storage":false,"size":0,"error":"could not reach storage to forward the blob: error sending request for url (http://timeout-storage:8090/blob/x): operation timed out"}|0|'
status="$(run)"
check "storage-forward-timeout: exits 3 with the face" \
  "[ '${status}' = '3' ] && grep -q 'face=storage-forward-timeout' '${log}'"
# The URL is not the cause: a host NAMED timeout-* that refused is not a timeout.
new_case fwdref; HOSTS="https://doorway-a"
fixture doorway-a health "${OK_HEALTH}"
fixture doorway-a put '200|{"success":true,"hash":"x","already_cached":true,"forwarded_to_storage":false,"size":0,"error":"could not reach storage to forward the blob: error sending request for url (http://timeout-storage:8090/blob/x): connection refused"}|0|'
status="$(run)"
check "a refused forward to a host named timeout-* is storage-forward-failed, not a timeout" \
  "[ '${status}' = '3' ] && grep -q 'face=storage-forward-failed' '${log}'"

# --- mixed hosts --------------------------------------------------------------
new_case mixed; HOSTS="https://doorway-a https://doorway-b"
fixture doorway-a health "${OK_HEALTH}"; fixture doorway-a put "${OK_PUT}"
fixture doorway-b health "${OK_HEALTH}"
fixture doorway-b put '503|{"status":"catching-up","retryAfter":30}|0|30'
status="$(run)"
check "mixed: one not-ready host refuses the fleet (exit 3)" "[ '${status}' = '3' ]"
check "mixed: the ready host is still named READY" "grep -qx 'FLEET-READY doorway-a' '${out}'"
check "mixed: the not-ready host is named with its face" \
  "grep -qx 'FLEET-NOT-READY doorway-b face=catching-up retryAfter=30' '${out}'"

# --- unreachable doorway -----------------------------------------------------
new_case down; HOSTS="https://doorway-a"
status="$(run)"
check "a doorway that does not answer is not ready (face=doorway-unreachable)" \
  "[ '${status}' = '3' ] && grep -q 'FLEET-NOT-READY doorway-a face=doorway-unreachable' '${log}'"

# --- unparseable / cannot judge => 2 ------------------------------------------
new_case junk; HOSTS="https://doorway-a"
fixture doorway-a health '200|this is not json|0|'
status="$(run)"
check "unparseable health body: exits 2 and names it" \
  "[ '${status}' = '2' ] && grep -q 'FLEET-READINESS-UNKNOWN doorway-a reason=health-unparseable' '${log}'"
new_case nocontract; HOSTS="https://doorway-a"
fixture doorway-a health '200|{"ok":true}|0|'
status="$(run)"
check "a 200 body without the serving contract fields is unparseable, not ready" \
  "[ '${status}' = '2' ] && grep -q 'reason=health-unparseable' '${log}'"
new_case unauth; HOSTS="https://doorway-a"
fixture doorway-a health "${OK_HEALTH}"
fixture doorway-a put '401|{"error":"unauthorized"}|0|'
status="$(run)"
check "an unauthorized write probe cannot judge (exit 2, reason named)" \
  "[ '${status}' = '2' ] && grep -q 'reason=probe-unauthorized' '${log}'"
new_case prec; HOSTS="https://doorway-a https://doorway-b"
fixture doorway-a health '200|garbage|0|'
fixture doorway-b health '503|{"shedding":true,"degrading":false,"upstreams":[]}|0|30'
status="$(run)"
check "not-ready outranks unknown across hosts (exit 3)" "[ '${status}' = '3' ]"

new_case usage; HOSTS=""
status="$(run)"
check "no doorway given: usage, exit 2" "[ '${status}' = '2' ] && grep -qi 'usage' '${log}'"

# --- the deploy intent ---------------------------------------------------------
new_case intent; HOSTS="https://doorway-a https://doorway-b"
fixture doorway-a health "${OK_HEALTH}"; fixture doorway-a put "${OK_PUT}"
fixture doorway-b health '503|{"shedding":true,"degrading":false,"upstreams":[]}|0|90'
intent="${root}/intent/deploy-intent.json"
status="$(run DEPLOY_INTENT_OUT="${intent}" DEPLOY_INTENT_COMMIT=abc123 DEPLOY_INTENT_ENV=dev \
  DEPLOY_INTENT_BUNDLES="landing:browser:${root}/app/pkg/dist/browser landing:server:${root}/app/pkg/dist/server")"
browser_sha="sha256-$(printf 'fake-browser-package' | sha256sum | awk '{print $1}')"
server_sha="sha256-$(printf 'fake-server-package' | sha256sum | awk '{print $1}')"
q() { "${REAL_NODE}" -e 'const o=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));process.stdout.write(String(eval(process.argv[2])))' "${intent}" "$1" 2>/dev/null; }
check "intent: refusal still exits 3" "[ '${status}' = '3' ]"
check "intent: written, and it is JSON" "[ -f '${intent}' ] && [ \"\$(q 'typeof o')\" = 'object' ]"
check "intent: commit, env, doorway, face, retryAfter" \
  "[ \"\$(q o.commit)\" = 'abc123' ] && [ \"\$(q o.env)\" = 'dev' ] && [ \"\$(q o.doorway)\" = 'doorway-b' ] && [ \"\$(q o.face)\" = 'shedding' ] && [ \"\$(q o.retryAfter)\" = '90' ]"
check "intent: every probed doorway URL is carried for the re-dispatch" \
  "[ \"\$(q 'o.doorways.join(\" \")')\" = 'https://doorway-a https://doorway-b' ]"
check "intent: bundle sha256s are the archive CIDs the stage leg would upload" \
  "[ \"\$(q 'o.bundles[0].sha256')\" = '${browser_sha}' ] && [ \"\$(q 'o.bundles[1].sha256')\" = '${server_sha}' ] && [ \"\$(q 'o.bundles[1].kind')\" = 'server' ]"
check "intent: never sleeps" "never_slept"

new_case nointent; HOSTS="https://doorway-a"
fixture doorway-a health "${OK_HEALTH}"; fixture doorway-a put "${OK_PUT}"
intent="${root}/nointent/deploy-intent.json"
status="$(run DEPLOY_INTENT_OUT="${intent}" DEPLOY_INTENT_COMMIT=abc DEPLOY_INTENT_ENV=dev)"
check "ready: no intent is written" "[ '${status}' = '0' ] && [ ! -e '${intent}' ]"

echo "--- ${checks} checks"
if [ "${fail}" -ne 0 ]; then echo "SOME FAILED"; exit 1; fi
echo "ALL PASS"
