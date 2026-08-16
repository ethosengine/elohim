#!/usr/bin/env python3
"""hc-mesh-monitor — unified local-dev system monitor for the 3-peer mesh.

One page over the probe surfaces that already exist (this is a READER for
instruments the runtime already carries, never a new instrument):

  storage peers  :8090-8092   /health · /p2p/status · /metrics
  doorways       :8888/:8889  /health · /db/content/<landing>
  conductors     :4444/:4454/:4464   TCP liveness
  quiesce log    $MESH_DIR/quiesce-log.txt (harness verdict/rate history)

The "gate legs" panel mirrors scripts/ci/fleet-quiesce-gate.sh's PASS
predicate (A caughtUp-or-resolved-empty · actionable<=tol · unmeasured==0 ·
both doorways 200) so "the dashboard looks quiesced" and "the gate will
pass" are the same statement.

Serves on MESH_MONITOR_PORT (default 4210) — declared as the `mesh-monitor`
endpoint in devfile.yaml so Eclipse Che surfaces it as a dashboard link.
Run on demand: `just mesh monitor` (never from devfile postStart).

Stdlib only. Every probe is best-effort with a short timeout; a dead
component renders as down, never as an exception.
"""

import json
import os
import re
import socket
import urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

PORT = int(os.environ.get("MESH_MONITOR_PORT", "4210"))
MESH_DIR = os.environ.get("MESH_DIR", "/tmp/elohim-local-mesh")
CONTENT_ID = os.environ.get("MESH_QUIESCE_CONTENT_ID", "elohim-host-landing")
ACTIONABLE_TOL = int(os.environ.get("QUIESCE_ACTIONABLE_TOLERANCE", "2"))
PROBE_TIMEOUT = float(os.environ.get("MESH_MONITOR_PROBE_TIMEOUT", "1.5"))

PEERS = [
    {"name": "matthew", "http": 8090, "admin": 4444},
    {"name": "jessica", "http": 8091, "admin": 4454},
    {"name": "james", "http": 8092, "admin": 4464},
]
DOORWAYS = [
    {"name": "doorway-A", "port": 8888},
    {"name": "doorway-B", "port": 8889},
]

# The handful of gauges worth a dashboard cell; everything else stays in
# /metrics for whoever wants the firehose.
METRIC_PATTERNS = {
    "divergent_actionable": r"^elohim_projection_reconcile_divergent_actionable (\S+)",
    "sweeps": r"^elohim_projection_reconcile_sweeps_total (\S+)",
    "converged": r"^elohim_projection_reconcile_converged (\S+)",
    "accept_with_provenance": r'^elohim_trust_priced_adoptions_total\{depth="accept_with_provenance"\} (\S+)',
    "full_chain": r'^elohim_trust_priced_adoptions_total\{depth="full_chain"\} (\S+)',
    "adopt_peer_links": r'^elohim_content_canonical_links_minted_total\{source="adopt_peer"\} (\S+)',
    "swarm_manifests": r"^elohim_blob_swarm_manifests_received_total (\S+)",
    "swarm_composites": r"^elohim_blob_swarm_composite_completed_total (\S+)",
    "unmeasured": r'^elohim_projection_reconcile_converged_blocked_by\{term="unmeasured"\} (\S+)',
}


def http_get(url, want_json=False, want_text=False):
    try:
        with urllib.request.urlopen(url, timeout=PROBE_TIMEOUT) as r:
            if want_json:
                return r.status, json.loads(r.read())
            if want_text:
                return r.status, r.read().decode("utf-8", "replace")
            r.read(64)
            return r.status, None
    except Exception:
        return None, None


def tcp_alive(port):
    try:
        with socket.create_connection(("127.0.0.1", port), timeout=PROBE_TIMEOUT):
            return True
    except Exception:
        return False


def parse_metrics(text):
    out = {}
    if not text:
        return out
    for line in text.splitlines():
        for key, pat in METRIC_PATTERNS.items():
            if key in out:
                continue
            m = re.match(pat, line)
            if m:
                try:
                    out[key] = float(m.group(1))
                except ValueError:
                    pass
    return out


def pull_leg(status):
    """Mirror of fleet-quiesce-gate.sh caught_up(): true on caughtUp===true
    OR an initialized resolved-empty pull (total==0, pending==0, failed==0)."""
    if not isinstance(status, dict):
        return False
    p = status.get("pull")
    if not isinstance(p, dict):
        return False
    if p.get("caughtUp") is True:
        return True
    return p.get("total") == 0 and p.get("pending") == 0 and p.get("failed") == 0


def collect_state():
    state = {"peers": [], "doorways": [], "gate": {}, "quiesce_log": []}

    for peer in PEERS:
        base = f"http://127.0.0.1:{peer['http']}"
        health, _ = http_get(f"{base}/health")
        _, status = http_get(f"{base}/p2p/status", want_json=True)
        _, metrics_text = http_get(f"{base}/metrics", want_text=True)
        pr = status.get("projectionReconcile") if isinstance(status, dict) else None
        state["peers"].append(
            {
                "name": peer["name"],
                "http_port": peer["http"],
                "health": health,
                "conductor_admin": peer["admin"],
                "conductor_up": tcp_alive(peer["admin"]),
                "pull_leg": pull_leg(status),
                "drain": (status or {}).get("drain") if isinstance(status, dict) else None,
                "reconcile": {
                    k: pr.get(k)
                    for k in ("pending", "divergentAnchor", "healedTotal", "sweeps", "caughtUp", "converged")
                }
                if isinstance(pr, dict)
                else None,
                "metrics": parse_metrics(metrics_text),
            }
        )

    for dw in DOORWAYS:
        base = f"http://127.0.0.1:{dw['port']}"
        health, _ = http_get(f"{base}/health")
        content, _ = http_get(f"{base}/db/content/{CONTENT_ID}")
        state["doorways"].append(
            {"name": dw["name"], "port": dw["port"], "health": health, "content": content}
        )

    # Gate-leg mirror (storage-A scoped, same as the CI gate's predicate).
    a = state["peers"][0]
    m = a["metrics"]
    actionable = m.get("divergent_actionable")
    unmeasured = m.get("unmeasured")
    legs = {
        "a_caught_up": a["pull_leg"],
        "a_actionable_within_tol": actionable is not None and actionable <= ACTIONABLE_TOL,
        "a_unmeasured_zero": unmeasured == 0,
        "doorways_serving": all(d["content"] == 200 for d in state["doorways"]),
    }
    legs["would_pass"] = all(legs.values())
    legs["actionable"] = actionable
    legs["tolerance"] = ACTIONABLE_TOL
    state["gate"] = legs

    try:
        with open(os.path.join(MESH_DIR, "quiesce-log.txt")) as f:
            state["quiesce_log"] = f.read().splitlines()[-5:]
    except Exception:
        state["quiesce_log"] = []

    return state


PAGE = """<!doctype html>
<html><head><meta charset="utf-8"><title>hc-mesh monitor</title>
<style>
  body { background:#12141a; color:#cfd6e4; font:13px/1.5 ui-monospace,monospace;
         margin:0; padding:1.2rem; }
  h1 { font-size:15px; margin:0 0 .8rem; color:#e8edf7; }
  h1 small { color:#6b7488; font-weight:normal; }
  .grid { display:grid; grid-template-columns:repeat(auto-fit,minmax(270px,1fr));
          gap:.8rem; margin-bottom:.9rem; }
  .card { background:#1a1e28; border:1px solid #2a3040; border-radius:8px;
          padding: .7rem .9rem; }
  .card h2 { font-size:13px; margin:0 0 .4rem; color:#e8edf7; }
  .dot { display:inline-block; width:9px; height:9px; border-radius:50%;
         margin-right:.45em; }
  .up { background:#3fb26f; } .down { background:#d64f4f; } .warn { background:#d6a94f; }
  table { border-collapse:collapse; width:100%; }
  td { padding:.05rem .4rem .05rem 0; color:#9aa4b8; }
  td.v { color:#cfd6e4; text-align:right; }
  .gate { border-color:#3fb26f55; }
  .gate.fail { border-color:#d64f4f55; }
  .verdict { font-size:14px; margin-bottom:.3rem; }
  pre { background:#151821; border:1px solid #2a3040; border-radius:8px;
        padding:.6rem .8rem; overflow-x:auto; color:#9aa4b8; font-size:12px; }
  .ts { color:#6b7488; font-size:11px; }
</style></head><body>
<h1>hc-mesh monitor <small>— local 3-peer mesh · refreshes every 5s · <span id="ts" class="ts"></span></small></h1>
<div class="grid" id="gate-row"></div>
<div class="grid" id="peers"></div>
<div class="grid" id="doorways"></div>
<h1>quiesce log <small>(last 5 runs)</small></h1>
<pre id="qlog">loading…</pre>
<script>
const dot = ok => `<span class="dot ${ok===true?'up':ok===false?'down':'warn'}"></span>`;
const row = (k,v) => `<tr><td>${k}</td><td class="v">${v ?? '—'}</td></tr>`;
async function tick() {
  let s;
  try { s = await (await fetch('/api/state')).json(); }
  catch(e) { document.getElementById('ts').textContent = 'monitor unreachable'; return; }
  document.getElementById('ts').textContent = new Date().toLocaleTimeString();
  const g = s.gate;
  document.getElementById('gate-row').innerHTML = `
    <div class="card gate ${g.would_pass?'':'fail'}">
      <div class="verdict">${dot(g.would_pass)}CI gate would ${g.would_pass?'PASS':'FAIL'} <span class="ts">(same legs as fleet-quiesce-gate.sh)</span></div>
      <table>
        ${row(dot(g.a_caught_up)+'A pull caught-up / resolved-empty', g.a_caught_up)}
        ${row(dot(g.a_actionable_within_tol)+'A divergent_actionable &le; '+g.tolerance, g.actionable)}
        ${row(dot(g.a_unmeasured_zero)+'A unmeasured == 0', g.a_unmeasured_zero)}
        ${row(dot(g.doorways_serving)+'both doorways serve landing 200', g.doorways_serving)}
      </table>
    </div>`;
  document.getElementById('peers').innerHTML = s.peers.map(p => {
    const r = p.reconcile || {}, m = p.metrics || {};
    return `<div class="card">
      <h2>${dot(p.health===200)}${p.name} <span class="ts">:${p.http_port} · conductor ${p.conductor_up?'up':'DOWN'} :${p.conductor_admin}</span></h2>
      <table>
        ${row('reconcile pending', r.pending)} ${row('divergentAnchor', r.divergentAnchor)}
        ${row('healedTotal', r.healedTotal)} ${row('sweeps', r.sweeps)}
        ${row('converged', r.converged)} ${row('divergent_actionable', m.divergent_actionable)}
        ${row('accept_with_provenance', m.accept_with_provenance)} ${row('full_chain priced', m.full_chain)}
        ${row('adopt_peer links', m.adopt_peer_links)}
        ${row('swarm manifests/composites', (m.swarm_manifests??'—')+' / '+(m.swarm_composites??'—'))}
        ${row('drain', p.drain ? `${p.drain.published}/${p.drain.total}` : null)}
      </table></div>`;
  }).join('');
  document.getElementById('doorways').innerHTML = s.doorways.map(d => `
    <div class="card"><h2>${dot(d.health===200)}${d.name} <span class="ts">:${d.port}</span></h2>
      <table>${row('/health', d.health)} ${row('landing content', d.content)}</table></div>`).join('');
  document.getElementById('qlog').textContent = (s.quiesce_log||[]).join('\\n') || '(no runs recorded yet)';
}
tick(); setInterval(tick, 5000);
</script></body></html>
"""


class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == "/api/state":
            body = json.dumps(collect_state()).encode()
            ctype = "application/json"
        elif self.path in ("/", "/index.html"):
            body = PAGE.encode()
            ctype = "text/html; charset=utf-8"
        else:
            self.send_response(404)
            self.end_headers()
            return
        self.send_response(200)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *args):
        pass


if __name__ == "__main__":
    server = ThreadingHTTPServer(("0.0.0.0", PORT), Handler)
    print(f"hc-mesh-monitor: http://localhost:{PORT}  (Che endpoint: mesh-monitor)")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
