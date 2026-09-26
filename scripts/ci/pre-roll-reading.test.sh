#!/usr/bin/env bash
# Hermetic test for scripts/ci/pre-roll-reading.sh — the pre-deploy reading the
# roll gate's settled-at-baseline branch compares against. Fake curl (an
# exported bash function, the sequenced-roll.test.sh pattern); no network.
#
#   1. a usable projectionReconcile APPENDS PRE_DIVERGENT_/PRE_HEALED_ lines
#   2. unreachable, no usable block, or an UNMEASURED sweep (peersAsked 0)
#      writes NOTHING and says so
#   3. the edge Jenkinsfile takes the reading before the deploy touches a peer
set -euo pipefail

REPO_ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
SCRIPT="${REPO_ROOT}/scripts/ci/pre-roll-reading.sh"
JENKINSFILE="${REPO_ROOT}/elohim/holochain/Jenkinsfile"
TEST_ROOT="$(mktemp -d)"

cleanup() { rm -rf "${TEST_ROOT}"; }
trap cleanup EXIT

fail() { echo "FAIL: $*" >&2; exit 1; }

curl() {
  case "$*" in
    *'/p2p/status'*)
      if [ -f "${FAKE_BODY}" ]; then cat "${FAKE_BODY}"; else return 7; fi
      ;;
    *) echo "unexpected curl invocation: $*" >&2; return 97 ;;
  esac
}
export -f curl
export FAKE_BODY="${TEST_ROOT}/status.json"

# ── 1. a usable reading appends both lines, keeping what is already there ───
STATE="${TEST_ROOT}/state"
printf 'PRE_DIVERGENT_jessica=3\nPRE_HEALED_jessica=1\n' > "${STATE}"
printf '{"peerId":"x","projectionReconcile":{"pending":0,"caughtUp":false,"peersAsked":4,"divergentAnchor":55,"healedTotal":812,"sweeps":40,"converged":false},"other":{"divergentAnchor":999}}' \
  > "${FAKE_BODY}"
OUT="$(bash "${SCRIPT}" james http://james:8090/ "${STATE}")" || fail "a usable reading must exit 0"
EXPECTED='PRE_DIVERGENT_jessica=3
PRE_HEALED_jessica=1
PRE_DIVERGENT_james=55
PRE_HEALED_james=812'
[ "$(cat "${STATE}")" = "${EXPECTED}" ] || { cat "${STATE}"; fail 'reading not appended as PRE_DIVERGENT_/PRE_HEALED_ lines (or read outside projectionReconcile)'; }
printf '%s\n' "${OUT}" | grep -q 'pre-roll-reading\[james\]: divergentAnchor=55 healedTotal=812 recorded' \
  || fail "reading not announced: ${OUT}"

# ── 2. unreachable / no usable block → writes nothing and says so ───────────
for shape in unreachable null-block missing-counter unmeasured; do
  STATE="${TEST_ROOT}/state-${shape}"
  case "${shape}" in
    unreachable) rm -f "${FAKE_BODY}" ;;
    null-block) printf '{"peerId":"x","projectionReconcile":null}' > "${FAKE_BODY}" ;;
    missing-counter) printf '{"projectionReconcile":{"peersAsked":2,"divergentAnchor":4,"sweeps":2}}' > "${FAKE_BODY}" ;;
    # db unavailable: sweep published with divergentAnchor 0 and nobody asked
    unmeasured) printf '{"projectionReconcile":{"peersAsked":0,"divergentAnchor":0,"healedTotal":9,"sweeps":2}}' > "${FAKE_BODY}" ;;
  esac
  set +e
  OUT="$(bash "${SCRIPT}" susan http://susan:8090 "${STATE}")"
  RC=$?
  set -e
  [ "${RC}" -eq 0 ] || fail "${shape}: an absent reading must never fail the deploy (got ${RC})"
  [ ! -e "${STATE}" ] || fail "${shape}: an absent reading wrote to the state file"
  printf '%s\n' "${OUT}" | grep -q 'pre-roll-reading\[susan\]: no usable projectionReconcile .* writing nothing' \
    || fail "${shape}: absent reading was not announced: ${OUT}"
done

# ── 3. Jenkins wiring: read before the deploy touches the peer, state kept ──
grep -Fq 'def recordPreRollReading(Map humanConfig, String stateFile)' "${JENKINSFILE}" \
  || fail 'Jenkinsfile lacks the recordPreRollReading helper'
grep -Fq "scripts/ci/pre-roll-reading.sh" "${JENKINSFILE}" || fail 'Jenkinsfile does not call pre-roll-reading.sh'
# Order inside deployHumansInParallel: state reset, then the per-peer reading
# ahead of deployHumanManifest (storage apply/restart), then the gate.
ORDER="$(awk '/^def deployHumansInParallel\(/{f=1} f&&/^}/{exit} f' "${JENKINSFILE}" \
  | grep -nE "rm -f '\\\$\{stateFile\}'|recordPreRollReading\(hc, stateFile\)|deployHumanManifest\(hc,|runPeerRollGate\(" \
  | sed -E 's/^([0-9]+):.*(rm -f|recordPreRollReading|deployHumanManifest|runPeerRollGate).*/\2/' | tr '\n' ' ')"
[ "${ORDER}" = "rm -f recordPreRollReading deployHumanManifest runPeerRollGate " ] \
  || fail "pre-roll wiring order drifted: '${ORDER}' (want: rm -f recordPreRollReading deployHumanManifest runPeerRollGate)"

echo 'pre-roll-reading: reading appended, absent reading writes nothing, Jenkins wiring ordered'
