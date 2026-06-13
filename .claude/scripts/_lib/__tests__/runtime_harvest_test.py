#!/usr/bin/env python3
"""Tests for runtime_harvest (the elevate-arm pure core: predicates + ledger
reconcile). Run: python3 .claude/scripts/_lib/__tests__/runtime_harvest_test.py
(exit 0 = pass). pytest is NOT installed — this is the bespoke harness."""
import json
import os
import sys
from pathlib import Path

here = Path(__file__).resolve()
for _ in range(8):
    if (here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(here / ".claude" / "scripts"))
        break
    here = here.parent
from _lib import runtime_harvest as rh  # noqa: E402

_p = 0


def check(label, cond):
    global _p
    assert cond, f"FAIL: {label}"
    _p += 1
    print(f"  ✅ {label}")


# ── normalize: build-invariant error-line normalization ──
check("normalize collapses whitespace", rh.normalize("a   b\tc") == "a b c")
check("normalize masks poll counts", rh.normalize("open for 7 polls") == "open for # polls")
check("normalize masks durations", rh.normalize("lag 42s") == "lag #")
check("normalize masks timestamps",
      rh.normalize("at 2026-06-13T07:34:28") == "at #")

# ── fingerprint: stable, node/class lowered, 12-hex ──
fp1 = rh.fingerprint("alpha", "self-heal-exhaustion", "render-degenerate")
check("fingerprint is 12 hex", len(fp1) == 12 and all(c in "0123456789abcdef" for c in fp1))
check("fingerprint stable across reruns",
      fp1 == rh.fingerprint("alpha", "self-heal-exhaustion", "render-degenerate"))
check("fingerprint node-cased-insensitive",
      fp1 == rh.fingerprint("ALPHA", "self-heal-exhaustion", "render-degenerate"))
check("fingerprint differs by node",
      fp1 != rh.fingerprint("jessica", "self-heal-exhaustion", "render-degenerate"))
check("fingerprint count-churn invariant (provenance normalized)",
      rh.fingerprint("alpha", "self-heal-exhaustion", "circuit open 7 polls")
      == rh.fingerprint("alpha", "self-heal-exhaustion", "circuit open 9 polls"))


# ── evaluate: render-degenerate predicate (LANDED-today signal) ──
def _win(node, samples):
    return {"node": node, "samples": samples}


def _render(rate, stalled=0, timed=0):
    return {"render": {"degenerateRate": rate, "stalled": stalled, "timedOut": timed}}


# sustained high degenerateRate across DEGEN_POLLS -> one finding
hot = _win("alpha", [_render(0.40, 5, 1)] * rh.DEGEN_POLLS)
f_hot = rh.evaluate(hot)
check("render-degenerate fires when sustained",
      any(f["provenance"] == "render-degenerate" for f in f_hot))
check("render-degenerate finding carries node+class",
      f_hot[0]["node"] == "alpha" and f_hot[0]["class"] == rh.CLASS)

# single hot poll (< DEGEN_POLLS) -> no finding
blip = _win("alpha", [_render(0.40, 5, 1)])
check("render-degenerate silent on a single blip",
      not any(f["provenance"] == "render-degenerate" for f in rh.evaluate(blip)))

# healthy (low rate) -> no finding
cool = _win("alpha", [_render(0.01)] * rh.DEGEN_POLLS)
check("render-degenerate silent when healthy",
      not any(f["provenance"] == "render-degenerate" for f in rh.evaluate(cool)))

# absent render field -> no finding (field-presence-tolerant)
empty = _win("alpha", [{}] * rh.DEGEN_POLLS)
check("render-degenerate silent when field absent", rh.evaluate(empty) == [])


# ── evaluate: circuit-open predicate (PENDING /admin/self-healing) ──
def _sh(upstreams=None, admission=None, projector=None):
    d = {}
    if upstreams is not None:
        d["upstreams"] = upstreams
    if admission is not None:
        d["admission"] = admission
    if projector is not None:
        d["projector"] = projector
    return d


open_win = _win("alpha", [_sh(upstreams=[{"endpoint": "storage", "circuit": "open"}])] * rh.OPEN_POLLS)
check("circuit-open fires when open >= N polls",
      any(f["provenance"].startswith("circuit:") for f in rh.evaluate(open_win)))
recov = _win("alpha", [_sh(upstreams=[{"endpoint": "storage", "circuit": "open"}]),
                       _sh(upstreams=[{"endpoint": "storage", "circuit": "closed"}]),
                       _sh(upstreams=[{"endpoint": "storage", "circuit": "open"}])])
check("circuit-open silent when not consecutive",
      not any(f["provenance"].startswith("circuit:") for f in rh.evaluate(recov)))

# ── evaluate: admission-shed predicate ──
shed = _win("alpha", [_sh(admission={"shedTotal": 10}), _sh(admission={"shedTotal": 14}),
                      _sh(admission={"shedTotal": 21})])
check("admission-shed fires on rising shedTotal",
      any(f["provenance"] == "admission-shed" for f in rh.evaluate(shed)))
flat = _win("alpha", [_sh(admission={"shedTotal": 10})] * rh.SHED_POLLS)
check("admission-shed silent when shedTotal flat",
      not any(f["provenance"] == "admission-shed" for f in rh.evaluate(flat)))

# ── evaluate: projector-lag predicate ──
lag = _win("alpha", [_sh(projector={"caughtUp": False, "lagSeconds": 40})] * rh.LAG_POLLS)
check("projector-lag fires when not caught up >= N polls",
      any(f["provenance"].startswith("projector:") for f in rh.evaluate(lag)))
caught = _win("alpha", [_sh(projector={"caughtUp": True, "lagSeconds": 2})] * rh.LAG_POLLS)
check("projector-lag silent when caught up",
      not any(f["provenance"].startswith("projector:") for f in rh.evaluate(caught)))

# absent /admin/self-healing block -> none of the pending predicates fire
absent = _win("alpha", [{"render": {"degenerateRate": 0.0}}] * rh.WINDOW)
check("pending predicates silent when self-healing block absent",
      not any(f["provenance"].startswith(("circuit:", "projector:")) or
              f["provenance"] == "admission-shed" for f in rh.evaluate(absent)))


# ── reconcile: idempotent append + closure-by-disappearance ──
def _finding(fp, prov="render-degenerate"):
    return {"fp": fp, "node": "alpha", "class": rh.CLASS, "provenance": prov,
            "line": "x"}


# new fp -> appended as open, returned as NEW
new, bumped, closed = rh.reconcile([], [_finding("aaa")], 100)
check("reconcile appends new fp", len(new) == 1 and new[0]["status"] == "open")
check("reconcile new fp seen=1 + poll watermarks",
      new[0]["seen"] == 1 and new[0]["first_poll"] == 100 and new[0]["last_poll"] == 100)

# re-run same finding next poll -> NOT new (idempotent), bumped instead
entries = list(new)
new2, bumped2, closed2 = rh.reconcile(entries, [_finding("aaa")], 101)
check("reconcile never double-files a known fp", new2 == [])
check("reconcile bumps seen + last_poll", bumped2 and bumped2[0]["seen"] == 2
      and bumped2[0]["last_poll"] == 101)

# absent for CLOSE_STREAK polls -> closed (deleted), not status-flipped
e = [{"fp": "bbb", "node": "alpha", "class": rh.CLASS, "provenance": "x", "line": "x",
      "status": "open", "seen": 1, "first_poll": 1, "last_poll": 1, "clean_poll_streak": 0}]
for i in range(rh.CLOSE_STREAK):
    new3, bumped3, closed3 = rh.reconcile(e, [], 10 + i)
check("reconcile closes by disappearance (deletes the line)",
      closed3 and closed3[0]["fp"] == "bbb" and not any(x["fp"] == "bbb" for x in e))

# blocked fp present again -> still NOT new (structural suppression, any status)
blocked = [{"fp": "ccc", "node": "alpha", "class": rh.CLASS, "provenance": "x",
            "line": "x", "status": "blocked", "seen": 5, "first_poll": 1,
            "last_poll": 9, "clean_poll_streak": 0}]
n4, _, _ = rh.reconcile(blocked, [_finding("ccc")], 50)
check("reconcile suppresses re-dispatch for blocked fp", n4 == [])

# ── shell smoke (Task 5): degrade-quiet on an unreachable node ──
import subprocess  # noqa: E402

_root = None
_h = Path(__file__).resolve()
for _ in range(8):
    if (_h / ".claude" / "scripts" / "runtime-harvest.py").is_file():
        _root = _h
        break
    _h = _h.parent
check("runtime-harvest.py shell exists", _root is not None)
_r = subprocess.run(
    ["python3", str(_root / ".claude" / "scripts" / "runtime-harvest.py"),
     "--nodes", "doesnotexist", "--base", "http://127.0.0.1:9"],
    capture_output=True, text=True, timeout=60)
check("shell degrades quietly on unreachable node (exit 0)", _r.returncode == 0)

# ── shell write side (Task 6): hot node files a finding + idempotent + dispatch ──
import http.server  # noqa: E402
import threading  # noqa: E402
import tempfile  # noqa: E402

_hot = json.dumps({"total": 100, "stalled": 40, "timedOut": 5, "degenerateRate": 0.45})


class _Hot(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        body = _hot if self.path == "/admin/render-stats" else "{}"
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(body.encode())

    def log_message(self, *a):
        pass


_srv = http.server.HTTPServer(("127.0.0.1", 0), _Hot)
threading.Thread(target=_srv.serve_forever, daemon=True).start()
_port = _srv.server_address[1]
_tmp = tempfile.mkdtemp()
_env = dict(os.environ, CLAUDE_PROJECT_DIR=_tmp)
os.makedirs(os.path.join(_tmp, ".claude", "data"), exist_ok=True)
_script = str(_root / ".claude" / "scripts" / "runtime-harvest.py")
for _ in range(rh.DEGEN_POLLS):
    subprocess.run(["python3", _script, "--nodes", "t",
                    "--base", f"http://127.0.0.1:{_port}"],
                   env=_env, capture_output=True, text=True, timeout=60)
_ledger = os.path.join(_tmp, ".claude", "data", "runtime-findings.jsonl")
_lines = [json.loads(x) for x in open(_ledger)] if os.path.exists(_ledger) else []
check("hot node files a self-heal-exhaustion finding",
      any(e["class"] == rh.CLASS and e["provenance"] == "render-degenerate"
          for e in _lines))
# idempotent: re-polling once more does NOT add a second line for the same fp
_before = len(_lines)
subprocess.run(["python3", _script, "--nodes", "t", "--base", f"http://127.0.0.1:{_port}"],
               env=_env, capture_output=True, text=True, timeout=60)
_after = len([json.loads(x) for x in open(_ledger)])
check("re-poll never double-files (idempotent ledger)", _after == _before)
# dispatch directive is emitted in --hook mode on a fresh ledger
_tmp2 = tempfile.mkdtemp()
_env2 = dict(os.environ, CLAUDE_PROJECT_DIR=_tmp2)
os.makedirs(os.path.join(_tmp2, ".claude", "data"), exist_ok=True)
for _ in range(rh.DEGEN_POLLS - 1):
    subprocess.run(["python3", _script, "--nodes", "t", "--base", f"http://127.0.0.1:{_port}"],
                   env=_env2, capture_output=True, text=True, timeout=60)
_hk = subprocess.run(["python3", _script, "--hook", "--nodes", "t",
                      "--base", f"http://127.0.0.1:{_port}"],
                     env=_env2, capture_output=True, text=True, timeout=60)
check("hook mode emits runtime-triage dispatch directive on new fp",
      "runtime-triage" in _hk.stdout and _hk.returncode == 0)
_srv.shutdown()

# ── agent definition (Task 7): presence + header contract ──
_agent = _root / ".claude" / "agents" / "runtime-triage.md"
check("runtime-triage agent definition exists", _agent.is_file())
_txt = _agent.read_text() if _agent.is_file() else ""
check("runtime-triage is model: opus", "model: opus" in _txt)
check("runtime-triage name matches dispatch", "name: runtime-triage" in _txt)
check("runtime-triage reads the runtime ledger", "runtime-findings.jsonl" in _txt)
check("runtime-triage names the backlog convention",
      "genesis/data/timeline/backlog/self-heal-" in _txt)
check("runtime-triage deletes-on-close (rides the pattern)",
      "DELETE" in _txt and "blocked" in _txt)

print(f"\n  {_p} assertions passed ✅")
