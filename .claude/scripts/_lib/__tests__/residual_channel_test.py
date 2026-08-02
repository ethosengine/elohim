#!/usr/bin/env python3
"""Tests for residual_channel — the C14 residual channel's intake + report half (plan P4.7).

Run: python3 .claude/scripts/_lib/__tests__/residual_channel_test.py  (exit 0 = pass).
pytest is NOT installed — this is the bespoke harness, same shape as runtime_harvest_test.py.

THE LOAD-BEARING CLAIM this file exists to prove: the residual channel BINDS the live
runtime-findings pipeline rather than forking it, and the pure core (_lib/runtime_harvest.py)
required ZERO code change to carry a second class. That is a claim about someone else's module,
so it is proven with fixtures against the UNMODIFIED core — never asserted in prose.
"""
import http.server
import json
import os
import subprocess
import sys
import tempfile
import threading
from pathlib import Path

here = Path(__file__).resolve()
_root = None
for _ in range(8):
    if (here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(here / ".claude" / "scripts"))
        _root = here
        break
    here = here.parent
from _lib import residual_channel as rc  # noqa: E402
from _lib import runtime_harvest as rh  # noqa: E402

_p = 0


def check(label, cond):
    global _p
    assert cond, f"FAIL: {label}"
    _p += 1
    print(f"  ✅ {label}")


# ── the fixture capsule: EXACTLY the one crates/seam-contracts/src/residual.rs tests, serialized
#    as its serde(rename_all = "camelCase") derive emits it. The two implementations are compared
#    to hand-written GOLDEN literals below, never to each other — a contract test that computes
#    both sides through one helper measures the helper (9e9c9d023, 2026-07-24).
CAPSULE = {
    "seam": "elohim-storage::head_adoption::decide_head_action",
    "residualKind": "declared-divergence-no-arm",
    "inputsDigest": "blake3:9f2c",
    "decisionState": {"kind": "raw", "value": "both sides declared, neither carries a candidate"},
    "context": [
        {"kind": "clock", "subject": "dht_link_ts", "value": "1785"},
        {"kind": "lag", "subject": "projector", "value": "42s"},
        {"kind": "peerState", "subject": "adam", "value": "declared=uhC0k"},
    ],
    "capsuleId": "",
    "observedAt": "",
}

GOLDEN_PROVENANCE = (
    "residual:elohim-storage::head_adoption::decide_head_action:declared-divergence-no-arm"
)

# ── namespace: the canon row's findings_namespace, hand-written here, derived in Rust ──
check("CLASS is the C14 canon findings_namespace", rc.CLASS == "concern:c14-witnessed-residual")
check("provenance matches the golden literal the Rust test also pins",
      rc.capsule_provenance(CAPSULE) == GOLDEN_PROVENANCE)
check("provenance is invariant to per-occurrence detail (fingerprint stability)",
      rc.capsule_provenance(dict(CAPSULE, inputsDigest="blake3:other",
                                 observedAt="2026-08-02T12:00:00Z", capsuleId="x"))
      == GOLDEN_PROVENANCE)
check("provenance discriminates two residual shapes at one seam",
      rc.capsule_provenance(dict(CAPSULE, residualKind="other-shape")) != GOLDEN_PROVENANCE)

# ── line: carries the context census + stays inside the ledger's 300-char budget ──
_line = rc.capsule_line(CAPSULE)
check("line carries the relative-context census", "clock=1" in _line and "lag=1" in _line
      and "peerState=1" in _line)
check("line carries the decision state", "raw=both sides declared" in _line)
check("line is bounded at 300 chars",
      len(rc.capsule_line(dict(CAPSULE,
                               decisionState={"kind": "raw", "value": "x" * 2000}))) == 300)
check("a capsule with no context says `ctx: none` rather than omitting the field",
      "ctx: none" in rc.capsule_line(dict(CAPSULE, context=[])))

# ── C14: witnessed, NEVER dropped — even when the capsule itself is garbage ──
_mixed = rc.findings_from_capsules("alpha", [CAPSULE, "not-an-object", {"seam": "s"}])
check("a malformed capsule is WITNESSED, not dropped",
      any(rc.MALFORMED_KIND in f["provenance"] for f in _mixed))
check("one malformed capsule never suppresses its well-formed siblings (per-row degradation)",
      any(f["provenance"] == GOLDEN_PROVENANCE for f in _mixed))
check("a capsule missing residualKind degrades to the malformed kind, keeping its seam",
      any(f["provenance"] == f"residual:s:{rc.MALFORMED_KIND}" for f in _mixed))
check("non-list capsule payload yields no findings (degrade quiet, C4)",
      rc.findings_from_capsules("alpha", None) == []
      and rc.findings_from_capsules("alpha", {"not": "a list"}) == [])

# ── C6b: N occurrences of one shape in ONE poll is ONE finding ──
check("within-poll duplicates collapse to one finding (idempotent effect)",
      len(rc.findings_from_capsules("alpha", [CAPSULE, dict(CAPSULE), dict(CAPSULE)])) == 1)

# ── finding SHAPE is exactly what runtime_harvest.evaluate emits ──
_f = rc.findings_from_capsules("alpha", [CAPSULE])[0]
check("finding carries node/class/provenance/line and nothing else",
      set(_f) == {"node", "class", "provenance", "line"})
check("finding class is the c14 namespace, not the exhaustion class",
      _f["class"] == rc.CLASS and _f["class"] != rh.CLASS)


# ══════════════════════════════════════════════════════════════════════════════════════════
# THE ANTI-FORK PROOF: the UNMODIFIED pure core carries a c14 finding identically.
# Every call below is into _lib/runtime_harvest.py, which this task changed by ZERO bytes.
# ══════════════════════════════════════════════════════════════════════════════════════════
_fp = rh.fingerprint(_f["node"], _f["class"], _f["provenance"])
check("core fingerprint accepts the c14 class unchanged (12 hex)",
      len(_fp) == 12 and all(c in "0123456789abcdef" for c in _fp))
check("class participates in the fingerprint — same provenance, different class, different fp",
      _fp != rh.fingerprint(_f["node"], rh.CLASS, _f["provenance"]))
check("fingerprint is stable across reruns",
      _fp == rh.fingerprint(_f["node"], _f["class"], _f["provenance"]))

_f = dict(_f, fp=_fp)
_entries = []
_new, _bumped, _closed = rh.reconcile(_entries, [_f], 1)
check("core reconcile appends a c14 finding as open/seen=1",
      len(_new) == 1 and _new[0]["status"] == "open" and _new[0]["class"] == rc.CLASS)
_new2, _bumped2, _closed2 = rh.reconcile(_entries, [_f], 2)
check("core reconcile never double-files a c14 fp (structural suppression)", _new2 == [])
check("core reconcile bumps seen/last_poll for a c14 fp",
      _bumped2 and _bumped2[0]["seen"] == 2 and _bumped2[0]["last_poll"] == 2)
for _i in range(rh.CLOSE_STREAK):
    _n3, _b3, _c3 = rh.reconcile(_entries, [], 10 + _i)
check("core closes a c14 finding by disappearance, same as any exhaustion finding",
      _c3 and _c3[0]["fp"] == _fp and _entries == [])

# mixed-class ledger: the two classes coexist without interfering
_exh = {"fp": "aaaaaaaaaaaa", "node": "alpha", "class": rh.CLASS,
        "provenance": "render-degenerate", "line": "x"}
_res = dict(_f)
_mixed_entries = []
rh.reconcile(_mixed_entries, [_exh, _res], 1)
check("one ledger carries both classes", len(_mixed_entries) == 2
      and {e["class"] for e in _mixed_entries} == {rh.CLASS, rc.CLASS})
_n5, _b5, _c5 = rh.reconcile(_mixed_entries, [_exh], 2)
check("closing one class leaves the other untouched",
      _b5 and _b5[0]["class"] == rh.CLASS and len(_mixed_entries) == 2)

# ── dispatch framing: a residual must NOT be described as a self-reported exhaustion ──
check("c14 framing names the concern class and the disposition set",
      "C14" in rc.framing_for(rc.CLASS) and "miss-of-canon" in rc.framing_for(rc.CLASS))
check("c14 framing carries the Mishpat clause (restored capability, never punishment)",
      "Mishpat" in rc.framing_for(rc.CLASS) and "never punishment" in rc.framing_for(rc.CLASS))
check("c14 framing does NOT claim the node self-reported exhaustion",
      "self-REPORTED exhaustion" not in rc.framing_for(rc.CLASS))
check("an unknown class falls back to the exhaustion framing",
      rc.framing_for("something-else") == rc.DEFAULT_FRAMING)


# ══════════════════════════════════════════════════════════════════════════════════════════
# LIVE PIPELINE: a node serving /admin/residuals -> ledger entry -> runtime-triage dispatch.
# ══════════════════════════════════════════════════════════════════════════════════════════
check("runtime-harvest.py shell exists", _root is not None)
_script = str(_root / ".claude" / "scripts" / "runtime-harvest.py")

_residual_body = json.dumps([CAPSULE])


class _ResidualNode(http.server.BaseHTTPRequestHandler):
    """A node that is perfectly healthy on every exhaustion meter and is witnessing a residual."""

    def do_GET(self):
        if self.path == "/admin/residuals":
            body = _residual_body
        elif self.path == "/admin/render-stats":
            body = json.dumps({"total": 100, "stalled": 0, "timedOut": 0, "degenerateRate": 0.0})
        else:
            body = "{}"
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(body.encode())

    def log_message(self, *a):
        pass


_srv = http.server.HTTPServer(("127.0.0.1", 0), _ResidualNode)
threading.Thread(target=_srv.serve_forever, daemon=True).start()
_port = _srv.server_address[1]

_tmp = tempfile.mkdtemp()
_env = dict(os.environ, CLAUDE_PROJECT_DIR=_tmp)
os.makedirs(os.path.join(_tmp, ".claude", "data"), exist_ok=True)
_hook = subprocess.run(["python3", _script, "--hook", "--nodes", "t",
                        "--base", f"http://127.0.0.1:{_port}"],
                       env=_env, capture_output=True, text=True, timeout=60)
check("residual poll exits 0", _hook.returncode == 0)
_ledger_path = os.path.join(_tmp, ".claude", "data", "runtime-findings.jsonl")
_ledger = [json.loads(x) for x in open(_ledger_path)] if os.path.exists(_ledger_path) else []
check("a served capsule lands on the LIVE ledger under the c14 namespace",
      any(e["class"] == rc.CLASS and e["provenance"] == GOLDEN_PROVENANCE for e in _ledger))
check("the ledger entry carries the full finding contract (fp/ts/status/seen/polls)",
      all(k in _ledger[0] for k in ("fp", "ts", "status", "seen", "first_poll", "last_poll")))
check("hook mode dispatches runtime-triage for the residual",
      "runtime-triage" in _hook.stdout)
check("the dispatch names the c14 class, NOT self-heal-exhaustion",
      rc.CLASS in _hook.stdout and "NEW self-heal-exhaustion" not in _hook.stdout)
check("the dispatch carries the C14/Mishpat framing", "Mishpat" in _hook.stdout)

# healthy exhaustion meters must not have produced an exhaustion finding — the residual is the
# ONLY thing this node reported, which is exactly the case C14 exists for.
check("a node healthy on every exhaustion meter still surfaces its residual",
      not any(e["class"] == rh.CLASS for e in _ledger))

# idempotence through the live shell
_before = len(_ledger)
subprocess.run(["python3", _script, "--nodes", "t", "--base", f"http://127.0.0.1:{_port}"],
               env=_env, capture_output=True, text=True, timeout=60)
_after = [json.loads(x) for x in open(_ledger_path)]
check("re-polling the same residual never double-files", len(_after) == _before)
check("re-polling bumps seen instead", _after[0]["seen"] >= 2)

# the ring buffer must not accumulate capsules (they are events, not window state)
_cursor = json.load(open(os.path.join(_tmp, ".claude", "data", "runtime-cursor.json")))
check("capsules never accumulate in the sample ring buffer",
      all("residuals" not in s for s in _cursor["windows"]["t"]))
_srv.shutdown()


# ══════════════════════════════════════════════════════════════════════════════════════════
# REPORT surface + the calibration handoff.
# ══════════════════════════════════════════════════════════════════════════════════════════
_panel = rc.render_residual_panel(_tmp)
check("panel renders the live residual", GOLDEN_PROVENANCE in _panel)
check("panel names the concern class", "C14" in _panel)
check("panel reports exception-metabolism as UNREACHABLE while no calibration ledger exists",
      "UNREACHABLE" in _panel and "0%" not in _panel)
check("panel carries the Mishpat disposition line",
      "Mishpat" in _panel and "never punishment" in _panel)

_thin = tempfile.mkdtemp()
os.makedirs(os.path.join(_thin, ".claude", "data"), exist_ok=True)
_thin_ledger = os.path.join(_thin, ".claude", "data", "runtime-findings.jsonl")
with open(_thin_ledger, "w") as fh:
    fh.write(json.dumps({"fp": "deadbeefcafe", "class": rc.CLASS, "node": "alpha",
                         "provenance": "residual:seam:kind",
                         "line": "seam residual [kind] raw=x | inputs=d | ctx: none",
                         "status": "open", "seen": 1, "first_poll": 1, "last_poll": 1}) + "\n")
check("panel flags a THIN capsule (no clock/lag/peer_state)",
      "thin capsule" in rc.render_residual_panel(_thin))
check("panel is empty when there is nothing to say (call site needs no guard)",
      rc.render_residual_panel(tempfile.mkdtemp()) == "")

# ── exception-metabolism: UNREACHABLE, never 0 (C4 turned on our own instrument) ──
_m = rc.exception_metabolism(_thin)
check("metabolism is UNREACHABLE with no calibration ledger", _m["status"] == "unreachable")
check("metabolism names the ledger path it looked for",
      rc.CALIBRATION_LEDGER_REL in _m["reason"])
check("the ledger path is the forecast leg's REAL artifact, not a guessed sibling",
      rc.CALIBRATION_LEDGER_REL == ".claude/data/seam-calibration.jsonl")

# The disposition writer refuses rows that would inflate the numerator with nothing behind them.
check("record refuses an unrecognized disposition",
      rc.record_residual_disposition(_thin, "deadbeefcafe", "punish") is False)
check("record refuses an empty residual fingerprint",
      rc.record_residual_disposition(_thin, "", "cure") is False)

# One live residual (deadbeefcafe, from the thin fixture ledger above), zero dispositioned.
_m1 = rc.exception_metabolism(_thin) if rc.find_calibration_ledger(_thin) else None
check("record appends a valid disposition to the real ledger",
      rc.record_residual_disposition(_thin, "deadbeefcafe", "cure", seam="s", note="fixed") is True)
_m2 = rc.exception_metabolism(_thin)
check("metabolism lights up the moment the calibration ledger appears",
      _m2["status"] == "present")
check("a dispositioned residual counts in BOTH terms", _m2["dispositioned"] == 1
      and _m2["novel"] == 1 and abs(_m2["rate"] - 1.0) < 1e-9)

# A second, still-open residual widens the denominator only.
with open(_thin_ledger, "a") as fh:
    fh.write(json.dumps({"fp": "0000outstanding", "class": rc.CLASS, "node": "alpha",
                         "provenance": "residual:seam:other",
                         "line": "seam residual [other] raw=y | inputs=d | ctx: clock=1",
                         "status": "open", "seen": 1, "first_poll": 1, "last_poll": 2}) + "\n")
_m3 = rc.exception_metabolism(_thin)
check("a LIVE undispositioned residual widens the denominator",
      _m3["novel"] == 2 and _m3["dispositioned"] == 1 and _m3["outstanding"] == 1
      and abs(_m3["rate"] - 0.5) < 1e-9)

# The forecast leg's own miss-of-canon rows join on residual_fp and never double-count.
_cal = Path(_thin) / rc.CALIBRATION_LEDGER_REL
with _cal.open("a") as fh:
    fh.write(json.dumps({"ts": "t", "kind": "calibration", "outcome": "miss-of-canon",
                         "residual_fp": "0000outstanding", "note": "argued"}) + "\n")
    fh.write(json.dumps({"ts": "t", "kind": "calibration", "outcome": "miss-of-canon",
                         "fp": "abc", "note": "not traced to a residual"}) + "\n")
    fh.write(json.dumps({"ts": "t", "kind": "calibration", "outcome": "hit", "fp": "x"}) + "\n")
_m4 = rc.exception_metabolism(_thin)
check("a miss-of-canon row naming a residual dispositions it as new-concern-class",
      _m4["dispositioned"] == 2 and _m4["by_disposition"].get("new-concern-class") == 1)
check("forecast-only rows (hit / untraced miss-of-canon) never enter the residual measure",
      _m4["novel"] == 2 and _m4["canon_growth_signals"] == 2)
check("a duplicate disposition for one fp is counted once (set semantics)",
      rc.record_residual_disposition(_thin, "deadbeefcafe", "declared-variance") is True
      and rc.exception_metabolism(_thin)["dispositioned"] == 2)
check("panel renders the rate once the ledger exists",
      "exception-metabolism rate: 100%" in rc.render_residual_panel(_thin))

# The additive row kind is INERT to the forecast leg's own summary.
try:
    from _lib import seam_forecast as _sf  # noqa: E402
    _sum = _sf.ledger_summary(_thin)
    check("residual-disposition rows are inert to the forecast leg's hit/miss counts",
          _sum["counts"]["hit"] == 1 and _sum["counts"]["miss-of-canon"] == 2
          and _sum["counts"]["miss-of-ranking"] == 0)
except ImportError:
    check("forecast leg not present in this checkout — composition check UNEVALUATED (C5)", True)

print(f"\n  {_p} assertions passed ✅")
