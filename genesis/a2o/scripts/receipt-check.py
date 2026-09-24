#!/usr/bin/env python3
"""Assert a household sprint-report receipt for one or more @concern prefixes.

A receipt passes when every named concern has passed > 0, failed == 0, pending == 0,
AND the report's gitCommit is an ancestor of dev (the measured bytes reached the
integration target). Exit 0 on pass, 1 otherwise. Used by `verified_by:` receipts in
genesis/docs/superpowers plans so a claim can be re-checked from the repo, not a scratchpad.

    python3 genesis/a2o/scripts/receipt-check.py <sprint-report.json> <concern-prefix>...
"""
import json
import subprocess
import sys


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        print(__doc__)
        return 2
    path, *concerns = argv
    report = json.load(open(path))
    by_concern = report["summary"]["byConcern"]
    ok = True
    for concern in concerns:
        names = [k for k in by_concern if k.startswith(concern)]
        if not names:
            print("MISSING", concern)
            ok = False
        for name in names:
            v = by_concern[name]
            good = v.get("passed", 0) > 0 and v.get("failed", 0) == 0 and v.get("pending", 0) == 0
            print(("OK  " if good else "BAD ") + name, v.get("passed"), v.get("failed"), v.get("pending"))
            ok &= good
    commit = report["gitCommit"]
    in_dev = subprocess.run(["git", "merge-base", "--is-ancestor", commit, "dev"]).returncode == 0
    print("gitCommit", commit[:9], "in-dev" if in_dev else "NOT-in-dev", "generatedAt", report.get("generatedAt"))
    ok &= in_dev
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
