#!/usr/bin/env python3
"""cleanup-pressure — the drift-elevation gate that decides WHEN the memory-cleanup loop should fire.

The PostToolUse hooks accumulate raw activity (placement-drift, map-currency-drift, claude-md-drift,
memory-coherence-drift) since the last cleanup; the --headline aggregates current-state debt. This rolls both
into ONE pressure score with a tunable threshold calibrated to roughly a WEEK OF HEAVY DEVELOPMENT, so a
weekly poll fires the loop only when enough has actually changed — not on a quiet week, not many times a day.

  pressure = activity_drift (items touched since last cleanup)  +  STATE_WEIGHT * unresolved_disciplines

A scheduled routine polls `--status`; on `due` it runs /memory-stasis-loop, then calls `--reset` to zero the
activity accumulators so the next week starts fresh. The check is pure-stdlib (no agent/LLM/CLI needed).

  --status [--json]   one line: pressure N / threshold M  (held ✅ | ⚠ cleanup due)
  --reset             zero the activity accumulators (call after a cleanup loop completes)

## The undenominated-threshold defect (found 2026-08-11, systems-discipline slice 2)

`THRESHOLD` is documented right below as *"calibrated to ~a week of heavy development."* That makes it a
**per-week** quantity. `pressure` was a bare cumulative count since an UNRECORDED moment — `reset()` zeroed the
accumulators without stamping when — so `210/120` meant 210 items since some unknown point, and 210 items in five
days read identically to 210 in five weeks. Only one of those says the system is losing.

That is the same shape as the `spatial_capacity.rs` defect this arc started from (a level compared against a
rate-calibrated ceiling), minus the monotonicity that made that one fatal — the reset saves it from reading
permanently overshot. `reset()` now stamps `reset_at` and records the cycle, which makes two things computable
that were not:

  * **the problem growth rate** — drifted items per week, the denominated form of the threshold's own calibration
  * **the response rate** — items drained per week by the last completed cleanup cycle

and therefore their ratio, which is Meadows' controllability index applied to this ceremony itself: *is drift
arriving faster than the loop can absorb it?* If that ratio exceeds 1.0, firing the loop more often cannot close
the gap — only raising the response rate structurally, or lowering the generation rate, can.

**The `due` predicate deliberately still fires on the LEVEL.** Migrating it to the rate would change when the
ceremony fires (a 3-week-old 210 is under threshold as a rate and over it as a level), and changing a governance
trigger is an operator decision, not an implementer's. The rate is reported alongside so the decision has
evidence; `rate_due` is computed and surfaced but gates nothing.
"""
import sys, os, json
from datetime import datetime, timezone

# Calibrated to ~a week of heavy development (distinct drifted items since the last cleanup).
# Tunable; raise to fire less often, lower to fire more. A heavy week churns ~100-150 distinct docs/entries.
# NOTE the denominator hiding in that sentence: this is a PER-WEEK figure. See the module docstring.
THRESHOLD = 120
THRESHOLD_PER = "week"

# Where the cycle history lives — the one thing that makes a rate computable at all.
CYCLE_FILE = "cleanup-cycles.json"
MAX_CYCLES = 24  # ~6 months of weekly cycles; bounded so the file never becomes its own dump
ACTIVITY_FILES = ["placement-drift.json", "map-currency-drift.json",
                  "claude-md-drift.json", "memory-coherence-drift.json",
                  "memory-index-drift.json"]  # projector budget/violation snapshot (memory-index-projector.py)
COLLECTIONS = ("files", "entries", "due", "items")  # the per-accumulator collection of drifted things


def _root():
    p = os.path.abspath(__file__)
    while p != "/":
        if os.path.isdir(os.path.join(p, ".claude", "memory-kit")):
            return p
        p = os.path.dirname(p)
    return os.getcwd()


def _kit(root):
    return os.path.join(root, ".claude", "memory-kit")


def activity(kit):
    """count of distinct drifted items across the accumulators since the last cleanup/reset."""
    total = 0
    for name in ACTIVITY_FILES:
        f = os.path.join(kit, name)
        if not os.path.exists(f):
            continue
        try:
            d = json.load(open(f))
        except (OSError, ValueError):
            continue
        for key in COLLECTIONS:
            v = d.get(key)
            if isinstance(v, dict):
                total += len(v)
            elif isinstance(v, list):
                total += len(v)
    return total


def _cycles(kit):
    """Completed cleanup cycles, oldest first. Missing file is a legitimate state (no cycle has
    been stamped yet) and yields an empty history — which makes every rate below honestly
    unknown rather than fabricated from a guessed start date."""
    f = os.path.join(kit, CYCLE_FILE)
    try:
        return json.load(open(f)).get("cycles", [])
    except (OSError, ValueError):
        return []


def _weeks_since(iso):
    """Weeks elapsed since an ISO timestamp; None when unparseable or absent."""
    if not iso:
        return None
    try:
        then = datetime.fromisoformat(iso)
        if then.tzinfo is None:
            then = then.replace(tzinfo=timezone.utc)
        secs = (datetime.now(timezone.utc) - then).total_seconds()
        return max(secs / 604800.0, 1e-9)  # never zero — a zero denominator is not a short week
    except (ValueError, TypeError):
        return None


def dynamics(root):
    """The denominated form: growth rate, response rate, and their ratio — each a declared
    measure on the `elohim/epr/src/measure.rs` wire vocabulary.

    Everything here degrades to honest ABSENCE rather than to a number. No stamped cycle means
    the elapsed time is unknown, which means the rate is unknown — and an unknown rate must not
    quietly become a zero or an infinity, because both of those compare against a threshold and
    an absence must not."""
    # _lib bootstrap is LOCAL to this function on purpose: `activity()` and `status()`'s level
    # reading stay pure-stdlib and cheap (this module is polled by a scheduled routine and is
    # documented as never shelling out), so the import cost is paid only when someone actually
    # asks for the denominated form.
    _here = os.path.abspath(__file__)
    for _ in range(8):
        if os.path.isdir(os.path.join(_here, ".claude", "scripts", "_lib")):
            if os.path.join(_here, ".claude", "scripts") not in sys.path:
                sys.path.insert(0, os.path.join(_here, ".claude", "scripts"))
            break
        _here = os.path.dirname(_here)
    from _lib import signal_measure as sm

    kit = _kit(root)
    a = activity(kit)
    cycles = _cycles(kit)
    last = cycles[-1] if cycles else None
    weeks = _weeks_since(last.get("reset_at") if last else None)

    level = sm.measure(
        float(a), "level", claim="witnessed",
        basis=(f"{a} distinct drifted items counted across {len(ACTIVITY_FILES)} accumulators "
               f"since the last reset" + (f" ({last['reset_at']})" if last else
                                          " — NO reset has ever been stamped")))
    if weeks is None:
        growth = sm.measure(
            None, "rate", per="week", claim="modelled",
            basis=("elapsed time since the last cleanup is unrecorded, so a per-week rate is not "
                   "computable — this is the defect the reset stamp exists to close, reported as "
                   "absence rather than papered over with a guessed start date"))
    else:
        growth = sm.measure(
            a / weeks, "rate", per="week", claim="witnessed",
            basis=(f"{a} drifted items over {weeks:.2f} weeks since {last['reset_at']} — the "
                   f"denominated form of THRESHOLD's own stated per-week calibration"))

    # The RESPONSE rate: what the last completed cycle actually drained, per week it took.
    response = sm.measure(
        None, "rate", per="week", claim="modelled",
        basis="no completed cleanup cycle has been stamped yet — response rate unknown")
    if len(cycles) >= 2:
        prev, cur = cycles[-2], cycles[-1]
        span = _weeks_since(prev.get("reset_at"))
        cur_span = _weeks_since(cur.get("reset_at"))
        if span is not None and cur_span is not None and span > cur_span:
            cycle_weeks = span - cur_span
            drained = cur.get("pressure_at_reset", 0)
            response = sm.measure(
                drained / cycle_weeks, "rate", per="week", claim="witnessed",
                basis=(f"{drained} items drained by the cleanup cycle that closed "
                       f"{cur['reset_at']}, over the {cycle_weeks:.2f} weeks it spanned"))

    controllability = sm.ratio_of_rates(
        growth, response,
        basis=("respite/response for the memory ceremony itself: drift arrival rate divided by "
               "cleanup absorption rate. Above 1.0, firing the loop more often cannot close the "
               "gap — only raising the response rate structurally or lowering the generation "
               "rate can (Meadows/Biesiot, Indicators pp.31-32)"))
    return {"level": level, "growth": growth, "response": response,
            "controllability": controllability, "cycles": len(cycles)}


def status(root, as_json=False):
    """activity drift accumulated since the last cleanup — the heavy-week gate. Pure stdlib (no --headline
    shell-out, so it never recurses through the audit and stays callable anywhere). The current-STATE drift
    (a dump forming, the index going stale) is what the loop itself measures + drains once it fires; this gate
    only answers WHEN enough has changed to be worth firing.

    `due` fires on the LEVEL, unchanged. `rate_due` is the denominated reading the threshold's own calibration
    implies; it is reported and gates nothing, because moving a governance trigger is the operator's call."""
    a = activity(_kit(root))
    due = a >= THRESHOLD
    try:
        dyn = dynamics(root)
    except Exception:  # noqa: BLE001 — a session-start surface never dies on its own instrument
        dyn = None
    rate = dyn["growth"]["value"] if dyn else None
    rate_due = None if rate is None else rate >= THRESHOLD

    if as_json:
        out = {"pressure": a, "threshold": THRESHOLD, "activity": a, "due": due,
               "threshold_per": THRESHOLD_PER, "rate_per_week": rate, "rate_due": rate_due}
        if dyn:
            out["measures"] = {k: v for k, v in dyn.items() if k != "cycles"}
            out["cycles"] = dyn["cycles"]
        return out

    line = (f"cleanup: pressure {a}/{THRESHOLD}  (drift items since last cleanup)"
            f"   ({'⚠ cleanup due → /memory-stasis-loop' if due else 'held ✅'})")
    if rate is None:
        line += f"  [rate/{THRESHOLD_PER}: unknown — no stamped reset]"
    else:
        line += (f"  [{rate:.0f}/{THRESHOLD_PER} vs a {THRESHOLD}/{THRESHOLD_PER} threshold"
                 f" → {'over' if rate_due else 'under'}]")
    if dyn and dyn["controllability"]["value"] is not None:
        line += f"  [respite/response {dyn['controllability']['value']:.2f}]"
    return line


def reset(root):
    kit = _kit(root)
    drained = activity(kit)  # what this cycle actually absorbed — captured BEFORE zeroing
    for name in ACTIVITY_FILES:
        f = os.path.join(kit, name)
        if not os.path.exists(f):
            continue
        try:
            d = json.load(open(f))
        except (OSError, ValueError):
            continue
        for key in COLLECTIONS:
            if isinstance(d.get(key), dict):
                d[key] = {}
            elif isinstance(d.get(key), list):
                d[key] = []
        json.dump(d, open(f, "w"), indent=2)

    # THE STAMP. Without it the count has no denominator and the threshold's own per-week
    # calibration is uncheckable. `pressure_at_reset` is what this cycle drained, which is the
    # numerator of the RESPONSE rate the next reading needs.
    stamp = {"reset_at": datetime.now(timezone.utc).isoformat(timespec="seconds"),
             "pressure_at_reset": drained}
    path = os.path.join(kit, CYCLE_FILE)
    try:
        state = json.load(open(path))
    except (OSError, ValueError):
        state = {"schema_version": 1, "cycles": []}
    state.setdefault("cycles", []).append(stamp)
    state["cycles"] = state["cycles"][-MAX_CYCLES:]
    json.dump(state, open(path, "w"), indent=2)
    print(f"cleanup-pressure: activity accumulators reset (drained {drained}; cycle stamped "
          f"{stamp['reset_at']} — the next reading can report a RATE, not just a count)")


def main():
    root = _root()
    if "--reset" in sys.argv:
        reset(root)
    elif "--dynamics" in sys.argv:
        print(json.dumps(dynamics(root), indent=2))
    else:
        print(json.dumps(status(root, True)) if "--json" in sys.argv else status(root))


if __name__ == "__main__":
    main()
