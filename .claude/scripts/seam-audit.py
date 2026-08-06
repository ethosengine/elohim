#!/usr/bin/env python3
"""seam-audit.py — the runnable surface for the seam-concern instruments (plan P4.2/P4.3/P4.4).

Thin shell over `_lib/seam_{cascade,matrix,forecast}.py`, exactly the split
`runtime-harvest.py` uses over `_lib/runtime_harvest.py`: the modules are pure logic and this file
owns the I/O and the mutations.

WHY A SEPARATE SCRIPT FROM placement-audit. `placement-audit.py --epr-meta` RENDERS the census,
cascade and matrix — it is a scoreboard and must stay read-only; an audit that mutates state as a
side effect of being read is how a scoreboard quietly becomes an actor. Every write in this
program is behind an explicit verb (`--file`, `--write`, `--record`, `--record-miss-of-canon`).

  --cascade [--file]          pin-keyed cascade; --file appends new findings to
                              .claude/data/cascade-findings.jsonl and prints dispatch directives
  --matrix [--write]          the concern x seam matrix; --write regenerates
                              .claude/epr-meta/generated/concern-seam-matrix.{json,md}
  --forecast                  the seeded forecast, seeded vs derived rank, ledger summary
  --calibrate [--record]      habits-flip intake for the calibration ledger (append-only)
  --record-miss-of-canon --note "<evidence>" [--seam S3.x] [--rung <node>]
                              the canon-growth intake; refuses an empty note
  --spot-check [N]            INDEPENDENT re-derivation of N sampled decision points from disk;
                              exit 2 on any mismatch (the ranking is not trustworthy until
                              explained — a mis-resolving audit reorders, it does not degrade)
  --json                      machine-readable output where the mode supports it

Exit codes: 0 ok · 1 usage/gate error · 2 spot-check mismatch.
"""
import argparse
import json
import sys
from pathlib import Path

_here = Path(__file__).resolve()
_ROOT = _here.parent.parent
for _ in range(8):
    if (_here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(_here / ".claude" / "scripts"))
        _ROOT = _here
        break
    _here = _here.parent

from _lib import seam_cascade as casc  # noqa: E402
from _lib import seam_forecast as fc  # noqa: E402
from _lib import seam_matrix as mx  # noqa: E402


def main() -> int:
    ap = argparse.ArgumentParser(add_help=True, description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--cascade", action="store_true")
    ap.add_argument("--matrix", action="store_true")
    ap.add_argument("--forecast", action="store_true")
    ap.add_argument("--calibrate", action="store_true")
    ap.add_argument("--record-miss-of-canon", action="store_true")
    ap.add_argument("--spot-check", nargs="?", const=8, type=int, default=None)
    ap.add_argument("--file", action="store_true", help="cascade: append findings to the ledger")
    ap.add_argument("--write", action="store_true", help="matrix: regenerate the artifacts")
    ap.add_argument("--record", action="store_true", help="calibrate: append to the ledger")
    ap.add_argument("--note", default="", help="miss-of-canon evidence prose (required)")
    ap.add_argument("--seam", default="")
    ap.add_argument("--rung", default="none")
    ap.add_argument("--seed", default="", help="spot-check: move the sample window")
    ap.add_argument("--json", action="store_true")
    args = ap.parse_args()

    root = _ROOT
    did = False
    rc = 0

    if args.matrix:
        did = True
        data = mx.matrix_data(root)
        print(mx.render_matrix(data, as_json=args.json))
        if args.write:
            written = mx.write_generated(root, data)
            print("\nwrote: " + ", ".join(written))
        else:
            state, detail = mx.generated_freshness(root, data)
            if state != "fresh":
                print(f"\ngenerated artifact: ⚠ {state} ({detail}) — regenerate with "
                      f"`seam-audit.py --matrix --write`")

    if args.cascade:
        did = True
        matrix = mx.matrix_data(root)
        data = casc.cascade_data(root, matrix=matrix)
        print(casc.render_cascade(data, as_json=args.json))
        if args.file:
            res = casc.file_findings(root, data)
            print(f"\nfiled: {len(res['new'])} new · {len(res['already'])} already live · "
                  f"{len(res['contended'])} lock-contended · {res['drained']} drained")
            for d in res["dispatches"]:
                print("\n" + d)

    if args.forecast:
        did = True
        print(fc.render_forecast(root, as_json=args.json))

    if args.calibrate:
        did = True
        cal = fc.calibrate(root)
        if args.json:
            print(json.dumps(cal, indent=1))
        else:
            print("CALIBRATION  (plan P4.4 — append-only ledger, habits-flip intake)")
            print("=" * 72)
            print(f"forecast rows: {cal['forecast_rows']} · snapshot on file: "
                  f"{cal['has_snapshot']} · flips: {len(cal['flips'])}")
            if cal["baseline_only"]:
                print("  first run — no snapshot on file. `--record` establishes the BASELINE and "
                      "files no\n  calibration rows (a baseline is not a set of events).")
            for f in cal["flips"]:
                if f["kind"] != "first-run":
                    print(f"  ⚑ {f['node']}: {f['from']} -> {f['to']} ({f['kind']})")
            for p in cal["proposals"]:
                print(f"    -> {p['outcome']}  fp={p['fp'] or '—'}  {p['trigger']}")
                if p.get("note"):
                    print(f"       {p['note']}")
            for e in cal["errors"]:
                print(f"  ⛔ {e}")
            if not cal["proposals"] and not cal["baseline_only"]:
                print("  no flips since the last snapshot — nothing to record ✅")
        if args.record:
            res = fc.record(root, cal)
            print(f"\nrecorded: {res['written']} calibration row(s)"
                  + (" + a fresh habits snapshot" if res["snapshot"] else ""))

    if args.record_miss_of_canon:
        did = True
        ok = fc.record_miss_of_canon(root, note=args.note, seam=args.seam, rung=args.rung)
        if not ok:
            print("miss-of-canon REFUSED: --note is required. A canon-growth claim without "
                  "evidence prose is exactly the content-free growth signal the admission bar "
                  "(>=2 dated instances at distinct seams) exists to prevent.", file=sys.stderr)
            return 1
        print(f"recorded miss-of-canon in {fc.CALIBRATION_LEDGER_REL}")

    if args.spot_check is not None:
        did = True
        res = mx.spot_check(root, args.spot_check, seed=args.seed)
        if args.json:
            print(json.dumps(res, indent=1))
        else:
            print("SPOT-CHECK  (independent re-derivation from disk — verify the measure BEFORE "
                  "the ranking)")
            print("=" * 72)
            print(f"sampled {len(res['checked'])} of {res['n_universe']} decision points")
            for c in res["checked"]:
                print(f"  · {c['registry']}  {c['crate']}::{c['point']} -> {c['seam']}  "
                      f"concerns={','.join(c['disk_concerns'])}  verified={c['disk_verified']}")
            print(f"\nverdict: {res['verdict']}")
            for m in res["mismatches"]:
                print(f"  ⛔ {m}")
        if res["verdict"] == "DO-NOT-TRUST":
            rc = 2

    if not did:
        ap.print_help()
        return 1
    return rc


if __name__ == "__main__":
    sys.exit(main())
