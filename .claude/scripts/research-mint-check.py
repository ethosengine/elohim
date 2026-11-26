#!/usr/bin/env python3
"""Check that every take a research note DECLARES has a home.

A note under genesis/research/ (top-level *.md with frontmatter, README.md
skipped) may declare `takes:` as a list of strings, `<take id> | <home>`:

  <cluster-slug>#<row>[,<row>...]  backlog row exists and names the note's id
  file: <repo-relative path>       the path exists
  released: <reason>               the reason is not empty
  owed: <what is missing>          a take the note still means and that has no home
                                   yet; always counted as failing, so it stays visible

Only declared takes are checked; prose is never read for takes. A note with no
`takes:` key, or with no frontmatter at all, is undeclared and counted as
unmeasured, never pass or fail (a no-frontmatter note is also listed by name).
`takes: []` declares that the note has no takes, and passes. A bare `takes:` with
no entries declares nothing: it is counted as unmeasured and listed by name, so a
half-written or emptied key does not read as a note with no takes.
Read-only. Exit 0 unless --strict (see --help).
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE / "_lib"))
import frontmatter  # noqa: E402  (shared stdlib parser; unwraps quoted list items)

DEFAULT_ROOT = HERE.parent.parent
RESEARCH = Path("genesis/research")
BACKLOG = Path("genesis/data/timeline/backlog")
_ROW_HOME = re.compile(r"^([A-Za-z0-9][A-Za-z0-9._-]*)#(\d+(?:\s*,\s*\d+)*)$")


def _rows_numbered(backlog_text: str, row: str) -> list[str]:
    """Every table line whose first cell is this number. More than one is ambiguous."""
    found = []
    for line in backlog_text.splitlines():
        s = line.strip()
        if not s.startswith("|"):
            continue
        cells = s.strip("|").split("|")
        if cells and cells[0].strip() == row:
            found.append(s)
    return found


def _names(line: str, note_id: str) -> bool:
    """The row names this note: its id as a whole slug, not as part of a longer one."""
    if not note_id:
        return False
    pattern = r"(?<![A-Za-z0-9_-])" + re.escape(note_id) + r"(?![A-Za-z0-9_-])"
    return re.search(pattern, line) is not None


def _shape_problem(raw_block: str) -> str | None:
    """A `takes:` entry must sit on one line; the shared parser drops what follows it."""
    lines = raw_block.splitlines()
    for i, line in enumerate(lines):
        if not re.match(r"^takes:\s*$", line):
            continue
        for follow in lines[i + 1:]:
            if not follow.strip():
                continue
            if not follow[:1].isspace():
                break  # the next top-level key
            if not follow.lstrip().startswith("- "):
                return "a takes: entry continues onto a second line; write each take on one line"
    return None


def check_take(root: Path, note_id: str, entry: str) -> tuple[str, str | None]:
    """Return (take_id, problem-or-None)."""
    if "|" not in entry:
        return "", "malformed: expected '<take id> | <home>'"
    take_id, home = (p.strip() for p in entry.split("|", 1))
    if not take_id or not home:
        return take_id, "malformed: empty take id or home"
    if home.startswith("owed:"):
        what = home[len("owed:"):].strip()
        return take_id, f"owed: {what}" if what else "owed with nothing said about what is missing"
    if home.startswith("released:"):
        if not home[len("released:"):].strip():
            return take_id, "released with no reason"
        return take_id, None
    if home.startswith("file:"):
        rel = home[len("file:"):].strip()
        if not rel:
            return take_id, "file: with no path"
        rel_path = Path(rel)
        if rel_path.is_absolute() or ".." in rel_path.parts:
            return take_id, f"file path must be repository-relative and stay inside it: {rel}"
        target = root / rel_path
        if not target.exists():
            return take_id, f"file not found: {rel}"
        if not target.resolve().is_relative_to(root.resolve()):
            return take_id, f"file path must be repository-relative and stay inside it: {rel} (a link that leaves the repository)"
        return take_id, None
    m = _ROW_HOME.match(home)
    if not m:
        return take_id, f"malformed home: {home!r} fits none of slug#row, file:, released:, owed:"
    slug, rows = m.group(1), [r.strip() for r in m.group(2).split(",")]
    bfile = root / BACKLOG / f"{slug}.md"
    if not bfile.is_file():
        return take_id, f"backlog cluster not found: {BACKLOG / (slug + '.md')}"
    text = bfile.read_text(encoding="utf-8", errors="replace")
    for row in rows:
        found = _rows_numbered(text, row)
        if not found:
            return take_id, f"row {row} not found in {slug}"
        if len(found) > 1:
            return take_id, f"row {row} is ambiguous in {slug}: {len(found)} table rows carry that number"
        if not _names(found[0], note_id):
            return take_id, f"row {row} of {slug} does not name note id {note_id!r}"
    return take_id, None


def scan(root: Path) -> dict:
    notes = sorted(p for p in (root / RESEARCH).glob("*.md") if p.name != "README.md")
    out = {"notes_scanned": 0, "declared": 0, "undeclared": 0, "takes_checked": 0,
           "takes_failing": 0, "no_frontmatter": [], "empty_takes_key": [], "unmeasured": [],
           "failures": []}
    for p in notes:
        rel = str(p.relative_to(root))
        text = p.read_text(encoding="utf-8", errors="replace")
        fm = frontmatter.parse(text)
        if not fm.raw_block and not fm.fields:
            out["notes_scanned"] += 1
            out["undeclared"] += 1
            out["no_frontmatter"].append(rel)
            out["unmeasured"].append(rel)
            continue
        out["notes_scanned"] += 1
        if "takes" not in fm.fields:
            out["undeclared"] += 1
            out["unmeasured"].append(rel)
            continue
        takes = fm.fields["takes"]
        if takes in ("", []):
            # A bare `takes:` says nothing. Only an explicit `takes: []` declares that
            # the note has no takes, so an emptied or half-written key stays unmeasured.
            out["undeclared"] += 1
            out["empty_takes_key"].append(rel)
            out["unmeasured"].append(rel)
            continue
        out["declared"] += 1
        if isinstance(takes, str):
            takes = [] if takes == "[]" else [takes]
        note_id = fm.get("id")
        seen: set[str] = set()
        shape = _shape_problem(fm.raw_block)
        if shape:
            out["takes_failing"] += 1
            out["failures"].append({"note": rel, "take": "(shape)", "problem": shape})
        for entry in takes:
            out["takes_checked"] += 1
            tid, problem = check_take(root, note_id, entry)
            if not problem and tid in seen:
                problem = "duplicate take id"
            if tid:
                seen.add(tid)
            if problem:
                out["takes_failing"] += 1
                out["failures"].append({"note": rel, "take": tid or entry, "problem": problem})
    return out


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--root", type=Path, default=DEFAULT_ROOT, help="repository root")
    ap.add_argument("--strict", action="store_true",
                    help="exit 1 if a declared take fails, or undeclared exceeds --max-undeclared")
    ap.add_argument("--max-undeclared", type=int, default=None, metavar="N")
    ap.add_argument("--json", action="store_true")
    args = ap.parse_args(argv)
    r = scan(args.root.resolve())
    over = args.max_undeclared is not None and r["undeclared"] > args.max_undeclared
    if args.json:
        print(json.dumps({**r, "undeclared_over_limit": over}, indent=2))
    else:
        for f in r["failures"]:
            print(f"FAIL {f['note']} take={f['take']}: {f['problem']}")
        for n in r["no_frontmatter"]:
            print(f"UNMEASURED {n}: no frontmatter")
        for n in r["empty_takes_key"]:
            print(f"UNMEASURED {n}: takes: key with no entries (write `takes: []` to declare none)")
        print(f"research-mint-check: notes scanned {r['notes_scanned']}, declared {r['declared']}, "
              f"undeclared {r['undeclared']} (unmeasured), takes checked {r['takes_checked']}, "
              f"takes failing {r['takes_failing']}")
    if r["takes_failing"] and not args.strict and not args.json:
        print("research-mint-check: failing takes are listed above; this plain run exits 0, --strict exits 1")
    if args.strict and (r["takes_failing"] or over):
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
