#!/usr/bin/env bash
# Live-process fixture for release-adoption exe-slot consumption. No Holochain
# or real storage binary is needed; the staged candidate is a tiny HTTP server.
set -u

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
fail=0
t() { if eval "$2"; then echo "ok   $1"; else echo "FAIL $1"; fail=1; fi; }

tmp="$(mktemp -d)"
old_pid=""
cleanup() {
  local pid=""
  [ -s "$MESH_DIR/pids/storage-unit" ] && read -r pid _ < "$MESH_DIR/pids/storage-unit"
  [[ "$pid" =~ ^[0-9]+$ ]] && kill "$pid" 2>/dev/null || true
  [[ "$old_pid" =~ ^[0-9]+$ ]] && kill "$old_pid" 2>/dev/null || true
  rm -rf "$tmp"
}
trap cleanup EXIT

export MESH_DIR="$tmp/mesh"
export MESH_PEERS=unit
export MESH_TRANSPORT_BACKEND=libp2p
source "$here/../hc-mesh.sh" >/dev/null
fixture_port=$((23000 + $$ % 1000))
http_port() { echo "$fixture_port"; }
probe_zome_paths() { return 0; }
refresh_fixture_pids() { :; }
mkdir -p "$LOGDIR"

# The current peer is a real process whose argv has the exact shape the restart
# arm resolves. Its executable is the known-good fallback for a failed slot.
python3 -c 'import time; marker = "elohim-storage"; time.sleep(300)' \
  --http-port "$fixture_port" &
old_pid=$!
sleep 0.1

slot="$(release_adoption_slot_for unit)"
mkdir -p "$(dirname "$slot")"
cat > "$slot" <<'CANDIDATE'
#!/usr/bin/env bash
port="$2"
exec python3 -c '
from http.server import BaseHTTPRequestHandler, HTTPServer
import sys
class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200)
        self.end_headers()
        self.wfile.write(b"staged")
    def log_message(self, *_):
        pass
HTTPServer(("127.0.0.1", int(sys.argv[1])), Handler).serve_forever()
' "$port"
CANDIDATE
chmod 0755 "$slot"
printf '{"releaseCid":"sha256-fixture","pendingRestart":true}\n' > "$slot.json"

out="$(restart_storage unit 2>&1)"; rc=$?
exefile="$MESH_DIR/storage-restart/unit.exe"
recorded="$(head -1 "$exefile" 2>/dev/null)"
t "restart boots the staged storage candidate (rc=$rc)" \
  '[ "$rc" -eq 0 ] && curl -fsS -m 2 "http://127.0.0.1:$fixture_port/health" | grep -qx staged'
t "live-peer resolution prefers the per-peer release slot" \
  '[[ "$out" == *"staged release candidate=$slot"* ]]'
t "successful boot consumes .next and exe record names its archived slot" \
  '[ ! -e "$slot" ] && [[ "$recorded" == "$slot.applied-"* ]] && [ -x "$recorded" ]'
t "the sidecar receipt moves with the applied candidate" \
  'compgen -G "$slot.json.applied-*" >/dev/null'

# A failed candidate is also disarmed, but the exe record returns to the last
# known-good binary. That makes an operator retry a recovery, not a boot loop.
failed_slot="$tmp/failed/elohim-storage.next"
failed_record="$tmp/failed.exe"
mkdir -p "$(dirname "$failed_slot")"
cp /bin/sleep "$failed_slot"
chmod 0755 "$failed_slot"
printf '%s\n' "$failed_slot" > "$failed_record"
archive_release_adoption_slot unit "$failed_slot" failed "$failed_record" /bin/sleep >/dev/null
t "failed candidate is disarmed so it cannot loop" \
  '[ ! -e "$failed_slot" ] && compgen -G "$failed_slot.failed-*" >/dev/null'
t "failed candidate restores the previous executable record" \
  '[ "$(head -1 "$failed_record")" = /bin/sleep ]'

exit "$fail"
