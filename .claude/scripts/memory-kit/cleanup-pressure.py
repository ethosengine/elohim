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
"""
import sys, os, json

# Calibrated to ~a week of heavy development (distinct drifted items since the last cleanup).
# Tunable; raise to fire less often, lower to fire more. A heavy week churns ~100-150 distinct docs/entries.
THRESHOLD = 120
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


def status(root, as_json=False):
    """activity drift accumulated since the last cleanup — the heavy-week gate. Pure stdlib (no --headline
    shell-out, so it never recurses through the audit and stays callable anywhere). The current-STATE drift
    (a dump forming, the index going stale) is what the loop itself measures + drains once it fires; this gate
    only answers WHEN enough has changed to be worth firing."""
    a = activity(_kit(root))
    due = a >= THRESHOLD
    if as_json:
        return {"pressure": a, "threshold": THRESHOLD, "activity": a, "due": due}
    return (f"cleanup: pressure {a}/{THRESHOLD}  (drift items since last cleanup)"
            f"   ({'⚠ cleanup due → /memory-stasis-loop' if due else 'held ✅'})")


def reset(root):
    kit = _kit(root)
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
    print("cleanup-pressure: activity accumulators reset (next cycle starts fresh)")


def main():
    root = _root()
    if "--reset" in sys.argv:
        reset(root)
    else:
        print(json.dumps(status(root, True)) if "--json" in sys.argv else status(root))


if __name__ == "__main__":
    main()
