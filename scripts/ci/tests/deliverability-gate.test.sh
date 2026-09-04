#!/usr/bin/env bash
# scripts/ci/tests/deliverability-gate.test.sh — run: bash scripts/ci/tests/deliverability-gate.test.sh
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
gate="$here/../deliverability-gate.sh"
port=18099
python3 - "$port" <<'PY' &
import sys, http.server
port = int(sys.argv[1])
class H(http.server.BaseHTTPRequestHandler):
    def _v(self):
        if "broken" in self.path: return ("broken", "missing-asset:main-EAKNZDUP.js")
        if "unjudged" in self.path: return ("not-judged", "not-held")
        return ("boots", None)
    def do_GET(self):
        v, r = self._v()
        self.send_response(200); self.send_header("X-Deliverability", v)
        if r: self.send_header("X-Deliverability-Reason", r)
        self.end_headers(); self.wfile.write(b"<html></html>")
    def do_HEAD(self):
        v, r = self._v()
        self.send_response(200); self.send_header("X-Deliverability", v)
        if r: self.send_header("X-Deliverability-Reason", r)
        self.end_headers()
    def log_message(self, *a): pass
http.server.HTTPServer(("127.0.0.1", port), H).serve_forever()
PY
srv=$!
trap 'kill $srv' EXIT
sleep 1
base="http://127.0.0.1:$port"

bash "$gate" "$base" "sha256-boots" 1 && echo "PASS boots"
if bash "$gate" "$base" "sha256-broken" 1; then echo "FAIL broken should exit 2"; exit 1; else rc=$?; [ "$rc" -eq 2 ] && echo "PASS broken rc=2"; fi
bash "$gate" "$base" "sha256-unjudged" 1 && echo "PASS not-judged is advisory"
if DELIVERABILITY_GATE=strict bash "$gate" "$base" "sha256-unjudged" 1; then echo "FAIL strict not-judged should exit 3"; exit 1; else rc=$?; [ "$rc" -eq 3 ] && echo "PASS strict rc=3"; fi
echo "ALL PASS"
