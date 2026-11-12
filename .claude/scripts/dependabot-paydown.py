#!/usr/bin/env python3
"""Check committed security paydown offline; SECURITY_PAYDOWN=warn|strict|off.

Retire this thin sketch when native epr flow sense owns the poll,
reconciliation and paydown judgement. This never writes or harvests a ledger.
"""
import argparse
import os
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
from _lib.dependabot_paydown import check, read_ledger


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", required=True)
    parser.add_argument("--head", required=True)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--ledger", type=Path, help="offline fixture ledger (default: root .claude/data)")
    args = parser.parse_args(argv)
    mode = os.environ.get("SECURITY_PAYDOWN", "warn")
    if mode == "off":
        return 0
    if mode not in {"warn", "strict"}:
        print("security-paydown: REFUSED invalid SECURITY_PAYDOWN; use warn|strict|off")
        return 1
    ledger = args.ledger or args.root / ".claude/data/dependabot-findings.jsonl"
    if not ledger.exists():
        print("security-paydown: unmeasured (no ledger)")
        return 0
    try:
        results = check(args.root, args.base, args.head, read_ledger(ledger))
    except Exception as exc:
        print(f"security-paydown: REFUSED unmeasured ({' '.join(str(exc).split())[:240]}) → restore ledger/range/checker inputs")
        return int(mode == "strict")
    for _, line in results:
        print(line)
    return int(mode == "strict" and any(not ok for ok, _ in results))


if __name__ == "__main__":
    sys.exit(main())
