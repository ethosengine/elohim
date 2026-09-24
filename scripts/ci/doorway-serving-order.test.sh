#!/usr/bin/env bash
# Hermetic regression test for the serving-first author order (app #1725,
# 2026-09-23: the run's one 7200 s readiness deadline was spent on alpha while
# elohim.host was never asked while there was time; offered the head only after
# the clock was gone, it answered 503 catching-up once — circuit closed,
# errorStreak 1, the DEGRADING arm — and every bundle read "NO doorway could
# author"). No network: curl is a stub executable in a temp dir, wired in
# through SERVING_ORDER_FETCH_CMD; node is the real one, because the JSON
# classification is the thing under test.
#
#   scripts/ci/doorway-serving-order.sh   serving hosts first, exit 0 always
#   Jenkinsfile                           authorHeadOnce iterates that order
#
# Run: bash scripts/ci/doorway-serving-order.test.sh
set -euo pipefail

REPO_ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
CI="${REPO_ROOT}/scripts/ci"
T="$(mktemp -d)"
trap 'rm -rf "${T}"' EXIT
fail() { echo "FAIL: $*" >&2; exit 1; }
pass() { echo "ok - $*"; }
ORDER="${CI}/doorway-serving-order.sh"
A="https://alpha.elohim.host"
B="https://elohim.host"

# ─── stub ───────────────────────────────────────────────────────────────────
BIN="${T}/bin"; mkdir -p "${BIN}"
# curl stub: answers from files under $KSTATE keyed by host — <host>.body is
# written to the -o file; <host>.code (default 200) is printed to stdout when
# -w '%{http_code}' is asked for, exactly as curl does; <host>.exit makes the
# call fail with that status, print 000 like curl, and write nothing (a
# transport failure). Logs every call.
cat > "${BIN}/curl" <<'STUB'
#!/usr/bin/env bash
echo "curl $*" >> "${KSTATE}/calls"
out=""; url=""; fmt=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    -o) out="$2"; shift 2 ;;
    -w) fmt="$2"; shift 2 ;;
    -m) shift 2 ;;
    -*) shift ;;
    *) url="$1"; shift ;;
  esac
done
host="${url#*://}"; host="${host%%/*}"
if [ -f "${KSTATE}/${host}.exit" ]; then
  echo "curl: (7) Failed to connect to ${host}" >&2
  [ "${fmt}" = '%{http_code}' ] && printf '000'
  exit "$(cat "${KSTATE}/${host}.exit")"
fi
[ -f "${KSTATE}/${host}.body" ] && cat "${KSTATE}/${host}.body" > "${out}"
code=200; [ -f "${KSTATE}/${host}.code" ] && code="$(cat "${KSTATE}/${host}.code")"
[ "${fmt}" = '%{http_code}' ] && printf '%s\n' "${code}"
exit 0
STUB
chmod +x "${BIN}/curl"
export SERVING_ORDER_FETCH_CMD="${BIN}/curl"
reset() { rm -rf "${T}/k"; mkdir -p "${T}/k"; export KSTATE="${T}/k"; }
# Bodies in the shape doorway-service routes/health.rs serializes (ServingHealth).
# body5 spells out every field the route's 503 predicate reads; body is the
# healthy-doorway shorthand (degrading false, 5 roles, warmup stocked).
body5() { # body5 <shedding> <degrading> <rolesDiscovered> <warmupEmpty> <storageServing.status>
  printf '{"shedding":%s,"degrading":%s,"upstreams":[{"endpoint":"http://s:8090","circuit":"closed","errorStreak":0,"shedding":false,"role":"primary"}],"rolesDiscovered":%s,"warmupEmpty":%s,"storageServing":{"endpoint":"http://s:8090","status":"%s","httpStatus":200}}' "$1" "$2" "$3" "$4" "$5"
}
body() { body5 "$1" false 5 false "$2"; } # body <shedding> <storageServing.status>
# The doorway's own status for a body: 503 on any of the five arms, as
# build_serving_response decides it (not-observed/not-configured do NOT 503).
answer() { # answer <host> <http-code> <body-json>
  printf '%s' "$2" > "${KSTATE}/$1.code"
  printf '%s' "$3" > "${KSTATE}/$1.body"
}
order() { bash "${ORDER}" "$@" 2>"${T}/err"; }
expect() { # expect <label> <want> <got>
  [ "$3" = "$2" ] || fail "$1: expected $(printf '%q' "$2"), got $(printf '%q' "$3")"
}
HEALTHY='shedding=false degrading=false rolesDiscovered=5 warmupEmpty=false'

# ─── 1. both serving → order unchanged ──────────────────────────────────────
reset
answer alpha.elohim.host 200 "$(body false serving)"
answer elohim.host 200 "$(body false serving)"
expect "both serving" "${A}"$'\n'"${B}" "$(order "${A}" "${B}")"
grep -q "alpha.elohim.host/health/serving" "${KSTATE}/calls" || fail "must probe /health/serving on the first host"
grep -q "elohim.host/health/serving" "${KSTATE}/calls" || fail "must probe /health/serving on the second host"
grep -q -- "-m 10" "${KSTATE}/calls" || fail "the probe must be bounded (-m 10)"
grep -q -- "-w %{http_code}" "${KSTATE}/calls" || fail "the probe must ask curl for the HTTP status (-w %{http_code}) — the status IS the doorway's verdict"
grep -q "${A} serving (http=200 ${HEALTHY} storageServing=\"serving\")" "${T}/err" || fail "a serving host's diagnostic must carry the status and every predicate field: $(cat "${T}/err")"
grep -q "${B} serving (http=200" "${T}/err" || fail "diagnostics must name each serving host"
pass "both serving: canonical order kept, both hosts probed once, bounded, status asked for"

# ─── 2. first refused (503), second serving → second first ──────────────────
reset
answer alpha.elohim.host 503 "$(body false refused)"
answer elohim.host 200 "$(body false serving)"
expect "first refused" "${B}"$'\n'"${A}" "$(order "${A}" "${B}")"
grep -q "${A} not serving (http=503 ${HEALTHY} storageServing=\"refused\")" "${T}/err" || fail "the refusing host must be named with its status and verdict: $(cat "${T}/err")"
pass "first refused, second serving: the serving host is offered first"

answer alpha.elohim.host 503 "$(body true serving)"
expect "first shedding" "${B}"$'\n'"${A}" "$(order "${A}" "${B}")"
pass "a shedding doorway sorts after a serving one even when its storage serves"

# ─── 2b. the doorway's OWN 503 verdict wins over a body that looks serving ──
# The three arms the body check alone cannot see: degrading (a non-zero error
# streak with the circuit still closed — elohim.host's ~23:42Z answer in #1725),
# conductor-blind (rolesDiscovered 0 — the 2026-08-21 regime), warmupEmpty.
# In each the body reads shedding=false AND storageServing=serving.
reset
answer alpha.elohim.host 503 "$(body5 false true 5 false serving)"
answer elohim.host 200 "$(body false serving)"
expect "degrading 503" "${B}"$'\n'"${A}" "$(order "${A}" "${B}")"
grep -q "${A} not serving (http=503 shedding=false degrading=true rolesDiscovered=5 warmupEmpty=false storageServing=\"serving\")" "${T}/err" || fail "a degrading doorway must be named by its status AND the degrading field: $(cat "${T}/err")"
answer alpha.elohim.host 503 "$(body5 false false 0 false serving)"
expect "conductor-blind 503" "${B}"$'\n'"${A}" "$(order "${A}" "${B}")"
grep -q "${A} not serving (http=503 shedding=false degrading=false rolesDiscovered=0" "${T}/err" || fail "a conductor-blind doorway must be named with rolesDiscovered=0: $(cat "${T}/err")"
answer alpha.elohim.host 503 "$(body5 false false 5 true serving)"
expect "warmupEmpty 503" "${B}"$'\n'"${A}" "$(order "${A}" "${B}")"
grep -q "${A} not serving (http=503 shedding=false degrading=false rolesDiscovered=5 warmupEmpty=true" "${T}/err" || fail "an empty-warmup doorway must be named with warmupEmpty=true: $(cat "${T}/err")"
# A 200 whose body says shedding=false + serving with the optional fields
# ABSENT (an older doorway, or a non-writer) is still serving — the status is
# the doorway's verdict; the body check is only the two author-leg fields.
answer alpha.elohim.host 200 '{"shedding":false,"degrading":false,"upstreams":[],"storageServing":{"status":"serving"}}'
expect "200 without optional fields" "${A}"$'\n'"${B}" "$(order "${A}" "${B}")"
grep -q "${A} serving (http=200 shedding=false degrading=false rolesDiscovered=undefined warmupEmpty=undefined storageServing=\"serving\")" "${T}/err" || fail "absent optional fields must read undefined, not block a 200: $(cat "${T}/err")"
pass "the doorway's own 503 (degrading / conductor-blind / warmupEmpty) sorts a serving-looking body last; a 200 without optional fields still serves"

# ─── 2c. and the body still matters under a 200 ─────────────────────────────
# not-observed / not-configured do not 503 the route (blocks_serving is
# refused|unreachable only) but cannot anchor a head; a 200 with them sorts last.
reset
answer alpha.elohim.host 200 "$(body false not-configured)"
answer elohim.host 200 "$(body false serving)"
expect "200 not-configured" "${B}"$'\n'"${A}" "$(order "${A}" "${B}")"
grep -q "${A} not serving (http=200 ${HEALTHY} storageServing=\"not-configured\")" "${T}/err" || fail "a 200 with no anchoring storage must be named as such: $(cat "${T}/err")"
answer alpha.elohim.host 200 "$(body false not-observed)"
expect "200 not-observed" "${B}"$'\n'"${A}" "$(order "${A}" "${B}")"
pass "a 200 whose storage is not-configured / not-observed still sorts after a serving host"

# ─── 3. both not serving → original order ───────────────────────────────────
reset
answer alpha.elohim.host 503 "$(body true serving)"
answer elohim.host 503 "$(body false unreachable)"
expect "none serving" "${A}"$'\n'"${B}" "$(order "${A}" "${B}")"
answer alpha.elohim.host 503 "$(body5 false true 5 false serving)"
answer elohim.host 503 "$(body5 false true 5 false serving)"
expect "both degrading" "${A}"$'\n'"${B}" "$(order "${A}" "${B}")"
pass "no host serving (by body, or by the doorway's own 503 alone): the input order comes back unchanged"

# ─── 4. curl failure = not serving ──────────────────────────────────────────
reset
echo 7 > "${KSTATE}/alpha.elohim.host.exit"
answer elohim.host 200 "$(body false serving)"
set +e; out="$(order "${A}" "${B}")"; rc=$?; set -e
[ "${rc}" -eq 0 ] || fail "a failed probe must not fail the script (rc=${rc})"
expect "curl failure" "${B}"$'\n'"${A}" "${out}"
grep -q "${A} not serving — fetch failed (exit 7" "${T}/err" || fail "a transport failure must be named as such, not as the doorway's answer"
pass "curl failure: treated as not serving, exit 0, reason on stderr"

# ─── 5. malformed / wrongly-typed bodies, non-200 non-JSON = not serving ────
reset
answer alpha.elohim.host 200 '{"shedding":false,"storageServing":{"status":"serv'
answer elohim.host 200 "$(body false serving)"
expect "malformed JSON" "${B}"$'\n'"${A}" "$(order "${A}" "${B}")"
grep -q "${A} not serving (http=200 body is not JSON)" "${T}/err" || fail "malformed JSON must be named as not JSON with its status: $(cat "${T}/err")"
answer alpha.elohim.host 200 '{"shedding":"false","storageServing":{"status":"serving"}}'
expect "string false" "${B}"$'\n'"${A}" "$(order "${A}" "${B}")"
answer alpha.elohim.host 200 '{"shedding":false}'
expect "no storageServing" "${B}"$'\n'"${A}" "$(order "${A}" "${B}")"
answer alpha.elohim.host 200 ''
expect "empty body" "${B}"$'\n'"${A}" "$(order "${A}" "${B}")"
answer alpha.elohim.host 200 '[{"shedding":false}]'
expect "array root" "${B}"$'\n'"${A}" "$(order "${A}" "${B}")"
answer alpha.elohim.host 404 '<html>404 page not found</html>'
expect "404 html" "${B}"$'\n'"${A}" "$(order "${A}" "${B}")"
grep -q "${A} not serving (http=404 body is not JSON)" "${T}/err" || fail "an image predating the route must be named by its 404: $(cat "${T}/err")"
answer alpha.elohim.host 502 '<html>502 Bad Gateway</html>'
expect "502 html" "${B}"$'\n'"${A}" "$(order "${A}" "${B}")"
pass "malformed JSON, string-typed shedding, missing block, empty and array bodies, 404/502 pages all read not serving"

# ─── 6. contract: stdout is only the URLs; exit 0 always; no args → nothing ──
reset
answer elohim.host 200 "$(body false serving)"
out="$(order "${B}" "${A}")"
[ "$(printf '%s\n' "${out}" | wc -l)" -eq 2 ] || fail "stdout must carry exactly the input URLs"
grep -qv '^https://' <<< "${out}" && fail "stdout must carry nothing but URLs (diagnostics belong on stderr)"
[ -s "${T}/err" ] || fail "diagnostics must reach stderr"
set +e; out="$(bash "${ORDER}" 2>/dev/null)"; rc=$?; set -e
[ "${rc}" -eq 0 ] && [ -z "${out}" ] || fail "no arguments must print nothing and exit 0 (rc=${rc} out='${out}')"
pass "contract: URLs-only stdout, diagnostics on stderr, exit 0, empty for no input"

# ─── 7. Jenkinsfile wiring ──────────────────────────────────────────────────
JF="${REPO_ROOT}/Jenkinsfile"
AUTHOR="$(awk '/^def authorHeadOnce\(/{p=1} /^def verifyEprMounts\(/{p=0} p' "${JF}")"
[ -n "${AUTHOR}" ] || fail "authorHeadOnce not found in the root Jenkinsfile"
grep -q "scripts/ci/doorway-serving-order.sh" <<< "${AUTHOR}" || fail "authorHeadOnce must compute the serving-first order"
grep -q "for (int i = 0; i < authorOrder.size(); i++)" <<< "${AUTHOR}" || fail "the AUTHOR loop must iterate the serving-first order"
grep -q "if (authorOrder.size() != doorwayEprUrls.size()) { authorOrder = doorwayEprUrls }" <<< "${AUTHOR}" || fail "an empty or partial order must fall back to the canonical list"
grep -q "for (int j = 0; j < doorwayEprUrls.size(); j++)" <<< "${AUTHOR}" || fail "the DECLARE_ONLY fan-out must keep the canonical list"
grep -q "if (j == i) { continue }" <<< "${AUTHOR}" && fail "the fan-out still skips by canonical index — that no longer names the authoring host"
grep -q "if (doorwayEprUrls\[j\] == doorwayEprUrl) { continue }" <<< "${AUTHOR}" || fail "the fan-out must skip the authoring host by URL"
grep -q "verifyProjectedHeads(doorwayEprUrls, bundles, gitCommitHash, outcomes)" "${JF}" || fail "verifyProjectedHeads must keep the canonical list"
grep -qE "<<-?'?[A-Z]+'?$" <<< "${AUTHOR}" && fail "authorHeadOnce must stay heredoc-free (CPS 64 KB)"
pass "Jenkinsfile wiring: author loop reordered, fan-out + probes canonical, heredoc-free"

echo "ALL PASS"
