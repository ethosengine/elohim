#!/usr/bin/env bash
# Regression coverage for the settled-at-baseline branch of peer-roll-gate.sh
# (edge #1486: four of six peers burned 450-599s on `no-movement(healed 0->0,
# divergentAnchor 55->55)` — a standing divergence carried INTO the roll).
#
# HOST-SIDE TEST ONLY — never runs in the deploy container (the scripts/ci
# .epr-meta bash+coreutils rule governs deploy-path scripts; the gate itself
# is exercised here from the host). A real local python3 http.server plays each peer's storage, serving a
# scripted sequence of /p2p/status bodies per scenario (the last body repeats).
# Prometheus is a refused loopback port, so the throttle leg runs DEGRADED and
# the gate decides on the convergence leg alone.
#
#   a. converged:true on a newer sweep      → exit 0, converged (unchanged)
#   b. divergentAnchor 41,41,41, PRE=41     → exit 0, settled-at-baseline
#   c. divergentAnchor 41,41,41, PRE=30     → keeps waiting, exit 3 at deadline
#   d. no PRE_DIVERGENT line                → today's behaviour, exit 3
#   e. UNMEASURED sweeps (divergentAnchor 0, peersAsked 0), PRE=41 → HOLD, exit 3
set -euo pipefail

REPO_ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
GATE="${GATE:-${REPO_ROOT}/scripts/ci/peer-roll-gate.sh}"
TEST_ROOT="$(mktemp -d)"
SERVER_PID=""

cleanup() {
  [ -n "$SERVER_PID" ] && kill "$SERVER_PID" 2>/dev/null || true
  rm -rf "${TEST_ROOT}"
}
trap cleanup EXIT

fail() { echo "FAIL: $*" >&2; exit 1; }

mkdir -p "${TEST_ROOT}/seq"
cat > "${TEST_ROOT}/server.py" <<'PY'
import http.server, os, sys

ROOT = sys.argv[1]
served = {}

class Handler(http.server.BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def do_GET(self):
        parts = self.path.strip("/").split("/")
        if len(parts) != 3 or parts[1:] != ["p2p", "status"]:
            self.send_error(404)
            return
        scenario = parts[0]
        try:
            with open(os.path.join(ROOT, scenario)) as f:
                bodies = [l for l in f.read().splitlines() if l.strip()]
        except OSError:
            self.send_error(404)
            return
        n = served.get(scenario, 0)
        served[scenario] = n + 1
        data = bodies[min(n, len(bodies) - 1)].encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

srv = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
with open(os.path.join(ROOT, "..", "port"), "w") as f:
    f.write(str(srv.server_address[1]))
srv.serve_forever()
PY

python3 "${TEST_ROOT}/server.py" "${TEST_ROOT}/seq" &
SERVER_PID=$!
for _ in $(seq 1 50); do
  [ -s "${TEST_ROOT}/port" ] && break
  sleep 0.1
done
[ -s "${TEST_ROOT}/port" ] || fail 'stub storage server did not start'
BASE="http://127.0.0.1:$(cat "${TEST_ROOT}/port")"

pr() { # converged healed divergent sweeps [peersAsked=3] -> one /p2p/status body
  printf '{"peerId":"x","projectionReconcile":{"converged":%s,"caughtUp":false,"peersAsked":%s,"healedTotal":%s,"divergentAnchor":%s,"sweeps":%s}}\n' \
    "$1" "${5:-3}" "$2" "$3" "$4"
}

export ROLL_POLL_SECS=1
export ROLL_PEER_MIN_DEADLINE_SECS=1
export ROLL_SEQUENCE_DEADLINE_SECS=60
export ROLL_CURL_TIMEOUT_SECS=2
# Refused loopback port: the throttle leg degrades immediately (the script
# prints its DEGRADED banner) instead of resolving the in-cluster default name.
export ROLL_PROM_URL='http://127.0.0.1:1'

run_gate() { # scenario peer deadline -> RC, LOG
  LOG="${TEST_ROOT}/gate-$1.log"
  set +e
  ROLL_PEER_DEADLINE_SECS="$3" bash "${GATE}" "$2" "${BASE}/$1" elohim-alpha "$2-conductor-0" 1 "${TEST_ROOT}/state-$1" \
    > "${LOG}" 2>&1
  RC=$?
  set -e
}

# ── a. converged:true on a newer sweep → exit 0 `converged` (unchanged) ─────
{ pr false 0 41 1; pr true 0 41 2; } > "${TEST_ROOT}/seq/a"
printf 'PRE_DIVERGENT_eve=41\nPRE_HEALED_eve=7\n' > "${TEST_ROOT}/state-a"
run_gate a eve 10
[ "${RC}" -eq 0 ] || { cat "${LOG}"; fail "(a) converged peer should exit 0, got ${RC}"; }
grep -q 'DEGRADED — Prometheus at' "${LOG}" || fail '(a) throttle leg did not degrade'
grep -q 'RELEASED after .* converge=converged(fresh-sweep 1->2)' "${LOG}" \
  || { cat "${LOG}"; fail '(a) converged reading must release as converged'; }
if grep -q 'settled-at-baseline(' "${LOG}"; then fail '(a) converged must win over settled-at-baseline'; fi

# ── b. 41 → 41 → 41 with PRE_DIVERGENT=41 → exit 0 `settled-at-baseline` ────
{ pr false 0 41 1; pr false 0 41 2; pr false 0 41 3; } > "${TEST_ROOT}/seq/b"
printf 'BUDGET_LEFT=60\nPRE_DIVERGENT_james=41\nPRE_HEALED_james=12\nPRE_DIVERGENT_matthew=9\nPRE_HEALED_matthew=3\n' \
  > "${TEST_ROOT}/state-b"
run_gate b james 15
[ "${RC}" -eq 0 ] || { cat "${LOG}"; fail "(b) peer back at its pre-roll baseline should exit 0, got ${RC}"; }
grep -q 'pre-roll baseline — divergentAnchor=41 healedTotal=12' "${LOG}" || fail '(b) pre-roll baseline not announced'
grep -q 'at-baseline-confirming(divergentAnchor pre=41 now=41, fresh sweeps 1/2)' "${LOG}" \
  || { cat "${LOG}"; fail '(b) one fresh sweep must not release — two are required'; }
grep -q 'RELEASED after .* converge=settled-at-baseline(divergentAnchor pre=41 now=41, sweeps +2)' "${LOG}" \
  || { cat "${LOG}"; fail '(b) settled-at-baseline release line missing or drifted'; }
if grep -q 'converged' "${LOG}" "${TEST_ROOT}/state-b.summary"; then
  cat "${LOG}"; fail '(b) settled-at-baseline must never be called converged'
fi
grep -q '^james|RELEASED|.*settled-at-baseline(' "${TEST_ROOT}/state-b.summary" \
  || fail '(b) roll summary missing the settled-at-baseline record'
# The gate rewrites BUDGET_LEFT only — later peers' readings survive it.
[ "$(grep -c '^BUDGET_LEFT=' "${TEST_ROOT}/state-b")" -eq 1 ] || fail '(b) BUDGET_LEFT duplicated in the state file'
grep -qx 'PRE_DIVERGENT_matthew=9' "${TEST_ROOT}/state-b" || fail "(b) the gate erased a later peer's pre-roll reading"
grep -qx 'PRE_HEALED_matthew=3' "${TEST_ROOT}/state-b" || fail "(b) the gate erased a later peer's pre-roll reading"

# ── c. same readings, PRE_DIVERGENT=30 → keeps waiting, exit 3 at deadline ──
{ pr false 0 41 1; pr false 0 41 2; pr false 0 41 3; } > "${TEST_ROOT}/seq/c"
printf 'PRE_DIVERGENT_susan=30\nPRE_HEALED_susan=0\n' > "${TEST_ROOT}/state-c"
run_gate c susan 5
[ "${RC}" -eq 3 ] || { cat "${LOG}"; fail "(c) divergence above the pre-roll baseline must hold to deadline (exit 3), got ${RC}"; }
grep -q 'DEADLINE 5s reached' "${LOG}" || fail '(c) deadline not logged'
grep -q 'no-movement(healed 0->0, divergentAnchor 41->41' "${LOG}" || fail '(c) holding reading not recorded'
if grep -q 'settled-at-baseline(\|at-baseline-confirming(' "${LOG}"; then fail '(c) settle branch fired above the baseline'; fi

# ── d. no PRE_DIVERGENT line → today's behaviour, no new branch ─────────────
{ pr false 0 41 1; pr false 0 41 2; pr false 0 41 3; } > "${TEST_ROOT}/seq/d"
printf 'PRE_DIVERGENT_someone-else=41\n' > "${TEST_ROOT}/state-d"
run_gate d gertrude 5
[ "${RC}" -eq 3 ] || { cat "${LOG}"; fail "(d) without a pre-roll reading the gate must behave as before (exit 3), got ${RC}"; }
grep -q 'pre-roll baseline — none recorded; settled-at-baseline is OFF' "${LOG}" || fail '(d) OFF banner missing'
grep -q 'no-movement(healed 0->0, divergentAnchor 41->41' "${LOG}" || fail '(d) no-movement reading not recorded'
if grep -q 'settled-at-baseline(\|at-baseline-confirming(' "${LOG}"; then fail '(d) settle branch fired with no pre-roll reading'; fi

# ── e. UNMEASURED sweeps (db unavailable) never count as at-baseline ───────
# ReaDiscovery::empty() publishes sweeps+1 with divergentAnchor=0 and
# peersAsked=0; 0 <= PRE would otherwise release the peer on no evidence.
{ pr false 0 41 1; pr false 0 0 2 0; pr false 0 0 3 0; pr false 0 0 4 0; } > "${TEST_ROOT}/seq/e"
printf 'PRE_DIVERGENT_adam=41\nPRE_HEALED_adam=5\n' > "${TEST_ROOT}/state-e"
run_gate e adam 5
[ "${RC}" -eq 3 ] || { cat "${LOG}"; fail "(e) unmeasured sweeps must HOLD to deadline (exit 3), got ${RC}"; }
grep -q 'unmeasured-sweep(peersAsked=0, divergentAnchor 41->0' "${LOG}" || { cat "${LOG}"; fail '(e) unmeasured sweep not named'; }
if grep -q 'settled-at-baseline(\|at-baseline-confirming(' "${LOG}"; then fail '(e) an unmeasured sweep counted toward settled-at-baseline'; fi

echo 'peer-roll-gate: converged kept, settled-at-baseline releases at the pre-roll baseline, holds above it, absent reading unchanged, unmeasured sweeps hold'
