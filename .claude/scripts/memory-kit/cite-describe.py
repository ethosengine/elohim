#!/usr/bin/env python3
"""cite-describe.py <doc> '<json {ref: desc}>' — set authored one-sentence descriptions on a doc's envelope
cites, preserving ref / fingerprint / status. The write-back for the description-authoring cleanup (the
epr-head hint that powers progressive discovery). Idempotent; only touches cites whose ref is in the map.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

_here = Path(__file__).resolve()
for _ in range(8):
    if (_here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(_here / ".claude" / "scripts"))
        break
    _here = _here.parent
from _lib import cite_graph as cg  # noqa: E402
from _lib import frontmatter as fm  # noqa: E402

ROOT = next(p for p in Path(__file__).resolve().parents if (p / ".git").exists())


def main() -> int:
    if len(sys.argv) < 3:
        print("usage: cite-describe.py <doc> '<json {ref: desc}>'", file=sys.stderr)
        return 2
    doc = (ROOT / sys.argv[1]) if not Path(sys.argv[1]).is_absolute() else Path(sys.argv[1])
    try:
        descs = json.loads(sys.argv[2])
    except json.JSONDecodeError as e:
        print(f"cite-describe: bad json: {e}", file=sys.stderr)
        return 2
    text = doc.read_text(encoding="utf-8", errors="replace")
    sp = cg.split_frontmatter(text)
    if sp is None:
        print(f"cite-describe: no frontmatter: {doc}", file=sys.stderr)
        return 2
    fm_lines, body = sp
    cites = cg.parse_cites(fm.parse(text))
    n = 0
    for c in cites:
        if not c.get("legacy_path") and c["ref"] in descs and descs[c["ref"]].strip():
            new = descs[c["ref"]].strip().replace("|", "/")  # | is the envelope delimiter — never in a desc
            if c.get("desc") != new:
                c["desc"] = new
                n += 1
    if n:
        out, ok = cg.set_cites_block(fm_lines, cg.serialize_cites(cites))
        if ok:
            doc.write_text(cg.reassemble(out, body), encoding="utf-8")
    print(f"cite-describe {sys.argv[1]}: {n} description(s) set")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
