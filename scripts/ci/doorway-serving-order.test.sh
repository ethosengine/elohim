#!/usr/bin/env bash
# Hermetic regression test for the serving-first author order (app #1725,
# 2026-09-23: the readiness wait sat ~53 min on a doorway that could not author
# while the other one could). No network: curl is a stub executable in a temp
# dir, wired in through SERVING_ORDER_FETCH_CMD; node is the real one, because
# the JSON classification is the thing under test.
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
# written to the -o file; <host>.exit makes the call fail with that status and
# write nothing (a transport failure). Logs every call.
cat > "${BIN}/curl" <<'EOF'
#!/usr/bin/env bash
echo "curl $*" >> "${KSTATE}/calls"
out=""; url=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    -o|-m) [ "$1" = "-o" ] && out="$2"; shift 2 ;;
    -*) shift ;;
    *) url="$1"; shift ;;
  esac
done
host="${url#*://}"; host="${host%%/*}"
[ -f "${KSTATE}/${host}.exit" ] && { echo "curl: (7) Failed to connect to ${host}" >&2; exit "$(cat "${KSTATE}/${host}.exit")"; }
[ -f "${KSTATE}/${host}.body" ] && cat "${KSTATE}/${host}.body" > "${out}"
exit 0
EOF
chmod +x "${BIN}/curl"
export SERVING_ORDER_FETCH_CMD="${BIN}/curl"
reset() { rm -rf "${T}/k"; mkdir -p "${T}/k"; export KSTATE="${T}/k"; }
# Bodies in the shape doorway-service routes/health.rs serializes (ServingHealth).
body() { # body <shedding> <storageServing.status>
  printf '{"shedding":%s,"degrading":false,"upstreams":[{"endpoint":"http://s:8090","circuit":"closed","errorStreak":0,"shedding":false,"role":"primary"}],"rolesDiscovered":5,"warmupEmpty":false,"storageServing":{"endpoint":"http://s:8090","status":"%s","httpStatus":200}}' "$1" "$2"
}
order() { bash "${ORDER}" "$@" 2>"${T}/err"; }
expect() { # expect <label> <want> <got>
  [ "$3" = "$2" ] || fail "$1: expected $(printf '%q' "$2"), got $(printf '%q' "$3")"
}

# ─── 1. both serving → order unchanged ──────────────────────────────────────
reset
body false serving > "${KSTATE}/alpha.elohim.host.body"
body false serving > "${KSTATE}/elohim.host.body"
expect "both serving" "${A}"$'\n'"${B}" "$(order "${A}" "${B}")"
grep -q "alpha.elohim.host/health/serving" "${KSTATE}/calls" || fail "must probe /health/serving on the first host"
grep -q "elohim.host/health/serving" "${KSTATE}/calls" || fail "must probe /health/serving on the second host"
grep -q -- "-m 10" "${KSTATE}/calls" || fail "the probe must be bounded (-m 10)"
grep -q "${A} serving" "${T}/err" && grep -q "${B} serving" "${T}/err" || fail "diagnostics must name each serving host"
pass "both serving: canonical order kept, both hosts probed once, bounded"

# ─── 2. first refused, second serving → second first ────────────────────────
reset
body false refused > "${KSTATE}/alpha.elohim.host.body"
body false serving > "${KSTATE}/elohim.host.body"
expect "first refused" "${B}"$'\n'"${A}" "$(order "${A}" "${B}")"
grep -q "${A} not serving (shedding=false storageServing=\"refused\")" "${T}/err" || fail "the refusing host must be named with its verdict"
pass "first refused, second serving: the serving host is offered first"

body true serving > "${KSTATE}/alpha.elohim.host.body"
expect "first shedding" "${B}"$'\n'"${A}" "$(order "${A}" "${B}")"
pass "a shedding doorway sorts after a serving one even when its storage serves"

# ─── 3. both not serving → original order ───────────────────────────────────
reset
body true serving > "${KSTATE}/alpha.elohim.host.body"
body false unreachable > "${KSTATE}/elohim.host.body"
expect "none serving" "${A}"$'\n'"${B}" "$(order "${A}" "${B}")"
pass "no host serving: the input order comes back unchanged"

# ─── 4. curl failure = not serving ──────────────────────────────────────────
reset
echo 7 > "${KSTATE}/alpha.elohim.host.exit"
body false serving > "${KSTATE}/elohim.host.body"
set +e; out="$(order "${A}" "${B}")"; rc=$?; set -e
[ "${rc}" -eq 0 ] || fail "a failed probe must not fail the script (rc=${rc})"
expect "curl failure" "${B}"$'\n'"${A}" "${out}"
grep -q "${A} not serving — fetch failed (exit 7" "${T}/err" || fail "a transport failure must be named as such, not as the doorway's answer"
pass "curl failure: treated as not serving, exit 0, reason on stderr"

# ─── 5. malformed / wrongly-typed bodies = not serving ──────────────────────
reset
printf '{"shedding":false,"storageServing":{"status":"serv' > "${KSTATE}/alpha.elohim.host.body"
body false serving > "${KSTATE}/elohim.host.body"
expect "malformed JSON" "${B}"$'\n'"${A}" "$(order "${A}" "${B}")"
grep -q "${A} not serving (body is not JSON)" "${T}/err" || fail "malformed JSON must be named as not JSON"
printf '{"shedding":"false","storageServing":{"status":"serving"}}' > "${KSTATE}/alpha.elohim.host.body"
expect "string false" "${B}"$'\n'"${A}" "$(order "${A}" "${B}")"
printf '{"shedding":false}' > "${KSTATE}/alpha.elohim.host.body"
expect "no storageServing" "${B}"$'\n'"${A}" "$(order "${A}" "${B}")"
: > "${KSTATE}/alpha.elohim.host.body"
expect "empty body" "${B}"$'\n'"${A}" "$(order "${A}" "${B}")"
printf '[{"shedding":false}]' > "${KSTATE}/alpha.elohim.host.body"
expect "array root" "${B}"$'\n'"${A}" "$(order "${A}" "${B}")"
pass "malformed JSON, string-typed shedding, missing block, empty and array bodies all read not serving"

# ─── 6. contract: stdout is only the URLs; exit 0 always; no args → nothing ──
reset
body false serving > "${KSTATE}/elohim.host.body"
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
