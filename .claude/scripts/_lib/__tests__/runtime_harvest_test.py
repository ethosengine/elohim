#!/usr/bin/env python3
"""Tests for runtime_harvest (the elevate-arm pure core: predicates + ledger
reconcile). Run: python3 .claude/scripts/_lib/__tests__/runtime_harvest_test.py
(exit 0 = pass). pytest is NOT installed — this is the bespoke harness."""
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

print(f"\n  {_p} assertions passed ✅")
