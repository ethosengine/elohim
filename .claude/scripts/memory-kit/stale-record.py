#!/usr/bin/env python3
"""The memory-ceremony's Phase 0 intake: in-flight STALE corrections, grouped by surface.

Reads ONLY the existing run-note ledger (.eprfs/status/flows.jsonl — REA events with
quantity.unit == "run-note"); never writes. A correction is a note whose classifiedAs[0] is
"run:correction"; its target is classifiedAs[1]; the reason is the remaining string slot (prefixed "reason:").
Only reasons beginning with "STALE" are the record (other corrections are code corrections).

  python3 .claude/scripts/memory-kit/stale-record.py [--since 2026-09-01] [--json]

--since defaults to the newest memory-ceremony chronicle date under .claude/memory-kit/ (or the
epoch if none), so a correction the last ceremony already absorbed is not carried twice.
"""
import argparse, glob, json, os, re, sys

ROOT = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(ROOT, "..", "..", ".."))
LEDGER = os.path.join(REPO, ".eprfs", "status", "flows.jsonl")


def newest_chronicle_date():
    dirs = sorted(d for d in glob.glob(os.path.join(REPO, ".claude", "memory-kit", "20*")) if os.path.isdir(d))
    for d in reversed(dirs):
        if glob.glob(os.path.join(d, "*ceremony*")) or glob.glob(os.path.join(d, "*chronicle*")):
            return os.path.basename(d)[:10]
    return "1970-01-01"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--since", default=None, help="ISO date; default = newest ceremony chronicle")
    ap.add_argument("--json", action="store_true")
    a = ap.parse_args()
    since = a.since or newest_chronicle_date()
    if not os.path.exists(LEDGER):
        print(f"no ledger at {LEDGER}", file=sys.stderr); print("{}" if a.json else "no record"); return 0
    by_surface = {}
    with open(LEDGER) as fh:
        for line in fh:
            line = line.strip()
            if not line or '"run-note"' not in line or "run:correction" not in line:
                continue
            try:
                rec = json.loads(line).get("record", {})
            except json.JSONDecodeError:
                continue
            slots = [x for x in rec.get("classifiedAs", []) if isinstance(x, str)]
            if len(slots) < 2 or slots[0] != "run:correction":
                continue
            target = slots[1]
            reason = None
            for x in slots[2:]:
                x = x[len("reason:"):] if x.startswith("reason:") else x
                if x.startswith("STALE"):
                    reason = x
                    break
            if reason is None:
                continue
            when = (rec.get("occurredAt") or "")[:10]
            if when < since:
                continue
            by_surface.setdefault(target, []).append({"date": when, "reason": reason})
    if a.json:
        print(json.dumps({"since": since, "surfaces": by_surface}, indent=1)); return 0
    if not by_surface:
        print(f"no STALE corrections since {since} — the record is empty; fall through to the Phase 1 audit"); return 0
    print(f"STALE corrections since {since} — {len(by_surface)} surface(s), pre-triaged for the ceremony:\n")
    for surface, notes in sorted(by_surface.items(), key=lambda kv: (-len(kv[1]), kv[0])):
        print(f"{surface}  ({len(notes)})")
        for n in sorted(notes, key=lambda n: n["date"]):
            print(f"  - {n['date']} · {n['reason'][:300]}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
