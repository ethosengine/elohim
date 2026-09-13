#!/usr/bin/env bash
# sequenced-roll-plan.sh — order the alpha conductor roll so the fleet is never
# restarted all at once.
#
# WHY THIS EXISTS (backlog/fleet-full-arc-conductor-saturation-and-coordinated-
# warmup-2026-09-11.md, cure slice 4): `elohim/holochain/Jenkinsfile` rolled
# every peer's conductor in one pass, and the quiesce legs were used only to
# MEASURE a deploy, never to SEQUENCE one. A simultaneous restart of seven
# full-arc conductors produces the reconnect/publish storm the atom measured
# (`could not insert incoming publish ops request into queue (Full)` on adam,
# 100% CFS throttle fleet-wide) and hours of catch-up afterwards.
#
# WHY BASH (root CLAUDE.md "Jenkinsfile Size Limit"): the edge Jenkinsfile sits
# against the JVM 64KB CPS method ceiling, and the CPS script-security sandbox
# has killed Groovy arithmetic in this very pipeline twice (#1403 rejected
# Double.parseDouble, #1404 rejected DGM toBigDecimal — see
# scripts/ci/conductor-split-budget.sh). Ordering and budget arithmetic live
# here; the Jenkinsfile only walks the printed plan.
#
# ORDERING CONTRACT
#   1. Suspended humans are skipped entirely (`"suspended": true` in
#      deployments.json). They have no running workload to roll.
#   2. LEECHERS FIRST — `edgenodeArcFactor == "0"`. A leecher holds no authority
#      arc, so restarting it does not move the authority set any peer is
#      gossiping against. They are emitted as ONE step (restarting them together
#      is safe; that is the whole point of the arc-0 lever the atom pulled for
#      susan on 2026-09-12).
#   3. Then FULL-ARC HOLDERS, STRICTLY ONE PER STEP, in deployments.json
#      declaration order (the order the operator reads, not alphabetical).
#   4. The GENESIS PAIR IS ALWAYS LAST — adam, then matthew very last —
#      regardless of arc factor. They are browser-facing, and matthew is
#      storage-A, the peer the Dataplane Validation quiesce predicate reads
#      (scripts/ci/fleet-quiesce-gate.sh). A genesis peer therefore never joins
#      the leecher step even if someone sets its arc factor to 0.
#
#   `edgenodeArcFactor` DEFAULTS TO "1" when absent — the same default the
#   Jenkinsfile's TARGET_ARC_FACTOR_PLACEHOLDER sed applies. An absent field is
#   a full-arc holder, never a leecher.
#
# ESCAPE HATCH: ROLL_SEQUENCED=0 restores the pre-slice-4 behaviour — one
# ungated step carrying every peer in the legacy order (non-genesis, then
# genesis, then matthew), gated by nothing and paced only by the legacy
# CONDUCTOR_STAGGER_SOAK_SECS soak. Default is sequenced.
#
# Usage: sequenced-roll-plan.sh <deployments.json> [include-names-csv]
#   include-names-csv — restrict the plan to the humans this deploy is actually
#   rolling (the Jenkinsfile passes its resolved human list). Omitted = every
#   non-suspended human in the registry.
#
# Output (stable, pipe-delimited, one record per line):
#   STEP|<n>|<phase>|<name[,name...]>|<gate|nogate>
#   PLAN|steps=..|gates=..|peers=..|leechers=..|full=..|genesis=..|mode=..
#   WORST-CASE|gates=..|peer-deadline=..|ungated-worst=..|sequence-budget=..|effective-worst=..
#
# The last step is always `nogate`: nothing follows it in this phase, and the
# fleet-wide judgement belongs to Dataplane Validation's own quiesce gate.
#
# Exit codes: 0 plan printed (possibly empty) | 1 usage/parse error
set -euo pipefail

usage() {
  echo "Usage: $(basename "$0") <deployments.json> [include-names-csv]" >&2
}

if [ "$#" -lt 1 ]; then
  usage
  exit 1
fi

REGISTRY="$1"
INCLUDE="${2:-}"

if [ ! -f "$REGISTRY" ]; then
  echo "sequenced-roll-plan: registry not found: ${REGISTRY}" >&2
  exit 1
fi

SEQUENCED="${ROLL_SEQUENCED:-1}"
PEER_DEADLINE="${ROLL_PEER_DEADLINE_SECS:-900}"
SEQ_BUDGET="${ROLL_SEQUENCE_DEADLINE_SECS:-2700}"

REGISTRY="$REGISTRY" INCLUDE="$INCLUDE" SEQUENCED="$SEQUENCED" \
PEER_DEADLINE="$PEER_DEADLINE" SEQ_BUDGET="$SEQ_BUDGET" python3 - <<'PYEOF'
import json, os, sys

registry_path = os.environ["REGISTRY"]
include_raw = os.environ.get("INCLUDE", "").strip()
sequenced = os.environ.get("SEQUENCED", "1") != "0"
peer_deadline = int(os.environ.get("PEER_DEADLINE", "900") or "900")
seq_budget = int(os.environ.get("SEQ_BUDGET", "2700") or "2700")

try:
    with open(registry_path, "r", encoding="utf-8") as fh:
        data = json.load(fh)
except Exception as exc:  # noqa: BLE001 — a malformed registry must be loud
    sys.stderr.write("sequenced-roll-plan: cannot parse %s: %s\n" % (registry_path, exc))
    sys.exit(1)

humans = data.get("humans")
if not isinstance(humans, list):
    sys.stderr.write("sequenced-roll-plan: %s has no humans[] array\n" % registry_path)
    sys.exit(1)

include = None
if include_raw:
    include = set(n.strip() for n in include_raw.split(",") if n.strip())

def arc_factor(h):
    # Absent => "1" (full arc) — mirrors the Jenkinsfile's
    # `humanConfig.edgenodeArcFactor ?: '1'` TARGET_ARC_FACTOR_PLACEHOLDER sed.
    v = h.get("edgenodeArcFactor")
    if v is None:
        return "1"
    return str(v).strip()

active = []
for h in humans:
    name = h.get("name")
    if not name:
        continue
    if h.get("suspended") is True:
        continue
    if include is not None and name not in include:
        continue
    active.append(h)

genesis = [h for h in active if h.get("genesisPeer") is True]
nongenesis = [h for h in active if h.get("genesisPeer") is not True]

# Genesis ordering: everything else first, matthew VERY last (storage-A, the
# peer the quiesce predicate reads). Preserve registry order among the rest.
genesis_ordered = [h for h in genesis if h.get("name") != "matthew"]
genesis_ordered += [h for h in genesis if h.get("name") == "matthew"]

steps = []  # (phase, [names], )
if not sequenced:
    # Passthrough: today's shape — one ungated step, legacy order
    # (non-genesis, then genesis, matthew last).
    names = [h["name"] for h in nongenesis] + [h["name"] for h in genesis_ordered]
    if names:
        steps.append(("all-at-once", names))
    leechers = []
    full_nongenesis = []
else:
    # A genesis peer is NEVER a leecher for ordering purposes, even at arc 0.
    leechers = [h for h in nongenesis if arc_factor(h) == "0"]
    full_nongenesis = [h for h in nongenesis if arc_factor(h) != "0"]

    if leechers:
        steps.append(("leechers", [h["name"] for h in leechers]))
    for h in full_nongenesis:
        steps.append(("full", [h["name"]]))
    for h in genesis_ordered:
        steps.append(("full-genesis", [h["name"]]))

total_peers = sum(len(names) for _phase, names in steps)
gates = max(len(steps) - 1, 0)

for idx, (phase, names) in enumerate(steps, start=1):
    gate = "gate" if (sequenced and idx < len(steps)) else "nogate"
    print("STEP|%d|%s|%s|%s" % (idx, phase, ",".join(names), gate))

mode = "sequenced" if sequenced else "all-at-once"
print(
    "PLAN|steps=%d|gates=%d|peers=%d|leechers=%d|full=%d|genesis=%d|mode=%s"
    % (
        len(steps),
        gates if sequenced else 0,
        total_peers,
        len(leechers),
        len(full_nongenesis),
        len(genesis_ordered),
        mode,
    )
)

def mins(secs):
    return "%ds(%dm)" % (secs, secs // 60)

if sequenced:
    ungated_worst = gates * peer_deadline
    effective_worst = min(ungated_worst, seq_budget)
else:
    ungated_worst = 0
    effective_worst = 0

print(
    "WORST-CASE|gates=%d|peer-deadline=%ds|ungated-worst=%s|sequence-budget=%s|effective-worst=%s"
    % (gates if sequenced else 0, peer_deadline, mins(ungated_worst), mins(seq_budget), mins(effective_worst))
)
PYEOF
