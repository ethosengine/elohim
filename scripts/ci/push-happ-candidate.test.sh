#!/usr/bin/env bash
# Hermetic test for push-happ-candidate.sh — an `oras` shim on PATH records
# every call. The candidate publish must name ONE commit's bytes: it never
# names a floating tag, pushes exactly once, and carries the content roll key
# when happ-roll-key.sh resolves one.
set -euo pipefail

REPO_ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
SCRIPT="${REPO_ROOT}/scripts/ci/push-happ-candidate.sh"
TEST_ROOT="$(mktemp -d)"
trap 'rm -r "${TEST_ROOT}"' EXIT

fail() { echo "FAIL: $*" >&2; exit 1; }

# 0. the script never names a floating tag — not in code, not in comments.
[ -f "${SCRIPT}" ] || fail "missing ${SCRIPT}"
if grep -nE 'dev-latest|(^|[^a-z-])latest' "${SCRIPT}"; then
  fail "push-happ-candidate.sh names a floating tag"
fi
echo "ok 0 no floating tag named"

mkdir -p "${TEST_ROOT}/bin"
cat > "${TEST_ROOT}/bin/oras" <<'SHIM'
#!/usr/bin/env bash
printf '%s\n' "$*" >> "${FAKE_ORAS_CALLS}"
exit 0
SHIM
chmod +x "${TEST_ROOT}/bin/oras"
export PATH="${TEST_ROOT}/bin:${PATH}"
export FAKE_ORAS_CALLS="${TEST_ROOT}/oras.calls"

# A DNA build directory as `Build DNA` leaves it.
BUILD="${TEST_ROOT}/dna"
mkdir -p "${BUILD}/workdir"
printf 'lamad=uhC0kAAAA\nimagodei=uhC0kBBBB\n' > "${BUILD}/dna-hashes.actual"
printf 'manifest_version: "1"\nname: elohim\n' > "${BUILD}/workdir/happ.yaml"
printf 'happ-bytes' > "${BUILD}/elohim.happ"

run() { # run <commit> → RC, OUT
  : > "${FAKE_ORAS_CALLS}"
  set +e
  OUT="$(cd "${BUILD}" && HARBOR_USER=robot HARBOR_PASS=secret bash "${SCRIPT}" "$1" 2>&1)"
  RC=$?
  set -e
}

# 1. roll key resolves: one push, exact candidate ref, roll-key annotation.
run 0123abcd
[ "${RC}" -eq 0 ] || fail "(1) exited ${RC}: ${OUT}"
[ "$(grep -c '^push ' "${FAKE_ORAS_CALLS}")" -eq 1 ] || fail "(1) want exactly one push: $(cat "${FAKE_ORAS_CALLS}")"
PUSH="$(grep '^push ' "${FAKE_ORAS_CALLS}")"
grep -q '^push harbor.ethosengine.com/ethosengine/elohim-happ:candidate-0123abcd ' <<< "${PUSH}" \
  || fail "(1) wrong ref: ${PUSH}"
grep -q -- '--annotation elohim.host/roll-key=bundle:sha256:' <<< "${PUSH}" \
  || fail "(1) missing roll-key annotation (no coordswap yet → bundle key): ${PUSH}"
grep -q 'elohim.happ:application/octet-stream$' <<< "${PUSH}" || fail "(1) wrong layer: ${PUSH}"
grep -q '^login harbor.ethosengine.com -u robot --password-stdin' "${FAKE_ORAS_CALLS}" \
  || fail "(1) login must read the password from stdin: $(cat "${FAKE_ORAS_CALLS}")"
grep -q 'secret' "${FAKE_ORAS_CALLS}" && fail "(1) the password reached argv"
echo "ok 1 one push of candidate-<commit> with the roll-key annotation"

# 2. roll key does not resolve: still one push, no annotation.
: > "${BUILD}/dna-hashes.actual"
run 0123abcd
[ "${RC}" -eq 0 ] || fail "(2) exited ${RC}: ${OUT}"
[ "$(grep -c '^push ' "${FAKE_ORAS_CALLS}")" -eq 1 ] || fail "(2) want exactly one push"
grep '^push ' "${FAKE_ORAS_CALLS}" | grep -q -- '--annotation' && fail "(2) annotation without a key"
echo "ok 2 unresolved roll key → one push, no annotation"

# 3. a non-commit argument is refused before any push.
for bad in '' 'dev' 'candidate-0123abcd' '0123ABCD' '01'; do
  run "${bad}"
  [ "${RC}" -eq 2 ] || fail "(3) '${bad}' exited ${RC} (want 2): ${OUT}"
  grep -q '^push ' "${FAKE_ORAS_CALLS}" && fail "(3) '${bad}' pushed"
done
echo "ok 3 non-commit arguments refused with exit 2, nothing pushed"

echo "PASS push-happ-candidate.test.sh"
