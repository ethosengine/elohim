#!/usr/bin/env bash
# Post-deploy substrate seam smoke — one named check per communication seam,
# so a red names ITSELF instead of surfacing days later as an unexplained
# convergence failure (the 2026-07-11 lesson: scenario-2 red could not
# localize which of five layers was broken; every check below had to be
# hand-built mid-incident).
#
# Seams covered (see the dht-unity plan's seam map):
#   1. bootstrap-sharing   — both doorways read the SAME kitsune2 store
#   2. signal-bus          — SBD frames deliver CROSS-relay (mongo bus)
#   3. peer-store          — each primary conductor holds addressed peers
#   4. dht-fetch           — advisory: divergent declared heads named
#
# Usage: substrate-seam-smoke.sh <doorwayA-url> <doorwayB-url> [--gate]
# Default is ADVISORY (always exit 0, print per-seam verdicts). With
# --gate, seams 1-3 failing exits non-zero. Seam 4 stays advisory until
# notary-authority scenario 2 is green ×2 (then flip it into the gate).
set -uo pipefail

A="${1:?doorway A url}"; B="${2:?doorway B url}"; GATE="${3:-}"
rc=0
note() { echo "seam-smoke[$1]: $2"; }
bad()  { note "$1" "FAIL — $2"; [ "$GATE" = "--gate" ] && rc=1; }

# ── 1. bootstrap-sharing ────────────────────────────────────────────────────
ba=$(curl -sS -m 20 "$A/admin/bootstrap-coherence" 2>/dev/null)
bb=$(curl -sS -m 20 "$B/admin/bootstrap-coherence" 2>/dev/null)
ca=$(echo "$ba" | python3 -c "import json,sys; d=json.load(sys.stdin); print(d.get('spaces',0), d.get('agents',0))" 2>/dev/null || echo "0 0")
cb=$(echo "$bb" | python3 -c "import json,sys; d=json.load(sys.stdin); print(d.get('spaces',0), d.get('agents',0))" 2>/dev/null || echo "0 0")
if [ "$ca" = "$cb" ] && [ "$ca" != "0 0" ]; then
  note bootstrap-sharing "OK — both doorways read the same store ($ca spaces/agents)"
else
  bad bootstrap-sharing "doorway views differ or empty (A=$ca B=$cb)"
fi

# ── 2. signal-bus (cross-relay delivery) ────────────────────────────────────
PROBE="$(dirname "$0")/../../doorway/doorway-service/tools/sbd-cross-relay-probe.py"
if python3 -c "import nacl, websockets" 2>/dev/null && [ -f "$PROBE" ]; then
  if out=$(timeout 150 python3 "$PROBE" 2>&1) && echo "$out" | grep -q "cross-AB=True cross-BA=True"; then
    note signal-bus "OK — frames deliver cross-relay both directions"
  else
    bad signal-bus "cross-relay delivery failed: $(echo "$out" | tail -1)"
  fi
else
  note signal-bus "SKIP — pynacl/websockets not in this runner (install to arm)"
fi

# ── 3. peer-store (primary conductor addressing) ────────────────────────────
for side in "$A" "$B"; do
  agents=$(curl -sS -m 30 "$side/db/p2p/conductor-diagnostics" 2>/dev/null | \
    python3 -c "
import json,sys
d=json.load(sys.stdin)
withurl=sum(1 for a in d.get('agents',[]) if a.get('url'))
print(f\"{d.get('agentCount',0)} {withurl}\")" 2>/dev/null || echo "0 0")
  total=${agents% *}; withurl=${agents#* }
  if [ "${total:-0}" -ge 5 ] && [ "${withurl:-0}" -ge 5 ]; then
    note peer-store "OK — $side conductor holds $total agent-infos ($withurl addressed)"
  else
    bad peer-store "$side conductor peer store thin (total=$total addressed=$withurl)"
  fi
done

# ── 4. dht-fetch / head convergence (ADVISORY until scenario 2 is stable) ───
ha=$(curl -sS -m 20 "$A/db/content/elohim-host-landing/head" 2>/dev/null | python3 -c "import json,sys; print(json.load(sys.stdin).get('headActionHash','?'))" 2>/dev/null || echo "?")
hb=$(curl -sS -m 20 "$B/db/content/elohim-host-landing/head" 2>/dev/null | python3 -c "import json,sys; print(json.load(sys.stdin).get('headActionHash','?'))" 2>/dev/null || echo "?")
if [ "$ha" = "$hb" ] && [ "$ha" != "?" ]; then
  note dht-fetch "OK — landing canonical head CONVERGED ($ha)"
else
  note dht-fetch "ADVISORY-DIVERGENT — A=$ha B=$hb (scenario-2 gap; gate this once green ×2)"
fi

exit $rc
