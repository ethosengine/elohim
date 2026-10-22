"""Print 1 when SUBJECT is an open candidate in an `epr flow sense --json` run on stdin, else 0.

The objective measure of a sense-serving shift: one number on stdout, as the shift objective
schema requires. A probe refusal prints nothing and exits 1 — an unmeasured candidate is never 0.

    epr flow sense --measure limit-models@1 --json | python3 -I open_subject.py <subject>
"""

from __future__ import annotations

import json
import sys


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit("usage: open_subject.py <subject>")
    subject = sys.argv[1]
    outcomes = json.load(sys.stdin)
    if not outcomes or any(o.get("status") == "refused" for o in outcomes):
        sys.exit("open_subject: the probe was refused; no measurement")
    print(1 if any(subject in o.get("open", {}) for o in outcomes) else 0)


if __name__ == "__main__":
    main()
