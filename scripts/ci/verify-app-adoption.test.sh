#!/usr/bin/env bash
# Regression coverage for verify-app-adoption.sh against a real local HTTP
# server serving per-peer adoption reports (native-delivery sprint Lane N5).
#
#   1. every peer applied the release (either wire spelling) → exit 0;
#   2. a peer refused it app_bundle_cannot_boot → exit 1 (FAILURE) at once,
#      without spending the bound;
#   3. a peer still on a transient refusal when the bound elapses → exit 3
#      (UNSTABLE), naming the peer, its state and its reason;
#   4. a peer that applied an OLDER release is not adopted.

set -euo pipefail

REPO_ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
SCRIPT="${REPO_ROOT}/scripts/ci/verify-app-adoption.sh"
TEST_ROOT="$(mktemp -d)"
SERVER_PID=""
CHANNEL="runtime:app-bundle:alpha:dev"
CID="uhCkkTheRelease"

cleanup() {
  [ -n "$SERVER_PID" ] && kill "$SERVER_PID" 2>/dev/null || true
  rm -rf "${TEST_ROOT}"
}
trap cleanup EXIT

# GET /db/p2p/adoption?peer=<name> serves <reports>/<name>.json.
mkdir -p "${TEST_ROOT}/reports"
cat > "${TEST_ROOT}/server.py" <<'PY'
import http.server, os, sys, urllib.parse
ROOT = sys.argv[2]
class Handler(http.server.BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass
    def do_GET(self):
        url = urllib.parse.urlparse(self.path)
        peer = urllib.parse.parse_qs(url.query).get("peer", [""])[0]
        file = os.path.join(ROOT, peer + ".json")
        if url.path == "/db/p2p/adoption" and os.path.exists(file):
            data = open(file, "rb").read()
            self.send_response(200)
        else:
            data = b"{}"
            self.send_response(404)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)
http.server.ThreadingHTTPServer(("127.0.0.1", int(sys.argv[1])), Handler).serve_forever()
PY
PORT=$(( 20000 + RANDOM % 20000 ))
python3 "${TEST_ROOT}/server.py" "${PORT}" "${TEST_ROOT}/reports" &
SERVER_PID=$!
DOORWAY="http://127.0.0.1:${PORT}"
for _ in $(seq 1 50); do
  curl -s -o /dev/null "${DOORWAY}/" && break
  sleep 0.1
done

report() { # <peer> <row-json>
  echo "{\"channels\":[$2]}" > "${TEST_ROOT}/reports/$1.json"
}
applied_row="{\"channelId\":\"${CHANNEL}\",\"appliedRelease\":{\"cid\":\"${CID}\"},\"verdict\":{\"state\":\"applied\",\"ok\":true,\"releaseCid\":\"${CID}\",\"vehicle\":\"app_mount_pointers\",\"alreadyCurrent\":false}}"
verdict_only_row="{\"channelId\":\"${CHANNEL}\",\"appliedRelease\":null,\"verdict\":{\"state\":\"applied\",\"ok\":true,\"releaseCid\":\"${CID}\"}}"
cannot_boot_row="{\"channelId\":\"${CHANNEL}\",\"appliedRelease\":null,\"verdict\":{\"state\":\"refused\",\"refusal\":{\"reason\":\"app_bundle_cannot_boot\",\"detail\":\"the browser zip for 'lamad-spa' cannot boot (missing-asset:main-X.js)\",\"arm\":\"verify\",\"transient\":false}}}"
transient_row="{\"channelId\":\"${CHANNEL}\",\"appliedRelease\":null,\"verdict\":{\"state\":\"refused\",\"refusal\":{\"reason\":\"artifact_unavailable\",\"detail\":\"not replicated yet\",\"arm\":\"fetch\",\"transient\":true}}}"
older_row="{\"channelId\":\"${CHANNEL}\",\"appliedRelease\":{\"cid\":\"uhCkkOlder\"},\"verdict\":{\"state\":\"applied\",\"ok\":true,\"releaseCid\":\"uhCkkOlder\"}}"

run() {
  VERIFY_ADOPTION_BUDGET_SECS=2 VERIFY_ADOPTION_POLL_SECS=1 \
    bash "${SCRIPT}" "${DOORWAY}" "${CID}" matthew jessica
}
fail() { echo "FAIL: $*" >&2; exit 1; }

# 1. both adopted, one through each wire spelling.
report matthew "${applied_row}"
report jessica "${verdict_only_row}"
out="$(run 2>&1)" || fail "adopted: expected exit 0: $out"
[ "$(grep -c '^APP-ADOPTED ' <<<"$out")" -eq 2 ] || fail "adopted: one line per peer: $out"
echo "ok 1 - every peer applied the release"

# 2. a bundle that cannot boot is FAILURE, at once.
report jessica "${cannot_boot_row}"
start=$(date +%s)
set +e; out="$(VERIFY_ADOPTION_BUDGET_SECS=60 VERIFY_ADOPTION_POLL_SECS=1 \
  bash "${SCRIPT}" "${DOORWAY}" "${CID}" matthew jessica 2>&1)"; rc=$?; set -e
[ "$rc" -eq 1 ] || fail "cannot boot: expected exit 1, got $rc: $out"
grep -q "APP-CANNOT-BOOT jessica detail=.*missing-asset:main-X.js" <<<"$out" || fail "cannot boot: names the peer and why: $out"
[ $(( $(date +%s) - start )) -lt 30 ] || fail "cannot boot: must not spend the bound"
echo "ok 2 - app_bundle_cannot_boot is FAILURE without waiting"

# 3. a transient refusal past the bound is UNSTABLE, named.
report jessica "${transient_row}"
set +e; out="$(run 2>&1)"; rc=$?; set -e
[ "$rc" -eq 3 ] || fail "transient: expected exit 3, got $rc: $out"
grep -q "APP-NOT-ADOPTED jessica state=refused reason=artifact_unavailable" <<<"$out" || fail "transient: names state and reason: $out"
echo "ok 3 - a transient refusal past the bound is UNSTABLE"

# 4. an older release is not this release; a missing report is unreachable.
report jessica "${older_row}"
rm -f "${TEST_ROOT}/reports/matthew.json"
set +e; out="$(run 2>&1)"; rc=$?; set -e
[ "$rc" -eq 3 ] || fail "older: expected exit 3, got $rc: $out"
grep -q "APP-NOT-ADOPTED jessica state=applied" <<<"$out" || fail "older: jessica is not adopted: $out"
grep -q "APP-NOT-ADOPTED matthew state=unreachable reason=HTTP 404" <<<"$out" || fail "older: matthew unreachable: $out"
echo "ok 4 - an older release, or no report, is not adoption"
