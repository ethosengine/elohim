#!/usr/bin/env python3
"""Tests for .claude/scripts/research-mint-check.py against a temporary tree."""
import importlib.util
import io
import json
import os
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", "..", ".."))
spec = importlib.util.spec_from_file_location(
    "research_mint_check", os.path.join(ROOT, ".claude", "scripts", "research-mint-check.py"))
rmc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(rmc)

BACKLOG = """# cluster

| # | Concern | Source |
|---|---|---|
| 12 | thing | from note-a |
| 15 | other | note-a too |
| 16 | unrelated | someone-else |
"""


def note(note_id, takes=None):
    fm = [f"id: {note_id}", "title: t"]
    if takes is not None:
        fm.append("takes:")
        fm += [f'  - "{t}"' for t in takes]
    return "---\n" + "\n".join(fm) + "\n---\n\nbody\n"


class Tree:
    def __init__(self, notes):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        (self.root / "genesis/research").mkdir(parents=True)
        (self.root / "genesis/data/timeline/backlog").mkdir(parents=True)
        (self.root / "genesis/data/timeline/backlog/cl.md").write_text(BACKLOG)
        (self.root / "docs").mkdir()
        (self.root / "docs/x.md").write_text("x")
        (self.root / "genesis/research/README.md").write_text("no takes here")
        for name, text in notes.items():
            (self.root / "genesis/research" / name).write_text(text)

    def run(self, *args):
        buf = io.StringIO()
        with redirect_stdout(buf):
            code = rmc.main(["--root", str(self.root), *args])
        return code, buf.getvalue()

    def facts(self):
        return json.loads(self.run("--json")[1])


class HomeForms(unittest.TestCase):
    def one(self, take):
        t = Tree({"a.md": note("note-a", [take])})
        self.addCleanup(t.tmp.cleanup)
        f = t.facts()
        self.assertEqual(f["takes_checked"], 1)
        return f

    def assertPass(self, take):
        self.assertEqual(self.one(take)["takes_failing"], 0, take)

    def assertFail(self, take, word):
        f = self.one(take)
        self.assertEqual(f["takes_failing"], 1, take)
        self.assertIn(word, f["failures"][0]["problem"])

    def test_row_passes_single_and_multi(self):
        self.assertPass("T1 | cl#12")
        self.assertPass("T1 | cl#12,15")

    def test_row_cluster_missing(self):
        self.assertFail("T1 | nope#12", "cluster not found")

    def test_row_missing(self):
        self.assertFail("T1 | cl#99", "row 99 not found")
        self.assertFail("T1 | cl#12,99", "row 99 not found")

    def test_row_does_not_name_note(self):
        self.assertFail("T1 | cl#16", "does not name")

    def test_file_passes_and_fails(self):
        self.assertPass("T4 | file: docs/x.md")
        self.assertFail("T4 | file: docs/missing.md", "file not found")
        self.assertFail("T4 | file:", "no path")

    def test_released_passes_and_fails(self):
        self.assertPass("T9 | released: superseded; see appendix")
        self.assertFail("T9 | released:   ", "no reason")

    def test_owed_is_always_failing_and_strict_refuses_it(self):
        tree = Tree({"a.md": note("note-a", ["T1 | cl#12", "T2 | owed: no row exists for this take", "T3 | owed:"])})
        r = rmc.scan(tree.root)
        self.assertEqual((r["takes_checked"], r["takes_failing"]), (3, 2))
        problems = sorted(f["problem"] for f in r["failures"])
        self.assertTrue(problems[0].startswith("owed with nothing said"))
        self.assertEqual(problems[1], "owed: no row exists for this take")
        self.assertEqual(tree.run("--strict")[0], 1)
        self.assertEqual(tree.run()[0], 0)

    def test_malformed(self):
        self.assertFail("T2 no separator", "malformed")
        self.assertFail("T2 | somewhere vague", "malformed home")


class NoteLevel(unittest.TestCase):
    def test_duplicate_id(self):
        t = Tree({"a.md": note("note-a", ["T1 | cl#12", "T1 | released: twice"])})
        self.addCleanup(t.tmp.cleanup)
        f = t.facts()
        self.assertEqual(f["takes_failing"], 1)
        self.assertIn("duplicate", f["failures"][0]["problem"])

    def test_undeclared_is_unmeasured_not_pass_or_fail(self):
        t = Tree({"a.md": note("note-a"), "b.md": note("note-b", ["T1 | released: x"])})
        self.addCleanup(t.tmp.cleanup)
        code, out = t.run()
        self.assertEqual(code, 0)
        self.assertIn("undeclared 1 (unmeasured)", out)
        f = t.facts()
        self.assertEqual((f["notes_scanned"], f["declared"], f["undeclared"]), (2, 1, 1))
        self.assertEqual(f["unmeasured"], ["genesis/research/a.md"])
        self.assertEqual(f["takes_failing"], 0)

    def test_plain_run_prints_failure_and_exits_zero(self):
        t = Tree({"a.md": note("note-a", ["T1 | cl#99"])})
        self.addCleanup(t.tmp.cleanup)
        code, out = t.run()
        self.assertEqual(code, 0)
        self.assertIn("FAIL genesis/research/a.md take=T1", out)


class Forms(unittest.TestCase):
    def tree(self, notes):
        t = Tree(notes)
        self.addCleanup(t.tmp.cleanup)
        return t

    def test_no_frontmatter_is_unmeasured(self):
        t = self.tree({"a.md": "# just prose\n"})
        f = t.facts()
        self.assertEqual((f["notes_scanned"], f["undeclared"]), (1, 1))
        self.assertEqual(f["no_frontmatter"], ["genesis/research/a.md"])
        self.assertIn("UNMEASURED genesis/research/a.md: no frontmatter", t.run()[1])

    def test_empty_flow_list_is_declared_zero(self):
        t = self.tree({"a.md": "---\nid: note-a\ntakes: []\n---\n"})
        f = t.facts()
        self.assertEqual((f["declared"], f["undeclared"], f["takes_checked"], f["takes_failing"]),
                         (1, 0, 0, 0))

    def test_bare_takes_key_is_unmeasured_not_declared(self):
        tree = Tree({"bare.md": "---\nid: bare\ntakes:\ncites:\n  - \"x\"\n---\n\nbody\n"})
        r = rmc.scan(tree.root)
        self.assertEqual((r["declared"], r["undeclared"], r["takes_failing"]), (0, 1, 0))
        self.assertEqual(len(r["empty_takes_key"]), 1)

    def test_list_followed_by_more_keys_read_whole(self):
        text = ('---\nid: note-a\ntakes:\n  - "T1 | cl#12"\n  - "T2 | released: r"\n'
                'cites:\n  - "x"\ntitle: t\n---\n')
        f = self.tree({"a.md": text}).facts()
        self.assertEqual((f["takes_checked"], f["takes_failing"]), (2, 0))


class Strict(unittest.TestCase):
    def test_strict_fails_on_failing_take(self):
        t = Tree({"a.md": note("note-a", ["T1 | cl#99"])})
        self.addCleanup(t.tmp.cleanup)
        self.assertEqual(t.run("--strict")[0], 1)

    def test_strict_passes_clean_and_ignores_undeclared_without_limit(self):
        t = Tree({"a.md": note("note-a", ["T1 | cl#12"]), "b.md": note("note-b")})
        self.addCleanup(t.tmp.cleanup)
        self.assertEqual(t.run("--strict")[0], 0)

    def test_strict_max_undeclared(self):
        t = Tree({"a.md": note("note-a"), "b.md": note("note-b")})
        self.addCleanup(t.tmp.cleanup)
        self.assertEqual(t.run("--strict", "--max-undeclared", "2")[0], 0)
        self.assertEqual(t.run("--strict", "--max-undeclared", "1")[0], 1)
        self.assertEqual(t.run("--max-undeclared", "1")[0], 0)  # not strict: report only


if __name__ == "__main__":
    unittest.main()
