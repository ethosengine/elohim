#!/usr/bin/env bash
# Fail-closed post-deploy proof for the federation-failover saga.
# Usage: post-deploy-saga-probe.sh <doorway-a> <doorway-b> <content-id> [storage-a] [storage-b]
set -euo pipefail
A="${1:?doorway A URL}"; B="${2:?doorway B URL}"; CONTENT="${3:?content id}"
SA="${4:-$A}"; SB="${5:-$B}"; TIMEOUT="${PROBE_TIMEOUT:-20}"; RC=0
ok() { echo "post-deploy[$1]: OK — $2"; }
bad() { echo "post-deploy[$1]: FAIL — $2" >&2; RC=1; }
get() { curl -fsS --max-time "$TIMEOUT" "$1" 2>/dev/null; }
# Bodies reach python as FILES: one env string is capped at 128 KiB
# (MAX_ARG_STRLEN) and storage /metrics outgrew it — see fleet-quiesce-gate.sh.
BODY_DIR=$(mktemp -d); trap 'rm -rf "$BODY_DIR"' EXIT

probe() {
  local name="$1" doorway="$2" storage="$3" card verdict
  get "${storage%/}/p2p/status" > "$BODY_DIR/status" || { bad "$name/status" "GET failed"; return; }
  get "${storage%/}/metrics" > "$BODY_DIR/metrics" || { bad "$name/metrics" "GET failed"; return; }
  verdict=$(BODY_DIR="$BODY_DIR" python3 -c '
import json,os,re,sys
try: d=json.load(open(os.path.join(os.environ["BODY_DIR"],"status")))
except Exception as e: print(e,file=sys.stderr); sys.exit(1)
p=d.get("pull")
if not isinstance(p,dict) or p.get("caughtUp") is not True: print("pull.caughtUp is not true",file=sys.stderr); sys.exit(1)
t=open(os.path.join(os.environ["BODY_DIR"],"metrics")).read()
def v(n):
 x=[]
 for l in t.splitlines():
  if l.startswith(n) and not l.startswith("#"):
   m=re.search(r"\s(-?\d+(?:\.\d+)?)\s*$",l)
   if m:x.append(float(m.group(1)))
 return max(x) if x else None
r,c,d=v("elohim_acquisition_pins_retired"),v("elohim_projection_reconcile_converged"),v("elohim_projection_reconcile_divergent")
if r is None or r<1: print(f"retired-pin gauge={r!r}; caught-up proof incomplete",file=sys.stderr); sys.exit(1)
if c!=1 or d is None or d<1: print(f"honesty fence failed: converged={c!r} divergent={d!r}",file=sys.stderr); sys.exit(1)
print(r)
') || { bad "$name/status" "caught-up/retirement or reconcile honesty evidence missing"; return; }
  ok "$name/status" "pull.caughtUp=true with retired pins=${verdict}"
  ok "$name/reconcile" "converged=1 with divergent>=1 honesty fence"
  card=$(get "${doorway%/}/api/v1/resilience/$(python3 -c 'import urllib.parse,sys; print(urllib.parse.quote(sys.argv[1],safe=""))' "$CONTENT")/household") || { bad "$name/card" "resilience snapshot request failed"; return; }
  if printf '%s' "$card" | python3 -c 'import json,sys; v=json.load(sys.stdin).get("stewardingCollectives"); assert isinstance(v,(int,float)) and v>=1'; then
    ok "$name/card" "stewardingCollectives>=1"
  else
    bad "$name/card" "stewardingCollectives missing or zero"
  fi
}
probe alpha-A "$A" "$SA"
probe alpha-B "$B" "$SB"
exit "$RC"
