#!/usr/bin/env bash
# Hermetic regression test for the sequenced fleet roll (slice 4 of
# backlog/fleet-full-arc-conductor-saturation-and-coordinated-warmup-2026-09-11).
#
# Covers both halves with fakes only — no cluster, no kubectl, no network:
#   scripts/ci/sequenced-roll-plan.sh  ordering, suspension, escape hatch
#   scripts/ci/peer-roll-gate.sh       convergence leg, throttle leg, deadline
#
# Pattern follows scripts/ci/capture-rollout-evidence.test.sh: the external
# command is a bash function exported into the script under test.
set -euo pipefail

REPO_ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
PLAN="${REPO_ROOT}/scripts/ci/sequenced-roll-plan.sh"
GATE="${REPO_ROOT}/scripts/ci/peer-roll-gate.sh"
JENKINSFILE="${REPO_ROOT}/elohim/holochain/Jenkinsfile"
TEST_ROOT="$(mktemp -d)"

cleanup() { rm -rf "${TEST_ROOT}"; }
trap cleanup EXIT

fail() { echo "FAIL: $*" >&2; exit 1; }

REGISTRY="${TEST_ROOT}/deployments.json"
cat > "${REGISTRY}" <<'JSON'
{
  "humans": [
    { "name": "jessica",  "edgenodeArcFactor": "1" },
    { "name": "james" },
    { "name": "gertrude", "edgenodeArcFactor": "0" },
    { "name": "susan",    "edgenodeArcFactor": "0" },
    { "name": "eve",      "edgenodeArcFactor": "1" },
    { "name": "pete",     "edgenodeArcFactor": "1", "suspended": true },
    { "name": "nancy",    "edgenodeArcFactor": "0", "suspended": true },
    { "name": "adam",     "genesisPeer": true },
    { "name": "matthew",  "genesisPeer": true, "edgenodeArcFactor": "1" }
  ]
}
JSON

# ── 1. ORDERING: leechers first, full holders one at a time, genesis LAST ────
PLAN_OUT="${TEST_ROOT}/plan.txt"
bash "${PLAN}" "${REGISTRY}" > "${PLAN_OUT}"

EXPECTED_STEPS='STEP|1|leechers|gertrude,susan|gate
STEP|2|full|jessica|gate
STEP|3|full|james|gate
STEP|4|full|eve|gate
STEP|5|full-genesis|adam|gate
STEP|6|full-genesis|matthew|nogate'
ACTUAL_STEPS="$(grep '^STEP|' "${PLAN_OUT}")"
if [ "${ACTUAL_STEPS}" != "${EXPECTED_STEPS}" ]; then
  echo "--- expected ---"; echo "${EXPECTED_STEPS}"
  echo "--- actual ---";   echo "${ACTUAL_STEPS}"
  fail 'roll ordering drifted (leechers first, full holders one per step, genesis pair last, matthew very last)'
fi

# An absent edgenodeArcFactor is a FULL holder ("1" default, mirroring the
# Jenkinsfile sed) — james must never land in the leecher step.
grep -q '^STEP|1|leechers|gertrude,susan|' "${PLAN_OUT}" \
  || fail 'leecher step must carry exactly the arc-0 non-genesis peers'

# ── 2. SUSPENDED humans are skipped entirely ────────────────────────────────
if grep -Eq '^STEP\|.*(pete|nancy)' "${PLAN_OUT}"; then
  fail 'suspended humans reached the roll plan'
fi
grep -q '^PLAN|steps=6|gates=5|peers=7|leechers=2|full=3|genesis=2|mode=sequenced$' "${PLAN_OUT}" \
  || fail "plan summary drifted: $(grep '^PLAN|' "${PLAN_OUT}")"

# ── 2b. include-list restricts the plan to the humans this deploy rolls ─────
bash "${PLAN}" "${REGISTRY}" 'susan,eve,matthew' > "${TEST_ROOT}/plan-subset.txt"
SUBSET='STEP|1|leechers|susan|gate
STEP|2|full|eve|gate
STEP|3|full-genesis|matthew|nogate'
[ "$(grep '^STEP|' "${TEST_ROOT}/plan-subset.txt")" = "${SUBSET}" ] \
  || fail 'include-names-csv did not restrict the plan'

# ── 3. WORST CASE is computed and printed, and the defaults FIT the budget ──
grep -q '^WORST-CASE|gates=5|peer-deadline=900s|ungated-worst=4500s(75m)|sequence-budget=2700s(45m)|effective-worst=2700s(45m)$' "${PLAN_OUT}" \
  || fail "worst-case line drifted: $(grep '^WORST-CASE|' "${PLAN_OUT}")"

# ── 4. ROLL_SEQUENCED=0 passthrough restores the all-at-once shape ──────────
ROLL_SEQUENCED=0 bash "${PLAN}" "${REGISTRY}" > "${TEST_ROOT}/plan-passthrough.txt"
[ "$(grep -c '^STEP|' "${TEST_ROOT}/plan-passthrough.txt")" -eq 1 ] \
  || fail 'ROLL_SEQUENCED=0 must emit exactly one step'
grep -q '^STEP|1|all-at-once|jessica,james,gertrude,susan,eve,adam,matthew|nogate$' "${TEST_ROOT}/plan-passthrough.txt" \
  || fail 'passthrough step must carry the legacy order (non-genesis, genesis, matthew last) and be ungated'
grep -q '^PLAN|steps=1|gates=0|peers=7|leechers=0|full=0|genesis=2|mode=all-at-once$' "${TEST_ROOT}/plan-passthrough.txt" \
  || fail 'passthrough plan summary drifted'

# ════════════════════════════════════════════════════════════════════════════
# GATE
# ════════════════════════════════════════════════════════════════════════════

# Fake curl. Three call shapes, discriminated on argv:
#   query=up          Prometheus reachability probe
#   query=rate(       the CFS-throttle ratio
#   /p2p/status       the peer's own convergence reading
# Behaviour is driven by files under FAKE_DIR so a test can script a sequence.
curl() {
  local args="$*"
  local n
  case "${args}" in
    *'query=up'*)
      if [ -f "${FAKE_DIR}/prom-down" ]; then return 7; fi
      printf '{"status":"success","data":{"result":[]}}'
      ;;
    *'query=rate('*)
      if [ -f "${FAKE_DIR}/prom-down" ]; then return 7; fi
      cat "${FAKE_DIR}/throttle.json"
      ;;
    *'/p2p/status'*)
      if [ -f "${FAKE_DIR}/status-delay-secs" ]; then
        sleep "$(cat "${FAKE_DIR}/status-delay-secs")"
      fi
      n=$(cat "${FAKE_DIR}/poll" 2>/dev/null || echo 0)
      n=$((n + 1))
      printf '%s' "${n}" > "${FAKE_DIR}/poll"
      if [ -f "${FAKE_DIR}/status-${n}.json" ]; then
        cat "${FAKE_DIR}/status-${n}.json"
      else
        cat "${FAKE_DIR}/status.json"
      fi
      ;;
    *)
      echo "unexpected curl invocation: ${args}" >&2
      return 97
      ;;
  esac
}
export -f curl

new_fake() {
  FAKE_DIR="${TEST_ROOT}/fake-$1"
  mkdir -p "${FAKE_DIR}"
  export FAKE_DIR
  printf '{"status":"success","data":{"result":[{"value":[0,"0.42"]}]}}' > "${FAKE_DIR}/throttle.json"
}

pr() { # converged healed divergent sweeps -> a /p2p/status body
  printf '{"peerId":"x","projectionReconcile":{"converged":%s,"caughtUp":true,"healedTotal":%s,"divergentAnchor":%s,"sweeps":%s}}' \
    "$1" "$2" "$3" "$4"
}

export ROLL_POLL_SECS=1
export ROLL_PEER_MIN_DEADLINE_SECS=1
export ROLL_SEQUENCE_DEADLINE_SECS=20
export ROLL_PROM_URL='http://prom.test:9090'

# ── 5. A peer that CONVERGES releases the roll (exit 0) ─────────────────────
new_fake converged
pr true 10 0 5 > "${FAKE_DIR}/status-1.json" # stale pre-restart snapshot: anchor only
pr true 10 0 6 > "${FAKE_DIR}/status.json"   # a fresh post-anchor sweep may release
STATE="${TEST_ROOT}/state-converged"
set +e
ROLL_PEER_DEADLINE_SECS=10 bash "${GATE}" eve http://eve:8090 elohim-alpha eve-conductor-0 1 "${STATE}" \
  > "${TEST_ROOT}/gate-converged.log" 2>&1
RC=$?
set -e
[ "${RC}" -eq 0 ] || { cat "${TEST_ROOT}/gate-converged.log"; fail "converging peer should exit 0, got ${RC}"; }
grep -q 'RELEASED' "${TEST_ROOT}/gate-converged.log" || fail 'converging peer did not log RELEASED'
grep -q 'baseline-anchored(healed=10,divergent=0,sweeps=5)' "${TEST_ROOT}/gate-converged.log" \
  || fail 'stale converged snapshot was not anchored before release'
grep -q 'converge=converged(fresh-sweep 5->6)' "${TEST_ROOT}/gate-converged.log" \
  || fail 'fresh converged reason missing'
grep -q 'throttle=ratio=0.4200<0.9' "${TEST_ROOT}/gate-converged.log" || fail 'throttle leg did not read the ratio'
grep -q '^eve|RELEASED|' "${STATE}.summary" || fail 'roll summary missing the RELEASED record'
grep -q '^BUDGET_LEFT=' "${STATE}" || fail 'budget state not written'

# ── 5b. HEALING (healedTotal up, divergentAnchor down, sweeps up) releases ──
new_fake healing
pr false 100 40 7 > "${FAKE_DIR}/status-1.json"   # baseline anchor
pr false 100 40 7 > "${FAKE_DIR}/status-2.json"   # no movement yet
pr false 118 31 8 > "${FAKE_DIR}/status.json"     # fresh sweep healed, divergence fell
STATE="${TEST_ROOT}/state-healing"
set +e
ROLL_PEER_DEADLINE_SECS=10 bash "${GATE}" adam http://adam:8090 elohim-alpha adam-conductor-0 1 "${STATE}" \
  > "${TEST_ROOT}/gate-healing.log" 2>&1
RC=$?
set -e
[ "${RC}" -eq 0 ] || { cat "${TEST_ROOT}/gate-healing.log"; fail "healing peer should exit 0, got ${RC}"; }
grep -q 'baseline-anchored(healed=100,divergent=40,sweeps=7)' "${TEST_ROOT}/gate-healing.log" \
  || fail 'baseline was not anchored from the first post-restart reading'
grep -q 'converge=healing(healed 100->118, divergentAnchor 40->31, sweeps 7->8)' "${TEST_ROOT}/gate-healing.log" \
  || fail 'healing predicate did not fire'
# caughtUp is telemetry, never a leg.
grep -q 'caughtUp=1(telemetry-only)' "${TEST_ROOT}/gate-healing.log" \
  || fail 'caughtUp must be printed as telemetry-only'

# ── 6. A peer that does NOT settle hits its deadline, is SKIPPED (exit 3) ───
new_fake stalled
pr false 100 40 7 > "${FAKE_DIR}/status-1.json"   # baseline
pr false 100 40 8 > "${FAKE_DIR}/status.json"     # fresh but no heal
STATE="${TEST_ROOT}/state-stalled"
set +e
# Allow enough wall-clock for two complete status/throttle/parse cycles even
# when the host is under process pressure. A 2s allocation can expire after
# the baseline cycle alone, making the assertion below depend on scheduler
# timing instead of gate behavior.
ROLL_PEER_DEADLINE_SECS=5 bash "${GATE}" gertrude http://gertrude:8090 elohim-alpha gertrude-conductor-0 1 "${STATE}" \
  > "${TEST_ROOT}/gate-stalled.log" 2>&1
RC=$?
set -e
[ "${RC}" -eq 3 ] || { cat "${TEST_ROOT}/gate-stalled.log"; fail "stalled peer should exit 3 (continue), got ${RC}"; }
grep -q 'DEADLINE 5s reached' "${TEST_ROOT}/gate-stalled.log" || fail 'stalled peer did not log its deadline'
grep -q 'CONTINUING to the next peer' "${TEST_ROOT}/gate-stalled.log" \
  || fail 'a stalled peer must explicitly continue the roll, never abort it'
grep -q 'no-movement(healed 100->100' "${TEST_ROOT}/gate-stalled.log" || fail 'stalled reading not recorded'
grep -q '^gertrude|DEADLINE|' "${STATE}.summary" || fail 'roll summary missing the DEADLINE record'

# ── 6b. An unreachable peer is NOT a pass (absence of evidence) ─────────────
new_fake unreachable
printf 'not json' > "${FAKE_DIR}/status.json"
STATE="${TEST_ROOT}/state-unreachable"
set +e
ROLL_PEER_DEADLINE_SECS=2 bash "${GATE}" james http://james:8090 elohim-alpha james-conductor-0 1 "${STATE}" \
  > "${TEST_ROOT}/gate-unreachable.log" 2>&1
RC=$?
set -e
[ "${RC}" -eq 3 ] || fail "an unparseable /p2p/status must not release the roll (got ${RC})"
grep -q 'converge=no-status' "${TEST_ROOT}/gate-unreachable.log" || fail 'missing status must read as no-status'

# ── 6c. A throttled conductor holds the roll ────────────────────────────────
new_fake throttled
pr true 10 0 5 > "${FAKE_DIR}/status-1.json"
pr true 10 0 6 > "${FAKE_DIR}/status.json"
printf '{"status":"success","data":{"result":[{"value":[0,"1.0"]}]}}' > "${FAKE_DIR}/throttle.json"
STATE="${TEST_ROOT}/state-throttled"
set +e
ROLL_PEER_DEADLINE_SECS=2 bash "${GATE}" susan http://susan:8090 elohim-alpha susan-conductor-0 1 "${STATE}" \
  > "${TEST_ROOT}/gate-throttled.log" 2>&1
RC=$?
set -e
[ "${RC}" -eq 3 ] || fail "a CFS-saturated conductor must hold the roll (got ${RC})"
grep -q 'throttle=ratio=1.0000>=0.9' "${TEST_ROOT}/gate-throttled.log" || fail 'throttle ceiling not enforced'

# ── 6d. Prometheus unreachable DEGRADES the throttle leg, never wedges ──────
new_fake promdown
pr true 10 0 5 > "${FAKE_DIR}/status-1.json"
pr true 10 0 6 > "${FAKE_DIR}/status.json"
touch "${FAKE_DIR}/prom-down"
STATE="${TEST_ROOT}/state-promdown"
set +e
ROLL_PEER_DEADLINE_SECS=4 bash "${GATE}" jessica http://jessica:8090 elohim-alpha jessica-conductor-0 1 "${STATE}" \
  > "${TEST_ROOT}/gate-promdown.log" 2>&1
RC=$?
set -e
[ "${RC}" -eq 0 ] || fail "an unreachable Prometheus must degrade to the convergence leg, not stall (got ${RC})"
grep -q 'DEGRADED — Prometheus at' "${TEST_ROOT}/gate-promdown.log" || fail 'degraded mode was not announced'
grep -q 'throttle-leg=degraded' "${STATE}.summary" || fail 'degraded leg not recorded in the summary'

# ── 7. Fair-share clamp keeps the phase inside its budget ───────────────────
# 5 gates left against a 30s budget => 6s share, under the 10s per-peer ceiling.
# Six seconds leaves scheduler headroom for the two complete fake
# status/throttle/parse cycles this release assertion requires.
new_fake clamp
pr true 10 0 5 > "${FAKE_DIR}/status-1.json"
pr true 10 0 6 > "${FAKE_DIR}/status.json"
STATE="${TEST_ROOT}/state-clamp"
printf 'BUDGET_LEFT=30\n' > "${STATE}"
set +e
ROLL_PEER_DEADLINE_SECS=10 bash "${GATE}" eve http://eve:8090 elohim-alpha eve-conductor-0 5 "${STATE}" \
  > "${TEST_ROOT}/gate-clamp.log" 2>&1
RC=$?
set -e
[ "${RC}" -eq 0 ] || fail "clamp case should still release (got ${RC})"
grep -q 'deadline=6s (ceiling=10s, budget-left=30s over 5 remaining gate(s)' "${TEST_ROOT}/gate-clamp.log" \
  || { cat "${TEST_ROOT}/gate-clamp.log"; fail 'fair-share clamp did not divide the phase budget across the remaining gates'; }

# ── 7b. Remaining phase budget beats the floor and bounds slow calls ────────
# Two seconds remain, two gates remain, and the configured floor is 120s. The
# allocation must be 1s, and a slow status request must not let a throttle call
# or poll sleep extend this gate beyond the allocated wall-clock.
new_fake hard-budget
pr true 10 0 5 > "${FAKE_DIR}/status.json"
printf '1' > "${FAKE_DIR}/status-delay-secs"
STATE="${TEST_ROOT}/state-hard-budget"
START=$(date +%s)
set +e
ROLL_SEQUENCE_DEADLINE_SECS=2 ROLL_PEER_MIN_DEADLINE_SECS=120 ROLL_PEER_DEADLINE_SECS=900 \
  bash "${GATE}" eve http://eve:8090 elohim-alpha eve-conductor-0 2 "${STATE}" \
  > "${TEST_ROOT}/gate-hard-budget.log" 2>&1
RC=$?
set -e
ELAPSED=$(( $(date +%s) - START ))
[ "${RC}" -eq 3 ] || fail "hard-budget case should reach its measured deadline (got ${RC})"
grep -q 'deadline=1s (ceiling=900s, budget-left=2s over 2 remaining gate(s), floor=120s)' \
  "${TEST_ROOT}/gate-hard-budget.log" || fail 'remaining budget did not cap the minimum deadline floor'
[ "${ELAPSED}" -le 2 ] || { cat "${TEST_ROOT}/gate-hard-budget.log"; fail "slow curl/sleep overran the 1s allocation (${ELAPSED}s)"; }

# ── 7c. Runtime/deadline contract failures are loud implementation errors ──
# Build a PATH containing every declared dependency except curl. The helper
# must fail at entry (rc 1), before it can degrade or swallow the missing tool.
NO_CURL_BIN="${TEST_ROOT}/no-curl-bin"
mkdir -p "${NO_CURL_BIN}"
for cmd in python3 awk sed tail date sleep; do
  ln -s "$(command -v "$cmd")" "${NO_CURL_BIN}/${cmd}"
done
set +e
( unset -f curl; PATH="${NO_CURL_BIN}" /bin/bash "${GATE}" eve http://eve:8090 elohim-alpha eve-conductor-0 1 "${TEST_ROOT}/state-no-curl" ) \
  > "${TEST_ROOT}/gate-no-curl.log" 2>&1
RC=$?
set -e
[ "${RC}" -eq 1 ] || fail "missing curl must be an implementation error (got ${RC})"
grep -q 'required command not found: curl' "${TEST_ROOT}/gate-no-curl.log" \
  || fail 'missing curl was swallowed instead of diagnosed'

# Zero/malformed timing knobs must fail before awk/curl. In particular, a zero
# curl timeout must never reach curl as `--max-time 0` (which means unlimited).
new_fake bad-timeout
pr true 10 0 5 > "${FAKE_DIR}/status.json"
for bad_timeout in 0 00; do
  set +e
  ROLL_CURL_TIMEOUT_SECS="$bad_timeout" bash "${GATE}" eve http://eve:8090 elohim-alpha eve-conductor-0 1 "${TEST_ROOT}/state-zero-timeout" \
    > "${TEST_ROOT}/gate-zero-timeout.log" 2>&1
  RC=$?
  set -e
  [ "${RC}" -eq 1 ] || fail "curl timeout ${bad_timeout} must fail at entry (got ${RC})"
  grep -q 'ROLL_CURL_TIMEOUT_SECS must be a canonical positive integer' "${TEST_ROOT}/gate-zero-timeout.log" \
    || fail "curl timeout ${bad_timeout} was not diagnosed"
  if grep -q -- '--max-time 0' "${TEST_ROOT}/gate-zero-timeout.log"; then
    fail "curl timeout ${bad_timeout} reached curl as an unlimited request"
  fi
done

set +e
ROLL_PEER_DEADLINE_SECS=oops bash "${GATE}" eve http://eve:8090 elohim-alpha eve-conductor-0 1 "${TEST_ROOT}/state-bad-deadline" \
  > "${TEST_ROOT}/gate-bad-deadline.log" 2>&1
RC=$?
set -e
[ "${RC}" -eq 1 ] || fail "malformed peer deadline must fail at entry (got ${RC})"
grep -q 'ROLL_PEER_DEADLINE_SECS must be a canonical positive integer' "${TEST_ROOT}/gate-bad-deadline.log" \
  || fail 'malformed peer deadline was silently coerced'

# ── 8. Jenkinsfile wiring ───────────────────────────────────────────────────
grep -Fq "scripts/ci/sequenced-roll-plan.sh" "${JENKINSFILE}" || fail 'Jenkinsfile does not call the plan helper'
grep -Fq "scripts/ci/peer-roll-gate.sh" "${JENKINSFILE}" || fail 'Jenkinsfile does not call the per-peer gate'
# Root CLAUDE.md: bash bodies live in scripts/ci/*.sh. RATCHET, not absolute —
# this file carried 10 inline `sh """` heredocs before slice 4 and slice 4 added
# none. The number may only go DOWN (museum trap #8, the 64KB CPS method ceiling).
HEREDOC_BASELINE=10
HEREDOCS="$(grep -c 'sh """' "${JENKINSFILE}" || true)"
if [ "${HEREDOCS}" -gt "${HEREDOC_BASELINE}" ]; then
  fail "edge Jenkinsfile grew an inline sh heredoc (${HEREDOCS} > ${HEREDOC_BASELINE}) — CPS 64KB method-size trap #8; bash bodies belong in scripts/ci/*.sh"
fi
# Museum trap #18: the shared-artifact conductor roll halts on first failure.
grep -Fq 'CONDUCTOR_ROLL_CONTINUE_ON_FAILURE' "${JENKINSFILE}" \
  || fail 'halt-on-first-failure escape hatch was lost in the sequenced rewrite'
grep -Fq 'HALTED — no gate run' "${JENKINSFILE}" \
  || fail 'a halted roll must not spend the settle budget'
# No timer-triggered re-roll of an unchanged commit.
if grep -Eq '^\s*(cron|triggers)\s*\(' "${JENKINSFILE}"; then
  fail 'a timer trigger appeared in the edge Jenkinsfile'
fi
# A measured deadline continues, but an implementation/environment failure
# must mark UNSTABLE and feed the existing halt-on-first-failure state.
grep -Fq 'helper exited ${rc} (implementation or environment failure) — remaining conductors are HELD' "${JENKINSFILE}" \
  || fail 'unexpected gate helper errors are not surfaced as UNSTABLE'
grep -Fq 'if (!runPeerRollGate(' "${JENKINSFILE}" \
  || fail 'gate helper result is not wired into the conductor halt state'
grep -Fq 'rollState.halted = true' "${JENKINSFILE}" \
  || fail 'unexpected gate helper errors do not halt remaining conductors'
# Read-only helpers: neither script may mutate the cluster.
for s in "${PLAN}" "${GATE}"; do
  if grep -Eq 'kubectl (apply|delete|patch|replace|rollout|scale|annotate)' "${s}"; then
    fail "$(basename "${s}") contains a mutating kubectl verb"
  fi
done

echo 'sequenced-roll: ordering, suspension, escape hatch, gate legs, deadline-skip and Jenkins wiring passed'
