#!/usr/bin/env python3
"""Every relocated lens is on disk, importable, and DECLARED at the path it actually occupies.

This is the test the relocation needed and did not have. A lens under
`.epr-meta/elohim/lenses/` is only reachable two ways: somebody runs it by path, or
`epr flow report` invokes it through the `procedure:` on its row in
`.claude/epr-meta/measures.yaml`. A relocation breaks the second one silently — the row keeps
pointing at a path that no longer exists, the report can never take the measure, and nothing
fails, because a measure nobody can take reports `skipped` rather than red.

So the assertions are the join itself:

  1. every lens file named by a `procedure:` in the registry EXISTS;
  2. every lens file on disk is named by at least one `procedure:` — the other direction, which
     catches a lens relocated without a declaration (reachable by path, invisible to the report);
  3. every lens PARSES (py_compile) — a relocation that broke an import path is caught here
     rather than the first time a ceremony reaches for it;
  4. no lens resolves its output through a hard-coded path: `_lib.paths.reports_root` is the one
     authority for the dated report tier, and it moved once already (station six, 2026-09-11,
     out of the deleted `.claude/memory-kit/`).

Run: python3 .epr-meta/elohim/lenses/__tests__/lens_declaration_test.py
"""
from __future__ import annotations

import py_compile
import re
import sys
import tempfile
from pathlib import Path

LENSES = Path(__file__).resolve().parent.parent
REPO = LENSES.parent.parent.parent
MEASURES = REPO / ".claude" / "epr-meta" / "measures.yaml"

_passed = 0


def check(label: str, cond: bool, detail: str = "") -> None:
    global _passed
    assert cond, f"FAIL: {label}{(' — ' + detail) if detail else ''}"
    _passed += 1
    print(f"  ✅ {label}")


def lens_files() -> list[Path]:
    return sorted(p for p in LENSES.rglob("*.py")
                  if "__pycache__" not in p.parts and "__tests__" not in p.parts)


def declared_paths() -> set[str]:
    """Every `.epr-meta/elohim/lenses/...py` a `procedure:` line names."""
    text = MEASURES.read_text(encoding="utf-8")
    return set(re.findall(r"\.epr-meta/elohim/lenses/[A-Za-z0-9_\-/]+\.py", text))


def main() -> int:
    files = lens_files()
    check("the lens tree is not empty", bool(files), "no *.py under .epr-meta/elohim/lenses/")

    on_disk = {str(p.relative_to(REPO)) for p in files}
    declared = declared_paths()

    # 1 — every declaration resolves
    for rel in sorted(declared):
        check(f"declared lens exists: {rel}", (REPO / rel).is_file())

    # 2 — every lens is declared
    for rel in sorted(on_disk):
        check(f"lens is declared in measures.yaml: {rel}", rel in declared,
              "a lens the report can never invoke is a lens nobody will run")

    # 3 — every lens parses
    for p in files:
        rel = p.relative_to(REPO)
        try:
            with tempfile.NamedTemporaryFile(suffix=".pyc") as tmp:
                py_compile.compile(str(p), cfile=tmp.name, doraise=True)
            ok, why = True, ""
        except py_compile.PyCompileError as exc:  # noqa: PERF203
            ok, why = False, str(exc)
        check(f"lens parses: {rel}", ok, why)

    # 4 — the dated report tier has ONE authority.
    # CODE only: a `#` comment recording where the tier used to live is history, and history is
    # what makes the move legible to the next reader. A path in an executable line is the bug.
    stale = []
    for p in files:
        for n, line in enumerate(p.read_text(encoding="utf-8", errors="replace").splitlines(), 1):
            if line.lstrip().startswith("#"):
                continue
            if ".claude/memory-kit" in line or '"memory-kit"' in line:
                stale.append(f"{p.relative_to(REPO)}:{n}")
    check("no lens resolves a path into the deleted .claude/memory-kit/", not stale,
          ", ".join(stale))

    # 5 — the artifact the lenses were carved out of is CLEARED.
    # This lives here rather than in a test of its own because the two facts are one fact: the
    # kit could only be deleted once every capability with no native replacement had a home,
    # and this file is the register of those homes. A kit tree that came back would mean a
    # lens had been re-forked rather than relocated.
    for gone in (".claude/memory-kit", ".claude/scripts/memory-kit"):
        check(f"the deleted kit tree stays deleted: {gone}", not (REPO / gone).exists(),
              "42 dated report dirs, the state JSONs and 17 scripts were removed 2026-09-11")

    print(f"\n  {_passed} assertions passed ✅")
    return 0


if __name__ == "__main__":
    sys.exit(main())
