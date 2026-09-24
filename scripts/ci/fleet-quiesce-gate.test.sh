#!/usr/bin/env bash
# Regression coverage for fleet-quiesce-gate.sh against a real local HTTP server.
#
#   1. oversized /metrics (> MAX_ARG_STRLEN, 128 KiB) still measures — the
#      edge #1471-#1475 defect passed the body to python3 through an env var,
#      every exec failed with "Argument list too long", and the gate polled its
#      full deadline reading nothing.
#   2. an evaluator that cannot run ends the gate as GATE-DEFECT (exit 4) after
#      QUIESCE_MAX_EVAL_ERRORS polls instead of burning the deadline blind.

set -euo pipefail

REPO_ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
SCRIPT="${REPO_ROOT}/scripts/ci/fleet-quiesce-gate.sh"
TEST_ROOT="$(mktemp -d)"
SERVER_PID=""

cleanup() {
  [ -n "$SERVER_PID" ] && kill "$SERVER_PID" 2>/dev/null || true
  rm -rf "${TEST_ROOT}"
}
trap cleanup EXIT

cat > "${TEST_ROOT}/server.py" <<'PY'
import http.server, sys

PADDING = "".join(
    f'elohim_test_padding_series{{stream="s{i:06d}",peer="alpha"}} {i}\n' for i in range(6000)
)
sweeps = 0

class Handler(http.server.BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def send(self, body):
        data = body.encode()
        self.send_response(200)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        global sweeps
        if self.path.endswith("/p2p/status"):
            self.send('{"pull":{"caughtUp":true,"total":3,"pending":0,"failed":0}}')
        elif self.path.endswith("/metrics"):
            sweeps += 1
            self.send(
                PADDING
                + 'elohim_projection_reconcile_converged 1\n'
                + f'elohim_projection_reconcile_sweeps_total {sweeps}\n'
                + 'elohim_projection_reconcile_divergent_actionable 0\n'
                + 'elohim_projection_reconcile_converged_blocked_by{term="divergent_actionable"} 0\n'
                + 'elohim_projection_reconcile_converged_blocked_by{term="unmeasured"} 0\n'
            )
        elif "/db/content/" in self.path:
            self.send("{}")
        else:
            self.send_response(404)
            self.end_headers()

server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
print(server.server_address[1], flush=True)
server.serve_forever()
PY

python3 "${TEST_ROOT}/server.py" > "${TEST_ROOT}/port" &
SERVER_PID=$!
for _ in $(seq 1 50); do
  [ -s "${TEST_ROOT}/port" ] && break
  sleep 0.1
done
PORT="$(cat "${TEST_ROOT}/port")"
BASE="http://127.0.0.1:${PORT}"

metrics_bytes=$(curl -fsS "${BASE}/metrics" | wc -c)
if [ "$metrics_bytes" -le 131072 ]; then
  echo "FAIL: fixture /metrics is ${metrics_bytes} bytes — must exceed MAX_ARG_STRLEN (131072) to cover the defect"
  exit 1
fi

run_gate() {
  QUIESCE_DEADLINE_SECS=30 QUIESCE_POLL_SECS=1 QUIESCE_SUSTAIN_SECS=1 \
    bash "$SCRIPT" "$BASE" "$BASE" elohim-host-landing "$BASE" "$BASE" \
    > "${TEST_ROOT}/out" 2>&1
}

# 1. Oversized /metrics measures and passes.
rc=0
run_gate || rc=$?
if [ "$rc" -ne 0 ] || ! grep -q "A-QUIESCED" "${TEST_ROOT}/out"; then
  echo "FAIL: oversized /metrics (${metrics_bytes} bytes) did not quiesce (exit ${rc})"
  cat "${TEST_ROOT}/out"
  exit 1
fi
if grep -q "Argument list too long" "${TEST_ROOT}/out"; then
  echo "FAIL: evaluator still hits E2BIG"
  cat "${TEST_ROOT}/out"
  exit 1
fi
echo "ok 1 - ${metrics_bytes}-byte /metrics body is measured (A-QUIESCED)"

# 2. An evaluator that cannot run ends as GATE-DEFECT, fast.
mkdir -p "${TEST_ROOT}/bin"
REAL_PY="$(command -v python3)"
cat > "${TEST_ROOT}/bin/python3" <<SH
#!/usr/bin/env bash
# Fail only the heredoc evaluator (python3 -); pass every other call through.
if [ "\${1:-}" = "-" ]; then echo "simulated evaluator crash" >&2; exit 1; fi
exec "${REAL_PY}" "\$@"
SH
chmod +x "${TEST_ROOT}/bin/python3"
rc=0
PATH="${TEST_ROOT}/bin:${PATH}" QUIESCE_MAX_EVAL_ERRORS=2 run_gate || rc=$?
if [ "$rc" -ne 4 ] || ! grep -q "GATE-DEFECT" "${TEST_ROOT}/out"; then
  echo "FAIL: broken evaluator should exit 4 with GATE-DEFECT (got exit ${rc})"
  cat "${TEST_ROOT}/out"
  exit 1
fi
echo "ok 2 - broken evaluator ends as GATE-DEFECT (exit 4) after 2 polls"
