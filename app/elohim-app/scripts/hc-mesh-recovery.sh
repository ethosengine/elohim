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

# Backpressure witness: max conductor receipt latency logged by each peer since t0.
# Defined here (above the RECOVERY_SOURCE_ONLY gate) so unit tests can source it
# alongside recovery_snapshot/recovery_predicate.
#
# Reads the CONDUCTOR's own log, never elohim-storage's: recv_validation_receipt_received
# is emitted by holochain_p2p (elohim/holochain-conductor/crates/holochain_p2p/src/spawn/
# actor.rs, warn! at elapsed_s>=5) — that output lands in
# $LOCAL_DEV_DIR/.sandbox_run_log.<peer> (a peer restarted individually, e.g.
# conductors-restart) or the shared $LOCAL_DEV_DIR/.sandbox_run_log (all conductors
# launched together by `start`) — never $LOGDIR/<peer>.log, which is elohim-storage's
# own log and never carries this line. LOCAL_DEV_DIR comes from hc-mesh.sh.
receipt_log_for() { # <peer> -> the conductor log this peer's witness should read
  local per="$LOCAL_DEV_DIR/.sandbox_run_log.$1"
  if [ -f "$per" ]; then echo "$per"; else echo "$LOCAL_DEV_DIR/.sandbox_run_log"; fi
}
receipt_scope_for() { # <peer> -> "per-peer" | "mesh-wide" — which log receipt_log_for picked
  if [ -f "$LOCAL_DEV_DIR/.sandbox_run_log.$1" ]; then echo "per-peer"; else echo "mesh-wide"; fi
}
receipt_max() { # <peer> <since-epoch> -> max elapsed_s, or the literal "null" if the
  # log is missing/unreadable or has no in-window sample (rc 0 either way — never
  # let a missing log destroy the caller's record).
  local log; log="$(receipt_log_for "$1")"
  python3 - "$log" "$2" <<'PY2'
import re, sys, datetime
log, since = sys.argv[1], int(sys.argv[2]); mx = None
# Strip ANSI SGR escapes BEFORE matching: the real tracing-formatter output
# interleaves them between field name/`=`/value (e.g.
# `\x1b[3melapsed_s\x1b[0m=\x1b[0m5.01...`), and an unstripped
# `elapsed_s[^0-9]*([0-9.]+)` pattern latches onto the first digit it meets —
# which can be the `0` inside a `\x1b[0m` reset code, not the real value.
ansi = re.compile(r'\x1b\[[0-9;]*[A-Za-z]')
pat = re.compile(r'(\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d)[^\n]*elapsed_s=([0-9.]+)[^\n]*recv_validation_receipt_received')
try:
    fh = open(log, errors="replace")
except OSError:
    fh = None
if fh is not None:
    with fh:
        for raw in fh:
            line = ansi.sub('', raw)
            m = pat.search(line)
            if not m: continue
            ts = datetime.datetime.strptime(m.group(1), "%Y-%m-%dT%H:%M:%S").replace(tzinfo=datetime.timezone.utc).timestamp()
            if ts >= since:
                v = float(m.group(2))
                if mx is None or v > mx: mx = v
print("null" if mx is None else f"{mx:.1f}")
PY2
}

# Pre-kill capture (Critical-2): mirrors restart_storage's live branch in
# hc-mesh.sh — read /proc/<pid>/environ to EOF with python (never cp/copyFile,
# which yields 0 bytes on procfs) and readlink /proc/<pid>/exe (stripped of a
# trailing " (deleted)") — BEFORE anything is killed or wiped. Without this,
# restart_storage falls to its stale-capture branch once the pid is gone; after
# a mesh reshape (`start` regenerates sandboxes and mints new agent keys) that
# capture carries a STALE AGENT_PUBKEY, and with no capture at all the peer
# cannot come back. Refuses (rc 5) rather than inflict loss with no way home.
recovery_capture_peer() { # <peer> <pid-or-empty> -> writes $MESH_DIR/storage-restart/<peer>.{environ,exe}
  local peer="$1" pid="${2:-}" workdir="$MESH_DIR/storage-restart" envfile exefile exe
  mkdir -p "$workdir"
  envfile="$workdir/$peer.environ"; exefile="$workdir/$peer.exe"
  if [ -n "$pid" ] && [ -r "/proc/$pid/environ" ]; then
    python3 - "$pid" "$envfile" <<'PY'
import sys
pid, destination = sys.argv[1:]
with open(f"/proc/{pid}/environ", "rb") as source:
    raw = source.read()
with open(destination, "wb") as target:
    target.write(raw)
PY
    exe="$(readlink "/proc/$pid/exe" 2>/dev/null | sed 's/ (deleted)$//')"
    [ -n "$exe" ] && printf '%s\n' "$exe" > "$exefile"
    return 0
  fi
  [ -s "$envfile" ] && return 0
  echo "$peer: no live pid and no capture — refusing to inflict loss" >&2
  return 5
}

# Record writer, factored out of main (Important-1) so a missing/unreadable
# conductor log can never silently drop the whole JSONL line: both receipt
# values tolerate "null"/empty instead of raising inside float(...).
recovery_write_record() { # <path> <shape> <peer> <survivor> <t_surv> <t_rec> <recovered 0|1> <elapsed_s> <polls> <failing_csv> <labels_json> <rr> <rs> <receipt_scope_rec> <receipt_scope_surv> <zome_verdict>
  python3 - "$1" "$2" "$3" "$4" "$5" "$6" "$7" "$8" "$9" "${10}" "${11}" "${12}" "${13}" "${14}" "${15}" "${16}" <<'PY'
import json, sys, datetime, os
p, shape, peer, surv, ts, tr, rec, el, polls, failing, labels, rr, rs, rscope_rec, rscope_surv, zome = sys.argv[1:17]
def numeric_or_none(x):
    xs = (x or "").strip()
    if xs == "" or xs.lower() == "null": return None
    return float(xs)
record = {
    "ts": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "shape": shape, "peer": peer, "survivor": surv,
    "survivors": int(os.environ.get("RECOVERY_SURVIVORS", "1")),
    "transport_survivor": ts, "transport_recovering": tr,
    "recovered": rec == "1", "time_to_recover_s": int(el),
    "polls": int(polls), "failing_legs": [x for x in failing.split(",") if x],
    "labels": json.loads(labels),
    "conductor_receipt_max_s": {"recovering": numeric_or_none(rr), "survivor": numeric_or_none(rs)},
    "conductor_receipt_scope": {"recovering": rscope_rec or "unknown", "survivor": rscope_surv or "unknown"},
    "zome_path": zome or "unknown",
}
# One open/write, not two separate appends: two calls each do their own
# open()/close() against the same path, so a concurrent recovery run's
# append (this loop is meant to run per-peer, potentially overlapping other
# invocations against the same MESH_DIR) can interleave a record's JSON body
# with its own trailing newline write. Single write == one atomic append.
with open(p, "a") as fh:
    fh.write(json.dumps(record) + "\n")
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
# stdout silenced, stderr preserved (Minor-2): _validate_peer_transports prints
# a MESH_PEER_TRANSPORTS refusal to stderr and returns non-zero WITHOUT raising
# under `set -u` (an undefined-command substitution inside $(...) is not a
# nounset violation) — `2>&1 >/dev/null` would still swallow it (redirection
# order matters: that form dups stdout to the CURRENT stderr target first,
# then redirects stdout to /dev/null, leaving stderr untouched-but-unmerged is
# fine, but the earlier `>/dev/null 2>&1` sent BOTH to /dev/null). The refusal
# used to surface only much later as an unrelated "PEERS: unbound variable".
source "$SCRIPT_DIR/hc-mesh.sh" >/dev/null
set -u
[ -n "${PEERS:-}" ] || { echo "hc-mesh.sh refused to source (see message above)" >&2; exit 2; }
# Refuse BEFORE any kill/wipe if the transport-identity lookup this script
# depends on isn't landed yet. Under `set -u`, an undefined *command* inside
# a `$(...)` substitution is not a `set -u` violation — it prints
# "command not found" to stderr, the substitution captures empty output, and
# execution CONTINUES with t_rec="". That would let the destructive step run
# with a silently-empty transport-identity field instead of stopping.
command -v peer_transport >/dev/null 2>&1 || {
  echo "hc-mesh.sh has no peer_transport (per-peer transport not landed) — refusing to inflict loss" >&2
  exit 5
}
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
[ -n "$t_rec" ] || { echo "empty transport for $peer — refusing" >&2; exit 5; }

snap="$(mktemp)"; recovery_snapshot "$sport" > "$snap" || { echo "survivor $survivor:$sport unreadable" >&2; exit 3; }
rlog "shape=$shape peer=$peer survivors=${#survivors[@]} survivor=$survivor rows=$(python3 -c 'import json,sys;print(len(json.load(open(sys.argv[1]))["rows"]))' "$snap") transports survivor=$t_surv recovering=$t_rec"

# Capture BEFORE the kill (Critical-2), never after: restart_storage's live
# branch — the one that trusts /proc/<pid>/environ over its own stale-capture
# fallback — is only reachable while the pid is still alive. Refuses (rc 5)
# rather than inflict loss the recovery step has no way to undo.
pid="$(storage_pid_for_port "$rport")"
recovery_capture_peer "$peer" "$pid" || exit 5
[ -n "$pid" ] && { kill "$pid"; for _ in $(seq 1 15); do kill -0 "$pid" 2>/dev/null || break; sleep 1; done; kill -0 "$pid" 2>/dev/null && kill -9 "$pid"; }
wipe=(sync.sled content.db content.db-shm content.db-wal graph.db blobs blobs_iroh cache contest-backoff.json)
[ "$shape" = cold ] && wipe+=(identity.key iroh.key)
for f in "${wipe[@]}"; do rm -rf "${MESH_DIR:?}/$peer/$f"; done
rlog "loss inflicted: $shape wipe of ${#wipe[@]} entries under $MESH_DIR/$peer"

# Restart diagnostics are not discarded (Critical-2): restart_storage's own
# probe_zome_paths verdict ("zome path alive" / "ZOME CALLS ARE DEAD" /
# "inconclusive") is the only signal that distinguishes "serving" from "able
# to anchor" — swallowing it hid the exact failure class this harness exists
# to surface.
restart_log="$MESH_DIR/storage-restart/$peer.restart.log"
MESH_RESTART_APPLY_PROFILE=1 restart_storage "$peer" > "$restart_log" 2>&1
restart_rc=$?
[ "$restart_rc" -ne 0 ] && rlog "restart_storage reported non-zero; polling anyway"
while IFS= read -r rline; do rlog "restart: $rline"; done < <(tail -3 "$restart_log")
zome_verdict="unknown"
zome_line="$(grep -E "^  ${peer}[[:space:]]" "$restart_log" | tail -1)"
case "$zome_line" in
  *"zome path alive"*) zome_verdict="alive" ;;
  *"ZOME CALLS ARE DEAD"*) zome_verdict="dead" ;;
  *inconclusive*) zome_verdict="inconclusive" ;;
esac

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
rcpt_rec="$(receipt_max "$peer" "$t0")"; rcpt_surv="$(receipt_max "$survivor" "$t0")"
rcpt_scope_rec="$(receipt_scope_for "$peer")"; rcpt_scope_surv="$(receipt_scope_for "$survivor")"
fmt_receipt() { [ "$1" = "null" ] && echo "none" || echo "${1}s"; }
rlog "conductor receipt latency max during recovery: recovering=$(fmt_receipt "$rcpt_rec") survivor=$(fmt_receipt "$rcpt_surv")"
RECOVERY_SURVIVORS="${#survivors[@]}" recovery_write_record "$MESH_DIR/recovery-timeline.jsonl" "$shape" "$peer" "$survivor" "$t_surv" "$t_rec" "$recovered" "$el" "$polls" "$failing" "$labels" "$rcpt_rec" "$rcpt_surv" "$rcpt_scope_rec" "$rcpt_scope_surv" "$zome_verdict"
rm -f "$snap"
[ "$recovered" -eq 1 ]
