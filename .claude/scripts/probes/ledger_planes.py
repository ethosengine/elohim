"""Probe for `ledger-planes@1` (emits: count).

Procedure (measures.yaml): count the files among `.claude/data/*.jsonl` and
`$CLAUDE_CONFIG_DIR/berth/ledger.jsonl` whose basename a script under `.claude/scripts` other than
its own appender parses.

A script APPENDS to a ledger when it names the basename and opens or writes in append mode; it
PARSES one when it names the basename and decodes JSON. A ledger counts when at least one parser
under `.claude/scripts` is not also an appender of it — a second reader with its own cursor, which
is what `ledgers-fold-once` forbids.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import _probe  # noqa: E402  (imported by path: probes run under python3 -I)

APPEND = re.compile(
    r"""open\([^)]*['"]a['"]|['"]a['"]\s*\)|append_if_new|appendFileSync|>>\s*["$]|\.append_line\("""
)
PARSE = re.compile(r"json\.loads|json\.load\(|JSON\.parse|serde_json::from_str")
SCRIPT_ROOTS = (".claude/scripts", ".claude/hooks", "genesis/agentic", "genesis/orchestrator")


def ledgers(base: Path) -> list[str]:
    names = sorted(p.name for p in (base / ".claude/data").glob("*.jsonl"))
    return names + ["ledger.jsonl"]  # the berth ledger lives under $CLAUDE_CONFIG_DIR


def main() -> None:
    base = _probe.root()
    scripts = _probe.tracked(base, SCRIPT_ROOTS)
    texts = {rel: _probe.read(base, rel) for rel in scripts}
    counted = 0
    detail = []
    for name in ledgers(base):
        token = re.compile(re.escape(name))
        naming = [rel for rel, text in texts.items() if token.search(text)]
        appenders = {rel for rel in naming if APPEND.search(texts[rel])}
        parsers = [
            rel
            for rel in naming
            if rel.startswith(".claude/scripts/")
            and PARSE.search(texts[rel])
            and rel not in appenders
        ]
        if parsers:
            counted += 1
            detail.append(f"{name}: read by {', '.join(parsers[:3])}" + (" …" if len(parsers) > 3 else ""))
    _probe.emit({"value": counted, "detail": detail})


if __name__ == "__main__":
    main()
