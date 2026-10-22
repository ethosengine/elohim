"""Probe for `limit-models@1` (emits: candidates).

Procedure (measures.yaml): every tracked development-system source file outside `elohim/epr-rea`
that judges a level against a limit with its own comparison — a `soft`/`hard`/`ceiling`/
`watermark` comparison, a comparison against a bound's `.limit`/`band_edge`, or a locally
declared `Bound`/`Compare` type — instead of asking `elohim_epr_rea::Bound` (`breached_by`,
`approached_by`) or the native report. One candidate per file, the lines as symbols.

Detection only. A declaration READER that projects YAML onto a shape (rather than judging a
level) and a consumer that only renders a verdict the native report produced are the restraint
cases a ruling classifies.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import _probe  # noqa: E402  (imported by path: probes run under python3 -I)

WORDS = r"(?:hard|soft|ceiling|watermark|loc_hard|loc_soft|band_edge\(\)|\.limit)\b"
JUDGE = re.compile(
    rf"""(?:>=|<=|(?<![-=])>(?!=)|(?<![-<])<(?!=|-))\s*[\w.\[\]'"()]*{WORDS}"""
    rf"""|{WORDS}[\w.\[\]'"()]*\s*(?:>=|<=|(?<![-=])>(?!=)|(?<![-<])<(?!=|-))"""
    r"""|\b(?:pub\s+)?(?:struct|enum)\s+(?:Bound|Compare)\b"""
)
SELF = "probes/limit_models.py"


def main() -> None:
    base = _probe.root()
    candidates = []
    for rel in _probe.tracked(base):
        if rel.endswith(SELF) or rel.startswith("elohim/epr-rea/"):
            continue
        lines = _probe.matching_lines(_probe.read(base, rel), JUDGE)
        if lines:
            candidates.append(
                {
                    "subject": rel,
                    "symbol": ",".join(f"L{n}" for n in lines[:4]),
                    "signature": "level-judged-outside-epr-rea",
                }
            )
    _probe.emit({"candidates": candidates})


if __name__ == "__main__":
    main()
