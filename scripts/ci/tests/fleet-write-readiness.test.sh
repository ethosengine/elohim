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
# leg = health (GET /health/serving), read (GET /db/content/<sentinel>, the
# route the household's declared shed covers) or put (PUT /admin/seed/blob).
# The host keys a fixture WITH its port, so two doorways on one machine are two
# fixtures. A missing fixture answers curl exit 7 (connection refused). Every
# call's argv is logged.
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
  */db/content/*) leg=read ;;
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
OK_READ='404|{"error":"Content not found"}|0|'
OK_PUT='409|{"success":false,"hash":"sha256-0000","already_cached":false,"forwarded_to_storage":false,"size":0,"error":"Hash mismatch"}|0|'

new_case() {
  FAKE_DIR="${root}/$1"; log="${root}/$1.log"; out="${root}/$1.out"
  mkdir -p "${FAKE_DIR}"
  export FAKE_DIR
  # Every case's doorways answer the shed-covered read as "not held" unless the
  # case says otherwise — the read leg is only interesting where it refuses.
  local h
  for h in doorway-a doorway-b localhost:8888 localhost:8889; do
    printf '%s' "${OK_READ}" > "${FAKE_DIR}/${h}.read"
  done
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
check "ready: names the host READY" "grep -qx 'FLEET-READY https://doorway-a' '${log}'"
check "ready: asks each of the three questions exactly once" \
  "[ \"\$(calls_to /health/serving)\" = '1' ] && [ \"\$(calls_to /db/content/)\" = '1' ] && [ \"\$(calls_to /admin/seed/blob)\" = '1' ]"
check "ready: the write probe is a ZERO-byte PUT on the never-held sentinel, with the admin key" \
  "grep -q -- '--data-binary  https://doorway-a/admin/seed/blob' '${FAKE_DIR}/calls' && grep -q 'X-Blob-Hash: sha256-0000000000000000000000000000000000000000000000000000000000000000' '${FAKE_DIR}/calls' && grep -q 'X-API-Key: test-key' '${FAKE_DIR}/calls'"
check "ready: every request carries a --max-time bound" \
  "[ \"\$(grep -c -- '--max-time' '${FAKE_DIR}/calls')\" = '3' ]"
check "ready: the result file carries the line" "grep -qx 'FLEET-READY https://doorway-a' '${out}'"
check "ready: never sleeps" "never_slept"

# --- face: shedding (health 503, Retry-After header) -------------------------
new_case shed; HOSTS="https://doorway-a"
fixture doorway-a health '503|{"shedding":true,"degrading":true,"upstreams":[],"storageServing":{"status":"serving"}}|0|45'
fixture doorway-a put "${OK_PUT}"
status="$(run)"
check "shedding: exits 3" "[ '${status}' = '3' ]"
check "shedding: names face and the advertised retryAfter" \
  "grep -qx 'FLEET-NOT-READY https://doorway-a face=shedding retryAfter=45' '${log}'"
check "shedding: refuses on the first answer — no write probe" "[ \"\$(calls_to /admin/seed/blob)\" = '0' ]"
check "shedding: never sleeps" "never_slept"

# --- face: degrading ----------------------------------------------------------
new_case degr; HOSTS="https://doorway-a"
fixture doorway-a health '503|{"shedding":false,"degrading":true,"upstreams":[],"storageServing":{"status":"serving"}}|0|30'
status="$(run)"
check "degrading: exits 3 with face=degrading" \
  "[ '${status}' = '3' ] && grep -qx 'FLEET-NOT-READY https://doorway-a face=degrading retryAfter=30' '${log}'"

# --- face: storage refused / unreachable -------------------------------------
new_case sref; HOSTS="https://doorway-a"
fixture doorway-a health '503|{"shedding":false,"degrading":false,"upstreams":[],"storageServing":{"status":"refused","httpStatus":503}}|0|20'
status="$(run)"
check "storage refused: exits 3 with face=storage-refused" \
  "[ '${status}' = '3' ] && grep -qx 'FLEET-NOT-READY https://doorway-a face=storage-refused retryAfter=20' '${log}'"
new_case sunr; HOSTS="https://doorway-a"
fixture doorway-a health '503|{"shedding":false,"degrading":false,"upstreams":[],"storageServing":{"status":"unreachable"}}|0|'
status="$(run)"
check "storage unreachable: exits 3, face named, default retryAfter when none advertised" \
  "[ '${status}' = '3' ] && grep -qx 'FLEET-NOT-READY https://doorway-a face=storage-unreachable retryAfter=60' '${log}'"

# --- the other two serving regimes the doorway 503s on ------------------------
new_case blind; HOSTS="https://doorway-a"
fixture doorway-a health '503|{"shedding":false,"degrading":false,"upstreams":[],"rolesDiscovered":0,"storageServing":{"status":"serving"}}|0|30'
status="$(run)"
check "no discovered roles is the plan face cell-not-running (the retired conductor-blind name is gone)" \
  "[ '${status}' = '3' ] && grep -qx 'FLEET-NOT-READY https://doorway-a face=cell-not-running retryAfter=30' '${log}' && ! grep -q 'conductor-blind' '${log}'"

# --- face: catching-up (write probe, body retryAfter) -------------------------
new_case catch; HOSTS="https://doorway-a"
fixture doorway-a health "${OK_HEALTH}"
fixture doorway-a put '503|{"status":"catching-up","retryAfter":30,"cause":"upstream"}|0|'
status="$(run)"
check "catching-up: exits 3 with face=catching-up and the body's retryAfter" \
  "[ '${status}' = '3' ] && grep -qx 'FLEET-NOT-READY https://doorway-a face=catching-up retryAfter=30' '${log}'"
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
check "mixed: the ready host is still named READY" "grep -qx 'FLEET-READY https://doorway-a' '${out}'"
check "mixed: the not-ready host is named with its face" \
  "grep -qx 'FLEET-NOT-READY https://doorway-b face=catching-up retryAfter=30' '${out}'"

# --- unreachable doorway -----------------------------------------------------
new_case down; HOSTS="https://doorway-a"
status="$(run)"
check "a doorway that does not answer is not ready (face=doorway-unreachable)" \
  "[ '${status}' = '3' ] && grep -q 'FLEET-NOT-READY https://doorway-a face=doorway-unreachable' '${log}'"

# --- unparseable / cannot judge => 2 ------------------------------------------
new_case junk; HOSTS="https://doorway-a"
fixture doorway-a health '200|this is not json|0|'
status="$(run)"
check "unparseable health body: exits 2 and names it" \
  "[ '${status}' = '2' ] && grep -q 'FLEET-READINESS-UNKNOWN https://doorway-a reason=health-unparseable' '${log}'"
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

# --- the shed-covered route (the household's declared shed and a real upstream
# shed both cover /db/*; /admin/* and /health/* are exempt, so a probe asking
# only those never sees a doorway turning deploys away) ------------------------
new_case readshed; HOSTS="https://doorway-a"
fixture doorway-a health "${OK_HEALTH}"
fixture doorway-a read '503|{"status":"catching-up","retryAfter":117,"cause":"dev-fixture"}|0|118'
fixture doorway-a put "${OK_PUT}"
status="$(run)"
check "shed read: a doorway shedding the content route is catching-up, advertised Retry-After first" \
  "[ '${status}' = '3' ] && grep -qx 'FLEET-NOT-READY https://doorway-a face=catching-up retryAfter=118' '${log}'"
check "shed read: refuses on that answer — no write probe" "[ \"\$(calls_to /admin/seed/blob)\" = '0' ]"
check "shed read: the read is a local-only GET (x-federation-hop: 1, no -X), bounded" \
  "grep -- '/db/content/' '${FAKE_DIR}/calls' | grep -q 'x-federation-hop: 1' && ! grep -- '/db/content/' '${FAKE_DIR}/calls' | grep -q -- '-X ' && grep -- '/db/content/' '${FAKE_DIR}/calls' | grep -q -- '--max-time'"
check "shed read: never sleeps" "never_slept"

new_case readcell; HOSTS="https://doorway-a"
fixture doorway-a health "${OK_HEALTH}"
fixture doorway-a read '503|{"error":"conductor call failed: CellDisabled(uhC0kabc)"}|0|15'
status="$(run)"
check "a CellDisabled answer on the content route is cell-not-running" \
  "[ '${status}' = '3' ] && grep -qx 'FLEET-NOT-READY https://doorway-a face=cell-not-running retryAfter=15' '${log}'"

new_case putcell; HOSTS="https://doorway-a"
fixture doorway-a health "${OK_HEALTH}"
fixture doorway-a put '502|{"success":false,"forwarded_to_storage":false,"error":"storage answered 503: CellDisabled"}|0|'
status="$(run)"
check "a CellDisabled answer on the write probe is cell-not-running, not a 5xx" \
  "[ '${status}' = '3' ] && grep -q 'FLEET-NOT-READY https://doorway-a face=cell-not-running' '${log}'"

# --- the host is the doorway's ORIGIN: two household doorways share localhost ---
new_case origin; HOSTS="http://localhost:8888/ http://localhost:8889"
fixture localhost:8888 health "${OK_HEALTH}"; fixture localhost:8888 put "${OK_PUT}"
fixture localhost:8889 health "${OK_HEALTH}"
fixture localhost:8889 read '503|{"status":"catching-up","retryAfter":40}|0|40'
status="$(run)"
check "origin: the ready doorway is named by scheme://host:port" \
  "[ '${status}' = '3' ] && grep -qx 'FLEET-READY http://localhost:8888' '${out}'"
check "origin: the not-ready doorway is named by ITS port, never a bare hostname" \
  "grep -qx 'FLEET-NOT-READY http://localhost:8889 face=catching-up retryAfter=40' '${out}' && ! grep -qE '^FLEET-[A-Z-]+ localhost' '${out}'"
new_case origin2; HOSTS="HTTPS://Doorway-A:443/api/"
# The fake keys a fixture by the host exactly as the URL spells it; the probe
# must still NAME it by its origin.
fixture Doorway-A:443 health "${OK_HEALTH}"; fixture Doorway-A:443 read "${OK_READ}"; fixture Doorway-A:443 put "${OK_PUT}"
status="$(run)"
check "origin: scheme and host lower-cased, default port and path dropped" \
  "[ '${status}' = '0' ] && grep -qx 'FLEET-READY https://doorway-a' '${out}'"

# --- ONE face vocabulary: scripts/ci/lib/readiness-faces.json ---------------------
faces_file="${here}/../lib/readiness-faces.json"
listed="$("${REAL_NODE}" -e 'const f=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8")).faces.map(x=>x.face);process.stdout.write([...f].sort().join(" "))' "${faces_file}" 2>/dev/null)"
listed_plan="$("${REAL_NODE}" -e 'const f=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8")).faces.filter(x=>x.plan===true).map(x=>x.face);process.stdout.write([...f].sort().join(" "))' "${faces_file}" 2>/dev/null)"
emitted="$( { grep -oE '"NOT [a-z0-9-]+"' "${script}" | sed 's/^"NOT //; s/"$//'; grep -oE "printf '(doorway-[a-z0-9-]+)'" "${script}" | sed "s/^printf '//; s/'\$//"; } | sort -u | tr '\n' ' ' | sed 's/ $//')"
stage_faces="$(sed -n '/^not_ready_face()/,/^}/p' "${here}/../stage-spa-blob.sh" | grep -oE "printf '[a-z0-9-]+'" | sed "s/^printf '//; s/'\$//" | sort -u | tr '\n' ' ' | sed 's/ $//')"
check "vocabulary: every face the probe can print is listed, and every listed face is one it can print" \
  "[ -n '${listed}' ] && [ '${emitted}' = '${listed}' ]"
check "vocabulary: the plan faces are exactly the three stage-spa-blob.sh names while it waits" \
  "[ '${listed_plan}' = 'catching-up cell-not-running storage-forward-timeout' ] && [ '${stage_faces}' = '${listed_plan}' ]"
check "vocabulary: no retired name survives in the probe" "! grep -q 'conductor-blind' '${script}'"

new_case unlisted; HOSTS="https://doorway-a"
fixture doorway-a health "${OK_HEALTH}"
fixture doorway-a read '503|{"status":"catching-up","retryAfter":30}|0|30'
narrow="${root}/narrow-faces.json"
printf '{"faces":[{"face":"shedding","plan":false}]}' > "${narrow}"
status="$(run READINESS_FACES_FILE="${narrow}")"
check "vocabulary is read at run time: a face the list does not name is never printed (exit 2, reason named)" \
  "[ '${status}' = '2' ] && grep -qx 'FLEET-READINESS-UNKNOWN https://doorway-a reason=unlisted-face-catching-up' '${log}' && ! grep -q 'FLEET-NOT-READY' '${log}'"
new_case nofaces; HOSTS="https://doorway-a"
status="$(run READINESS_FACES_FILE="${root}/no-such-faces.json")"
check "a missing face list cannot judge anything (exit 2, no request made)" \
  "[ '${status}' = '2' ] && [ ! -s '${FAKE_DIR}/calls' ]"

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
  "[ \"\$(q o.commit)\" = 'abc123' ] && [ \"\$(q o.env)\" = 'dev' ] && [ \"\$(q o.doorway)\" = 'https://doorway-b' ] && [ \"\$(q o.face)\" = 'shedding' ] && [ \"\$(q o.retryAfter)\" = '90' ]"
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
