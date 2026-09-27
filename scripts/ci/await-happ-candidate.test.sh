#!/usr/bin/env bash
# Hermetic test for await-happ-candidate.sh — an `oras` shim on PATH stands in
# for the registry. The shim answers `manifest fetch` from FAKE_ORAS_PRESENT_AFTER:
# the number of fetches that fail before the tag "appears" (-1 = never).
set -euo pipefail

REPO_ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
SCRIPT="${REPO_ROOT}/scripts/ci/await-happ-candidate.sh"
TEST_ROOT="$(mktemp -d)"
trap 'rm -rf "${TEST_ROOT}"' EXIT

mkdir -p "${TEST_ROOT}/bin"
cat > "${TEST_ROOT}/bin/oras" <<'SHIM'
#!/usr/bin/env bash
printf '%s\n' "$*" >> "${FAKE_ORAS_CALLS}"
case "$1 ${2:-}" in
  'manifest fetch')
    n=$(grep -c '^manifest fetch' "${FAKE_ORAS_CALLS}")
    [ "${FAKE_ORAS_PRESENT_AFTER}" -ge 0 ] && [ "${n}" -gt "${FAKE_ORAS_PRESENT_AFTER}" ] && exit 0
    echo "Error: not found" >&2; exit 1 ;;
  'login '*) exit 0 ;;
esac
exit 0
SHIM
chmod +x "${TEST_ROOT}/bin/oras"
export PATH="${TEST_ROOT}/bin:${PATH}"
export FAKE_ORAS_CALLS="${TEST_ROOT}/oras.calls"
unset HARBOR_USER HARBOR_PASS || true

fail() { echo "FAIL: $*" >&2; exit 1; }
run() { # run <present-after> <deadline> [poll] → sets RC, OUT, ELAPSED
  : > "${FAKE_ORAS_CALLS}"
  local start=${SECONDS}
  set +e
  OUT="$(FAKE_ORAS_PRESENT_AFTER="$1" AWAIT_POLL_SECS="${3:-15}" bash "${SCRIPT}" candidate-0123abcd "$2" 2>&1)"
  RC=$?
  set -e
  ELAPSED=$((SECONDS - start))
}

# (a) the candidate is already there: exit 0 on the first poll, no sleeping.
run 0 60
[ "${RC}" -eq 0 ] || fail "(a) present tag exited ${RC}: ${OUT}"
[ "${ELAPSED}" -le 2 ] || fail "(a) present tag took ${ELAPSED}s — must return within one poll"
grep -q 'manifest fetch harbor.ethosengine.com/ethosengine/elohim-happ:candidate-0123abcd' "${FAKE_ORAS_CALLS}" \
  || fail "(a) did not fetch the exact candidate reference: $(cat "${FAKE_ORAS_CALLS}")"
echo "ok (a) present candidate → exit 0 within one poll"

# (b) never appears: exit 3 with the named line, never 0 or 1.
run -1 1 1
[ "${RC}" -eq 3 ] || fail "(b) absent tag exited ${RC} (want 3): ${OUT}"
grep -Eq '^HAPP-CANDIDATE-ABSENT tag=candidate-0123abcd after [0-9]+s$' <<< "${OUT}" \
  || fail "(b) missing HAPP-CANDIDATE-ABSENT line: ${OUT}"
echo "ok (b) absent candidate → exit 3 HAPP-CANDIDATE-ABSENT"

# (c) the deadline is honoured even when the poll interval is longer than it.
run -1 2 15
[ "${RC}" -eq 3 ] || fail "(c) exited ${RC} (want 3): ${OUT}"
[ "${ELAPSED}" -lt 5 ] || fail "(c) a 2s deadline took ${ELAPSED}s"
echo "ok (c) 2s deadline ends in ${ELAPSED}s"

# (d) appears on a later poll: exit 0 after polling again.
run 2 30 1
[ "${RC}" -eq 0 ] || fail "(d) late tag exited ${RC}: ${OUT}"
[ "$(grep -c '^manifest fetch' "${FAKE_ORAS_CALLS}")" -eq 3 ] || fail "(d) want 3 fetches: $(cat "${FAKE_ORAS_CALLS}")"
echo "ok (d) candidate published mid-wait → exit 0 on the poll that sees it"

# (e) a floating tag is refused: the awaiter names one commit's bytes, never a moving tag.
for t in dev-latest latest ''; do
  set +e; OUT="$(bash "${SCRIPT}" "${t}" 5 2>&1)"; RC=$?; set -e
  [ "${RC}" -eq 2 ] || fail "(e) tag '${t}' exited ${RC} (want 2 usage refusal): ${OUT}"
done
echo "ok (e) floating/empty tags refused with exit 2"

echo "PASS await-happ-candidate.test.sh"
