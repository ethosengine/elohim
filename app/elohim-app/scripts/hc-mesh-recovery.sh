#!/usr/bin/env bash
# hc-mesh-recovery.sh — the churn primitive of the two-peer harness.
#
#   hc-mesh-recovery.sh <warm|cold> <recovering-peer> [--label k=v ...]
#
# One peer holds the other's full recoverable state. This script INFLICTS the
# loss and TIMES the recovery from the survivor, then writes one record. It is
# the resiliency saga's own demand (ch.1-2 awaken/form, 5-7 co-steward/converge/
# custody, 11 pull queue finishes), read from the recovering peer's HTTP
# surface only — never a filesystem diff (blob layouts differ per transport).
#
#   warm   stop the peer's storage; wipe DocStore + content db + blobs + caches;
#          keep identity.key/iroh.key (same transport identity, same agent).
#   cold   warm + identity.key + iroh.key: a NEW libp2p PeerId and iroh NodeId.
#          Declared limit: the conductor agent key survives (sandboxes are not
#          regenerated here) — cold join is a new TRANSPORT identity, not yet a
#          new agent. Regenerating one sandbox is a separate row.
#
# Backpressure witness (spec §3.5): the SURVIVOR's conductor is the one being
# hammered while the loser re-acquires. Both peers' max
# `recv_validation_receipt_received elapsed_s` during the window is recorded —
# a sync that destabilises a conductor shows up here as seconds, per transport.
#
# "Recovered" = five legs, all true (spec §3.3):
#   P0 /sync/v1/elohim/docs total == survivor's
#   P1 every survivor content row with a blobHash: /db/content/<id> 200 + equal blobHash
#   P2 GET /blob/<hash> 200 on the recovering peer for each of those hashes
#   P3 /p2p/status pull.caughtUp == true && pull.failed == 0
#   P4 doorway A and B both 200 on /db/content/elohim-host-landing
#
# Sourceable with RECOVERY_SOURCE_ONLY=1 (unit tests use the functions).
set -u
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RECOVERY_DEADLINE_SECS="${RECOVERY_DEADLINE_SECS:-900}"
RECOVERY_POLL_SECS="${RECOVERY_POLL_SECS:-5}"
RECOVERY_DOORWAY_A="${RECOVERY_DOORWAY_A:-http://localhost:${DOORWAY_PORT:-8888}}"
RECOVERY_DOORWAY_B="${RECOVERY_DOORWAY_B:-http://localhost:${DOORWAY_B_PORT:-8889}}"
RECOVERY_LANDING_PATH="${RECOVERY_LANDING_PATH:-/db/content/elohim-host-landing}"
MESH_DIR="${MESH_DIR:-/tmp/elohim-local-mesh}"

rlog() { printf 'recovery[%s]: %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$1"; }

recovery_snapshot() { # <survivor-http-port> -> JSON on stdout
  local port="$1"
  python3 - "$port" <<'PY'
import json, sys, urllib.request
port = sys.argv[1]
def get(p):
    with urllib.request.urlopen(f"http://localhost:{port}{p}", timeout=20) as r: return json.load(r)
items = get("/db/content?limit=500").get("items", [])
print(json.dumps({
  "docs": get("/sync/v1/elohim/docs?limit=1").get("total", 0),
  "content": get("/db/stats").get("contentCount", 0),
  "rows": [{"id": i["id"], "blobHash": i["blobHash"]} for i in items if i.get("blobHash")],
}))
PY
}

recovery_predicate() { # <snapshot-file> <recovering-http-port> -> "P0=.. P1=.. P2=.. P3=.. P4=.."; rc 0 iff all 1
  local snap="$1" port="$2"
  python3 - "$snap" "$port" "$RECOVERY_DOORWAY_A" "$RECOVERY_DOORWAY_B" "$RECOVERY_LANDING_PATH" <<'PY'
import json, sys, urllib.request, urllib.error
snap, port, dwa, dwb, landing = sys.argv[1:6]
s = json.load(open(snap))
def code(url):
    # Read the body BEFORE the `with` exits: urlopen's context manager closes
    # the underlying connection on return, so handing back the response
    # object itself (post-close) leaves callers reading an exhausted stream.
    try:
        with urllib.request.urlopen(url, timeout=15) as r: return r.status, r.read()
    except urllib.error.HTTPError as e: return e.code, None
    except Exception: return 0, None
def getj(p):
    c, body = code(f"http://localhost:{port}{p}")
    return json.loads(body) if c == 200 and body else None
docs = getj("/sync/v1/elohim/docs?limit=1")
p0 = int(bool(docs) and docs.get("total") == s["docs"])
p1 = 1; p2 = 1
for row in s["rows"]:
    j = getj(f"/db/content/{row['id']}")
    if not j or j.get("blobHash") != row["blobHash"]: p1 = 0
    if code(f"http://localhost:{port}/blob/{row['blobHash']}")[0] != 200: p2 = 0
st = getj("/p2p/status") or {}
pull = st.get("pull") or {}
p3 = int(pull.get("caughtUp") is True and pull.get("failed", 1) == 0)
import os
if os.environ.get("MESH_DOORWAYS", "1") == "0":
    p4 = "-"   # skipped, not passed: there are no doorways to serve through
else:
    p4 = int(code(dwa + landing)[0] == 200 and code(dwb + landing)[0] == 200)
print(f"P0={p0} P1={p1} P2={p2} P3={p3} P4={p4}")
sys.exit(0 if p0 and p1 and p2 and p3 and p4 != 0 else 1)
PY
}

if [ "${RECOVERY_SOURCE_ONLY:-0}" = "1" ]; then return 0 2>/dev/null || exit 0; fi

# ---- main -------------------------------------------------------------------
shape="${1:-}"; peer="${2:-}"; shift 2 2>/dev/null || true
case "$shape" in warm|cold) ;; *) echo "usage: $0 <warm|cold> <recovering-peer> [--label k=v ...]" >&2; exit 2 ;; esac
labels="{}"
while [ $# -gt 0 ]; do
  case "$1" in --label) labels="$(python3 -c 'import json,sys;d=json.loads(sys.argv[1]);k,v=sys.argv[2].split("=",1);d[k]=v;print(json.dumps(d))' "$labels" "$2")"; shift 2 ;;
  *) echo "unknown arg $1" >&2; exit 2 ;; esac
done

set +e
# shellcheck source=hc-mesh.sh
source "$SCRIPT_DIR/hc-mesh.sh" >/dev/null 2>&1
set -u
idx=-1; i=0; for n in "${PEERS[@]}"; do [ "$n" = "$peer" ] && idx=$i; i=$((i+1)); done
[ "$idx" -ge 0 ] || { echo "$peer is not in MESH_PEERS=$MESH_PEERS" >&2; exit 2; }
[ "${#PEERS[@]}" -ge 2 ] || { echo "recovery harness needs at least two peers (MESH_PEERS=$MESH_PEERS)" >&2; exit 2; }
# Survivors = every other peer (the fanout family has N of them); the snapshot
# is taken from the FIRST survivor — they are converged with each other by
# construction (the loser is the only one that lost anything).
survivors=(); for n in "${PEERS[@]}"; do [ "$n" != "$peer" ] && survivors+=("$n"); done
survivor="${survivors[0]}"; sidx=-1; i=0; for n in "${PEERS[@]}"; do [ "$n" = "$survivor" ] && sidx=$i; i=$((i+1)); done
rport="$(http_port "$idx")"; sport="$(http_port "$sidx")"
t_surv="$(storage_transport_for "$survivor" "$sport")"; t_rec="$(peer_transport "$peer")"

snap="$(mktemp)"; recovery_snapshot "$sport" > "$snap" || { echo "survivor $survivor:$sport unreadable" >&2; exit 3; }
rlog "shape=$shape peer=$peer survivors=${#survivors[@]} survivor=$survivor rows=$(python3 -c 'import json,sys;print(len(json.load(open(sys.argv[1]))["rows"]))' "$snap") transports survivor=$t_surv recovering=$t_rec"

pid="$(storage_pid_for_port "$rport")"
[ -n "$pid" ] && { kill "$pid"; for _ in $(seq 1 15); do kill -0 "$pid" 2>/dev/null || break; sleep 1; done; kill -0 "$pid" 2>/dev/null && kill -9 "$pid"; }
wipe=(sync.sled content.db content.db-shm content.db-wal graph.db blobs blobs_iroh cache contest-backoff.json)
[ "$shape" = cold ] && wipe+=(identity.key iroh.key)
for f in "${wipe[@]}"; do rm -rf "${MESH_DIR:?}/$peer/$f"; done
rlog "loss inflicted: $shape wipe of ${#wipe[@]} entries under $MESH_DIR/$peer"

MESH_RESTART_APPLY_PROFILE=1 restart_storage "$peer" >/dev/null 2>&1 || rlog "restart_storage reported non-zero; polling anyway"
t0=$(date +%s); until curl -sf -m 2 "http://localhost:$rport/health" >/dev/null; do sleep 1; [ $(( $(date +%s) - t0 )) -gt 120 ] && { echo "$peer never served /health" >&2; exit 4; }; done
t0=$(date +%s); polls=0; legs=""; recovered=0
while :; do
  legs="$(recovery_predicate "$snap" "$rport")"; rc=$?; polls=$((polls+1)); el=$(( $(date +%s) - t0 ))
  if [ "$rc" -eq 0 ]; then rlog "PASS $legs — elapsed ${el}s"; recovered=1; break; else rlog "FAIL $legs — elapsed ${el}s"; fi
  [ "$el" -ge "$RECOVERY_DEADLINE_SECS" ] && break
  sleep "$RECOVERY_POLL_SECS"
done
failing="$(tr ' ' '\n' <<<"$legs" | grep '=0$' | cut -d= -f1 | paste -sd, -)"
if [ "$recovered" -eq 1 ]; then echo "RECOVERED in ${el}s"; else echo "NOT-RECOVERED after ${el}s ($failing)"; fi
# Backpressure witness: max conductor receipt latency logged by each peer since t0.
receipt_max() { # <peer-log> <since-epoch> -> max elapsed_s (0 if none)
  python3 - "$1" "$2" <<'PY2'
import re, sys, datetime
log, since = sys.argv[1], int(sys.argv[2]); mx = 0.0
pat = re.compile(r'(\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d)[^\n]*elapsed_s[^0-9]*([0-9.]+)[^\n]*recv_validation_receipt_received')
for line in open(log, errors="replace"):
    m = pat.search(line)
    if not m: continue
    ts = datetime.datetime.strptime(m.group(1), "%Y-%m-%dT%H:%M:%S").replace(tzinfo=datetime.timezone.utc).timestamp()
    if ts >= since: mx = max(mx, float(m.group(2)))
print(f"{mx:.1f}")
PY2
}
rcpt_rec="$(receipt_max "$LOGDIR/$peer.log" "$t0")"; rcpt_surv="$(receipt_max "$LOGDIR/$survivor.log" "$t0")"
rlog "conductor receipt latency max during recovery: recovering=${rcpt_rec}s survivor=${rcpt_surv}s"
RECOVERY_SURVIVORS="${#survivors[@]}" python3 - "$MESH_DIR/recovery-timeline.jsonl" "$shape" "$peer" "$survivor" "$t_surv" "$t_rec" "$recovered" "$el" "$polls" "$failing" "$labels" "$rcpt_rec" "$rcpt_surv" <<'PY'
import json, sys, datetime, os
p, shape, peer, surv, ts, tr, rec, el, polls, failing, labels, rr, rs = sys.argv[1:14]
json.dump({"ts": datetime.datetime.utcnow().strftime("%Y-%m-%dT%H:%M:%SZ"), "shape": shape, "peer": peer, "survivor": surv, "survivors": int(os.environ.get("RECOVERY_SURVIVORS", "1")),
           "transport_survivor": ts, "transport_recovering": tr, "recovered": rec == "1", "time_to_recover_s": int(el),
           "polls": int(polls), "failing_legs": [x for x in failing.split(",") if x], "labels": json.loads(labels),
           "conductor_receipt_max_s": {"recovering": float(rr), "survivor": float(rs)}}, open(p, "a"))
open(p, "a").write("\n")
PY
rm -f "$snap"
[ "$recovered" -eq 1 ]
