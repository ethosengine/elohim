#!/usr/bin/env bash
# Post-deploy substrate seam smoke — one named check per communication seam,
# so a red names ITSELF instead of surfacing days later as an unexplained
# convergence failure (the 2026-07-11 lesson: scenario-2 red could not
# localize which of five layers was broken; every check below had to be
# hand-built mid-incident).
# Runtime: the Jenkins builder container after deploy, never the bash-only
# deploy container. Python 3 is an intentional dependency for JSON URL parsing.
#
# Seams covered (see the dht-unity plan's seam map):
#   1. bootstrap-sharing   — both doorways read the SAME kitsune2 store
#   2. relay-reachability  — both sovereign iroh relays serve every client path
#   3. peer-store          — each primary conductor holds addressed peers
#   4. n0-contamination    — peer URLs name only our relays, never n0's fleet
#   5. no-lingering-tx5    — conductor peer URLs never use the tx5 wss scheme
#   6. dht-fetch           — advisory: divergent declared heads named
#
# Usage: substrate-seam-smoke.sh <doorwayA-url> <doorwayB-url> [--gate]
# Default is ADVISORY (always exit 0, print per-seam verdicts). With
# --gate, seams 1-5 failing exits non-zero. Seam 6 stays advisory until
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

# ── 2. relay reachability (replaces the retired tx5/SBD signal-bus) ────────
RELAY_A_URL="${RELAY_A_URL:-https://relay.alpha.elohim.host}"
RELAY_B_URL="${RELAY_B_URL:-https://relay.elohim.host}"

probe_relay() {
  local relay="${1%/}" ping_code generate_code ws_headers ws_code ws_protocol
  local plain_relay="http://${relay#*://}"

  ping_code=$(curl --http1.1 -sS -m 20 -o /dev/null -w '%{http_code}' \
    "${relay}/ping" 2>/dev/null || true)
  generate_code=$(curl --http1.1 -sS -m 20 -o /dev/null -w '%{http_code}' \
    "${plain_relay}/generate_204" 2>/dev/null || true)

  # A successful WebSocket handshake deliberately leaves the upgraded stream
  # open. The short curl deadline therefore may return 28 after already writing
  # the response headers; judge the captured handshake, not curl's final status.
  # HTTP/1.1 is load-bearing: h2 strips Upgrade headers and false-reds with 400.
  ws_headers=$(curl --http1.1 -sS -m 5 -D - -o /dev/null \
    -H 'Connection: Upgrade' \
    -H 'Upgrade: websocket' \
    -H 'Sec-WebSocket-Version: 13' \
    -H 'Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==' \
    -H 'Sec-WebSocket-Protocol: iroh-relay-v1' \
    "${relay}/relay" 2>/dev/null || true)
  ws_code=$(printf '%s\n' "$ws_headers" | tr -d '\r' | \
    awk '/^HTTP\// {code=$2} END {print code}')
  ws_protocol=$(printf '%s\n' "$ws_headers" | tr -d '\r' | \
    awk -F ':' 'tolower($1) == "sec-websocket-protocol" {
      value=$2; sub(/^[[:space:]]*/, "", value); protocol=value
    } END {print protocol}')

  if [ "$ping_code" = "200" ] && [ "$generate_code" = "204" ] && \
     [ "$ws_code" = "101" ] && [ "$ws_protocol" = "iroh-relay-v1" ]; then
    note relay-reachability "OK — ${relay} (/ping=200 /generate_204=204 WS=101 protocol=iroh-relay-v1)"
  else
    bad relay-reachability "${relay} (/ping=${ping_code:-unreadable} /generate_204=${generate_code:-unreadable} WS=${ws_code:-unreadable} protocol=${ws_protocol:-missing})"
  fi
}

probe_relay "$RELAY_A_URL"
probe_relay "$RELAY_B_URL"

# ── 3. peer-store (primary conductor addressing) ────────────────────────────
for side in "$A" "$B"; do
  diagnostics=$(curl -sS -m 30 "$side/db/p2p/conductor-diagnostics" 2>/dev/null)
  agents=$(printf '%s' "$diagnostics" | \
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

  # Inspect ONLY conductor-diagnostics peer URLs. Healthy iroh relay-dial logs
  # legitimately contain wss:// and must never feed the lingering-tx5 verdict.
  peer_url_checks=$(printf '%s' "$diagnostics" | python3 -c '
import json, sys
from urllib.parse import urlsplit

allowed = {"relay.alpha.elohim.host", "relay.elohim.host"}
try:
    document = json.load(sys.stdin)
except Exception:
    print("FAIL\tunreadable conductor-diagnostics JSON")
    print("FAIL\tunreadable conductor-diagnostics JSON")
    raise SystemExit(0)

urls = [entry.get("url") for entry in document.get("agents", [])
        if isinstance(entry.get("url"), str) and entry.get("url")]
if not urls:
    print("FAIL\tno peer URLs to inspect")
    print("FAIL\tno peer URLs to inspect")
    raise SystemExit(0)

foreign = []
invalid_shape = []
lingering_tx5 = 0
for url in urls:
    try:
        parsed = urlsplit(url)
        host = (parsed.hostname or "").lower().removesuffix(".")
        port = parsed.port
    except ValueError:
        host = "<malformed>"
        parsed = None
        port = None
    if host not in allowed:
        foreign.append(host or "<missing-host>")
    if (parsed is None or parsed.scheme.lower() != "https" or port != 443
            or not parsed.path.strip("/")):
        invalid_shape.append(urlsplit(url).scheme if parsed is not None else "malformed")
    if parsed is not None and parsed.scheme.lower() == "wss":
        lingering_tx5 += 1

if foreign or invalid_shape:
    failures = []
    if foreign:
        failures.append("foreign relay host(s): " + ", ".join(sorted(set(foreign))))
    if invalid_shape:
        failures.append("non-iroh peer URL shape(s): " + ", ".join(sorted(set(invalid_shape))))
    print("FAIL\t" + "; ".join(failures))
else:
    print(f"OK\t{len(urls)} peer URL host(s) belong to the sovereign relay pair")
if lingering_tx5:
    print(f"FAIL\t{lingering_tx5} conductor-diagnostics peer URL(s) still use wss://")
else:
    print(f"OK\t{len(urls)} conductor-diagnostics peer URL(s) contain no tx5 wss:// scheme")
' 2>/dev/null)
  contamination_check=$(printf '%s\n' "$peer_url_checks" | sed -n '1p')
  tx5_check=$(printf '%s\n' "$peer_url_checks" | sed -n '2p')
  case "$contamination_check" in
    OK$'\t'*) note n0-contamination "OK — $side ${contamination_check#*$'\t'}" ;;
    *) bad n0-contamination "$side ${contamination_check#*$'\t'}" ;;
  esac
  case "$tx5_check" in
    OK$'\t'*) note no-lingering-tx5 "OK — $side ${tx5_check#*$'\t'}" ;;
    *) bad no-lingering-tx5 "$side ${tx5_check#*$'\t'}" ;;
  esac
done

# ── 6. dht-fetch / head convergence (ADVISORY until scenario 2 is stable) ───
ha=$(curl -sS -m 20 "$A/db/content/elohim-host-landing/head" 2>/dev/null | python3 -c "import json,sys; print(json.load(sys.stdin).get('headActionHash','?'))" 2>/dev/null || echo "?")
hb=$(curl -sS -m 20 "$B/db/content/elohim-host-landing/head" 2>/dev/null | python3 -c "import json,sys; print(json.load(sys.stdin).get('headActionHash','?'))" 2>/dev/null || echo "?")
if [ "$ha" = "$hb" ] && [ "$ha" != "?" ]; then
  note dht-fetch "OK — landing canonical head CONVERGED ($ha)"
else
  note dht-fetch "ADVISORY-DIVERGENT — A=$ha B=$hb (scenario-2 gap; gate this once green ×2)"
fi

exit $rc
