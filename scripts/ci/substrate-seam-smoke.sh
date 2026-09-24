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
# PROBE-BROKEN ≠ empty store: a non-200 or unparseable body must never read as
# "0 0" (that masked a doorway 404 as a partition signal on 2026-08-07 — see
# backlog/probe-conductor-diagnostics-doorway-404). Fail-closed on plumbing.
probe_json() { # url -> "HTTP <code>\n<body>"
  local url="$1" out code
  out=$(curl -sS -m 20 -w '\n%{http_code}' "$url" 2>/dev/null) || { echo "000"; return; }
  code=${out##*$'\n'}
  printf '%s\n%s' "$code" "${out%$'\n'*}"
}
pa=$(probe_json "$A/admin/bootstrap-coherence"); code_a=${pa%%$'\n'*}; ba=${pa#*$'\n'}
pb=$(probe_json "$B/admin/bootstrap-coherence"); code_b=${pb%%$'\n'*}; bb=${pb#*$'\n'}
if [ "$code_a" != "200" ] || [ "$code_b" != "200" ]; then
  bad bootstrap-sharing "PROBE-BROKEN — /admin/bootstrap-coherence A=HTTP:$code_a B=HTTP:$code_b (plumbing, not store state)"
else
  ca=$(echo "$ba" | python3 -c "import json,sys; d=json.load(sys.stdin); print(d.get('spaces',0), d.get('agents',0))" 2>/dev/null || echo "PARSE-FAIL")
  cb=$(echo "$bb" | python3 -c "import json,sys; d=json.load(sys.stdin); print(d.get('spaces',0), d.get('agents',0))" 2>/dev/null || echo "PARSE-FAIL")
  if [ "$ca" = "PARSE-FAIL" ] || [ "$cb" = "PARSE-FAIL" ]; then
    bad bootstrap-sharing "PROBE-BROKEN — unparseable body (A=$ca B=$cb)"
  elif [ "$ca" = "$cb" ] && [ "$ca" != "0 0" ]; then
    note bootstrap-sharing "OK — both doorways read the same store ($ca spaces/agents)"
  else
    bad bootstrap-sharing "doorway views differ or empty (A=$ca B=$cb)"
  fi
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
  pd=$(probe_json "$side/db/p2p/conductor-diagnostics")
  diag_code=${pd%%$'\n'*}; diagnostics=${pd#*$'\n'}
  if [ "$diag_code" != "200" ]; then
    # Fail-closed on plumbing: a 404/timeout is NOT an empty peer store (the
    # 2026-08-07 partition triage read a doorway 404 as total=0 for hours).
    bad peer-store "PROBE-BROKEN — $side/db/p2p/conductor-diagnostics HTTP:$diag_code (plumbing, not store state; see backlog/probe-conductor-diagnostics-doorway-404)"
    continue
  fi
  # DEGRADED-200 IS NOT A THIN STORE. Since 2026-09-22 the route answers 200
  # with `agentsObservable:false` and NO `agents`/`agentCount` when the
  # conductor's peer store could not be read (a cell that has not joined its
  # network has no kitsune space, so `agent_info` answers K2SpaceNotFound).
  # Defaulting the missing keys to 0 printed "total=0 addressed=0" and sent the
  # operator down the bootstrap-diagnosis path in the trust-contract runbook —
  # the same lossy-measure shape as the 2026-08-07 doorway 404.
  agents=$(printf '%s' "$diagnostics" | \
    python3 -c "
import json,sys
d=json.load(sys.stdin)
if d.get('agentsObservable') is False or 'agents' not in d:
    # The conductor's own words, VERBATIM (newlines folded so the shell reads
    # one line). Truncating them is how a cause gets lost and then guessed.
    err = str(d.get('agentsError', 'agentsObservable=false'))
    print('UNOBSERVABLE ' + ' '.join(err.split()))
else:
    withurl=sum(1 for a in d.get('agents',[]) if a.get('url'))
    print(f\"{d.get('agentCount',0)} {withurl}\")" 2>/dev/null || echo "PARSE-FAIL")
  if [ "$agents" = "PARSE-FAIL" ]; then
    bad peer-store "PROBE-BROKEN — $side conductor-diagnostics body unparseable"
    continue
  fi
  case "$agents" in
    UNOBSERVABLE*)
      # THE ERROR DECIDES THE SENTENCE. `agent_info` resolves each space's peer
      # store through `space_if_exists`, so a MISSING KITSUNE SPACE really does
      # mean the cells have not joined their network — but a closed socket
      # ("Websocket closed: No connection"), an auth failure and a timeout all
      # arrive on this same degraded-200 body. Printing the network-join
      # explanation for every one of them sends an operator hunting a startup
      # window during a socket outage: an unsupported cause, which is the
      # failure class this seam exists to refuse.
      unobservable_reason="${agents#UNOBSERVABLE }"
      case "$unobservable_reason" in
        *K2SpaceNotFound*|*[Ss]pace\ not\ found*)
          bad peer-store "PROBE-DEGRADED — $side conductor peer store is NOT OBSERVABLE (HTTP 200, agentsObservable=false): ${unobservable_reason}. This is not an empty or thin store; that error names a MISSING KITSUNE SPACE, so the conductor's cells have not joined their network. Read the route's \`cells\` block (ListCellIds membership + per-role state) before touching bootstrap."
          ;;
        *)
          bad peer-store "PROBE-DEGRADED — $side conductor peer store is UNREADABLE (HTTP 200, agentsObservable=false): ${unobservable_reason}. This is not an empty or thin store, and this error text does NOT establish why — in particular it does not establish that the cells have not joined their network. Read the route's \`cells\` block (ListCellIds membership + per-role state) and the conductor's own error above before choosing a cure."
          ;;
      esac
      # Deliberately NOT a `continue`: the peer-URL legs below detect the same
      # degraded body and print an explicit SKIP. A leg that simply vanishes
      # from the output is the unalertable-absence shape this script exists to
      # avoid — an operator must be able to read that they were not evaluated.
      ;;
    *)
      total=${agents% *}; withurl=${agents#* }
      if [ "${total:-0}" -ge 5 ] && [ "${withurl:-0}" -ge 5 ]; then
        note peer-store "OK — $side conductor holds $total agent-infos ($withurl addressed)"
      else
        bad peer-store "$side conductor peer store thin (total=$total addressed=$withurl)"
      fi
      ;;
  esac

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

if document.get("agentsObservable") is False or "agents" not in document:
    # Degraded 200: the peer store is unreadable, so there are no URLs to
    # inspect and NOTHING is established about relay contamination. Naming that
    # is honest; printing a contamination FAIL would be a fabricated verdict.
    print("SKIP\tpeer store not observable (agentsObservable=false) — no peer URLs to inspect")
    print("SKIP\tpeer store not observable (agentsObservable=false) — no peer URLs to inspect")
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
    # SKIP = nothing established (degraded 200). Neither a pass nor a red: the
    # peer-store leg above already carries the PROBE-DEGRADED verdict, and a
    # second red about relay hosts we never read would be a fabricated finding.
    SKIP$'\t'*) note n0-contamination "SKIP — $side ${contamination_check#*$'\t'}" ;;
    *) bad n0-contamination "$side ${contamination_check#*$'\t'}" ;;
  esac
  case "$tx5_check" in
    OK$'\t'*) note no-lingering-tx5 "OK — $side ${tx5_check#*$'\t'}" ;;
    SKIP$'\t'*) note no-lingering-tx5 "SKIP — $side ${tx5_check#*$'\t'}" ;;
    *) bad no-lingering-tx5 "$side ${tx5_check#*$'\t'}" ;;
  esac
done

# ── 6. dht-fetch / head convergence (ADVISORY until scenario 2 is stable) ───
# Compares the notarized head AND the bytes served under it. The head alone is
# not the visitor's truth: on alpha (2026-09-19, edge/dev 1465) both doorways
# reported one notarized headActionHash while serving two different blobHashes,
# and this seam printed CONVERGED.
read_head() { # doorway-url -> "<headActionHash>\t<blobHash>\t<updatedAt>", "?" for an unread field
  curl -sS -m 20 "$1/db/content/elohim-host-landing/head" 2>/dev/null | python3 -c "
import json, sys
try:
    d = json.load(sys.stdin)
    print('\t'.join(str(d.get(k) or '?') for k in ('headActionHash', 'blobHash', 'updatedAt')))
except Exception:
    print('?\t?\t?')
" 2>/dev/null || printf '?\t?\t?\n'
}
IFS=$'\t' read -r ha ba_blob ua < <(read_head "$A")
IFS=$'\t' read -r hb bb_blob ub < <(read_head "$B")
if [ "$ha" = "?" ] || [ "$hb" = "?" ] || [ "$ha" != "$hb" ]; then
  note dht-fetch "ADVISORY-DIVERGENT — A=$ha B=$hb (scenario-2 gap; gate this once green ×2)"
elif [ "$ba_blob" = "?" ] || [ "$bb_blob" = "?" ]; then
  note dht-fetch "ADVISORY-UNREAD-BYTES — head CONVERGED ($ha) but a served blobHash was unreadable (A=$ba_blob B=$bb_blob)"
elif [ "$ba_blob" != "$bb_blob" ]; then
  note dht-fetch "ADVISORY-SAME-HEAD-DIFFERENT-BYTES — one notarized head ($ha), two blobs: A=$ba_blob ($ua) B=$bb_blob ($ub)"
else
  note dht-fetch "OK — landing canonical head CONVERGED ($ha) serving $ba_blob"
fi

# ── 6b. per-doorway torn-row check — compare the head RECORD's own blob_cid,
# not the anchor (story-1.4a design §e). The checks above compare the two
# doorways to EACH OTHER; this compares each doorway to its OWN ground truth:
# `GET /db/content/{id}/head-record` (http.rs:9152-9304) serves THIS peer's
# local DHT `Record` for its declared head, fetched through its own
# conductor. A doorway can agree with its neighbor on head+blobHash at the
# projection layer while its own DHT entry never named that blob at all —
# that is the shape 1.4a exists to make legible.
#
# `record` is base64 of a Holochain `Record`: the head action plus its
# `Content` entry, msgpack-encoded (`SerializedBytes`) — NOT structured JSON.
# Verified 2026-09-20 against the live household doorways (both currently
# CellDisabled, so only the error shape could be read live) and against
# source: elohim/elohim-storage/src/http.rs:9292-9298 builds the response as
# `{"headActionHash":…, "record": base64(bytes)}`; the DNA field is
# `content_store_integrity/src/lib.rs:521` `pub blob_cid: Option<String>` — a
# plain string, not a nested structure. msgpack encodes a string as a length
# prefix followed by raw UTF-8 with no escaping, so a known substring appears
# byte-for-byte in the decoded record — this is why containment
# (`served.encode() in decoded`) is a safe test WITHOUT a msgpack decoder
# (python3's stdlib is not guaranteed to carry one; design §e / risk U3).
#
# Containment finding the served hash -> OK (positive evidence they agree).
# Containment NOT finding it is deliberately NOT read as proof of a tear
# (U3 is unverified from source alone) — only extracting a DIFFERENT
# blob_cid-shaped run from the same record counts as ADVISORY-TORN-ROW
# evidence. Anything else (HTTP error, CellDisabled, unexpected JSON shape,
# unreadable base64, no blob_cid-shaped run at all) is UNREADABLE-HEAD-RECORD
# — it must never read as OK and never as TORN.
check_head_record_pointer() { # doorway-url content-id served-blob-hash head-hash
  local doorway="$1" cid="$2" served="$3" head="$4"
  if [ "$served" = "?" ]; then
    # No blob served (the common corpus shape: ~100% of content has no
    # blob_cid at all). Nothing to tear — never advisory.
    note dht-fetch "OK — doorway=$doorway no blob served for $cid (nothing to tear)"
    return
  fi
  local resp code body
  resp=$(curl -sS -m 20 -w '\n%{http_code}' "$doorway/db/content/$cid/head-record" 2>/dev/null)
  if [ -z "$resp" ]; then
    note dht-fetch "UNREADABLE-HEAD-RECORD doorway=$doorway reason=curl-failed"
    return
  fi
  code=${resp##*$'\n'}
  body=${resp%$'\n'*}
  if [ "$code" != "200" ]; then
    local reason="http-$code"
    # Live-verified 2026-09-20: a CellDisabled conductor answers 502 with
    # `{"error":"...CellDisabled(...)..."}` — no structured `cause` field on
    # the currently-running binary, so match the substring directly rather
    # than depend on a field that may not be deployed yet.
    case "$(printf '%s' "$body" | tr '[:upper:]' '[:lower:]')" in
      *celldisabled*) reason="cell-disabled" ;;
    esac
    note dht-fetch "UNREADABLE-HEAD-RECORD doorway=$doorway reason=$reason"
    return
  fi
  local verdict
  verdict=$(printf '%s' "$body" | SEAM_SERVED_BLOB_HASH="$served" python3 -c "
import base64, json, os, re, sys
served = os.environ.get('SEAM_SERVED_BLOB_HASH', '')
try:
    d = json.load(sys.stdin)
    record_b64 = d['record']
except Exception:
    print('UNREADABLE\tunexpected-shape')
    raise SystemExit(0)
try:
    decoded = base64.b64decode(record_b64)
except Exception:
    print('UNREADABLE\tbase64-decode-failed')
    raise SystemExit(0)
if served.encode('utf-8') in decoded:
    print('OK')
    raise SystemExit(0)
candidates = sorted(set(
    m.decode('ascii', 'replace')
    for m in re.findall(rb'sha256-[0-9a-fA-F]+', decoded)
    if m.decode('ascii', 'replace') != served
))
if candidates:
    print('TORN\t' + candidates[0])
else:
    print('UNREADABLE\tno-blob-cid-run')
" 2>/dev/null)
  case "$verdict" in
    OK)
      note dht-fetch "OK — doorway=$doorway head-record names served blob $served"
      ;;
    TORN$'\t'*)
      note dht-fetch "ADVISORY-TORN-ROW doorway=$doorway head=$head served=$served record=${verdict#*$'\t'}"
      ;;
    UNREADABLE$'\t'*)
      note dht-fetch "UNREADABLE-HEAD-RECORD doorway=$doorway reason=${verdict#*$'\t'}"
      ;;
    *)
      note dht-fetch "UNREADABLE-HEAD-RECORD doorway=$doorway reason=verdict-unreadable"
      ;;
  esac
}
check_head_record_pointer "$A" elohim-host-landing "$ba_blob" "$ha"
check_head_record_pointer "$B" elohim-host-landing "$bb_blob" "$hb"

exit $rc
