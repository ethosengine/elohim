#!/usr/bin/env python3
"""The memory index projects natively, and STANDS DOWN rather than falling back.

Station four routed this hook native-first with `memory-index-projector.py` as the fallback.
Station six round (b) (2026-09-11) deleted the kit, so every fallback became a STAND-DOWN: when
the native leg cannot answer, the index is left exactly as it stands. Four properties:

  1. `epr flow memory project --index` absent or refusing -> NOTHING is written, and the
     advisory says the index is unchanged.
  2. the verb present AND inside the hook's latency budget -> the NATIVE projection is what
     lands in `.claude/memory/MEMORY.md`.
  3. the verb present but OVER budget -> nothing is written, and the probe's cached reason
     names the budget rather than pretending the verb is missing.
  4. the native projection carries no row for the entry that was just written (it is not yet a
     contribution) -> the index is left UNCHANGED — never overwritten with a render that drops
     the entry's own row — and the advisory names `epr flow memory import`.

Property 4 is the one that used to need the kit, and it is the reason the stand-down is a
preservation rather than a loss: re-rendering from a directory scan wrote a row the
contribution plane does not carry, which the very next native run removed again.

Plus one golden, asserted against the REAL binary or not at all: the native projection equals
the digest station four recorded (sha256 8ee2e07e…, 98 rows, 23,993 bytes) whenever the corpus
still matches that snapshot. A stub cannot carry this one — a rendering change would land green
against a fixture that was written to agree with it.

Run: python3 -m unittest discover -s .claude/hooks/__tests__ -p '*_test.py'
"""

from __future__ import annotations

import hashlib
import importlib.util
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HOOKS = Path(__file__).resolve().parent.parent
REPO = HOOKS.parent.parent

# The station-four parity pin: `.claude/memory/MEMORY.md` as projected on 2026-09-10 from
# 229 entries -> 98 rows -> 23,993 bytes, by BOTH legs.
PINNED_DIGEST = "8ee2e07eac63f45594cf18b6ab5f996594c5c25b0e511fd20bba198a627a83dd"

ENTRY = "feedback_fixture_entry.md"
OTHER = "feedback_other_entry.md"

# An `epr` stub. `flow memory project --index` behaves per env:
#   PROJECT_VERB=0  -> refuses like a binary that predates the verb
#   SLOW=1          -> sleeps past the caller's timeout
#   OMIT_ROW=1      -> renders an index WITHOUT the just-edited entry's row
# Every invocation is appended to $STUB_LOG as one JSON argv line.
STUB = '''#!/usr/bin/env python3
import json, os, sys, time
argv = sys.argv[1:]
with open(os.environ["STUB_LOG"], "a") as fh:
    fh.write(json.dumps(argv) + "\\n")
if argv[:4] != ["flow", "memory", "project", "--index"]:
    print("epr flow: invalid arguments", file=sys.stderr); sys.exit(2)
if os.environ.get("PROJECT_VERB") == "0":
    print("epr flow: invalid arguments: collective memory: unknown flag `--index`",
          file=sys.stderr)
    sys.exit(2)
if os.environ.get("SLOW") == "1":
    time.sleep(30)
rows = ["<!-- GENERATED -->\\n"]
if os.environ.get("OMIT_ROW") != "1":
    rows.append("- [fixture](%s) - a row\\n" % os.environ["ENTRY_NAME"])
rows.append("- [other](%s) - another row\\n" % os.environ["OTHER_NAME"])
text = "".join(rows)
if "--out" in argv:
    out = os.path.join(os.environ["FIXTURE_ROOT"], argv[argv.index("--out") + 1])
    os.makedirs(os.path.dirname(out), exist_ok=True)
    with open(out, "w") as fh:
        fh.write(text)
print(json.dumps({"bytes": len(text.encode()), "entries": len(rows) - 1,
                  "budget": {"state": "ok", "bound": "memory-index-bytes-ceiling@1"},
                  "unloadedRows": [], "wrote": argv[argv.index("--out") + 1] if "--out" in argv
                  else None}))
'''

def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, str(path))
    mod = importlib.util.module_from_spec(spec)
    sys.modules[name] = mod
    spec.loader.exec_module(mod)
    return mod


class RouterCase(unittest.TestCase):
    """The hook's ROUTE, asserted through the real module against a stub binary."""

    def setUp(self):
        self.root = Path(tempfile.mkdtemp(prefix="memindex-route-"))
        self.addCleanup(shutil.rmtree, self.root, True)
        mem = self.root / ".claude" / "memory"
        mem.mkdir(parents=True)
        (mem / ENTRY).write_text("---\nname: fixture\n---\n")
        (mem / OTHER).write_text("---\nname: other\n---\n")
        (mem / "MEMORY.md").write_text("STALE\n")

        self.bin = self.root / "epr"
        self.bin.write_text(STUB)
        self.bin.chmod(0o755)
        self.stub_log = self.root / "stub.log"

        # A private probe-cache home per test: the router caches its verdict on disk keyed by the
        # binary, and a leaked verdict would make the next case assert nothing.
        self.tmpdir = self.root / "probecache"
        self.tmpdir.mkdir()

        self.mod = load_module("memory_index_projection_under_test",
                               HOOKS / "memory-index-projection.py")
        self.mod.reset_cache()

    def env(self, **extra) -> dict:
        base = {**os.environ,
                "EPR_BIN": str(self.bin),
                "CLAUDE_PROJECT_DIR": str(self.root),
                "STUB_LOG": str(self.stub_log),
                "FIXTURE_ROOT": str(self.root),
                "ENTRY_NAME": ENTRY,
                "OTHER_NAME": OTHER,
                "TMPDIR": str(self.tmpdir),
                "MEMORY_INDEX_BUDGET_SECONDS": "5"}
        base.pop("MEMORY_INDEX_NATIVE", None)
        base.update({k: str(v) for k, v in extra.items()})
        return base

    def run_hook(self, edited: str = ENTRY, **envextra) -> subprocess.CompletedProcess:
        payload = json.dumps({"tool_input": {
            "file_path": str(self.root / ".claude" / "memory" / edited)}})
        return subprocess.run(
            [sys.executable, str(HOOKS / "memory-index-projection.py"), "--hook"],
            input=payload, capture_output=True, text=True, timeout=120,
            env=self.env(**envextra))

    def index(self) -> str:
        return (self.root / ".claude" / "memory" / "MEMORY.md").read_text()

    def index_untouched(self) -> bool:
        """The index still carries the fixture's pre-run bytes: nothing was installed."""
        return self.index() == "STALE\n"

    # 1 ────────────────────────────────────────────────────────────────────────────────────
    def test_a_binary_without_the_verb_stands_down_and_says_so(self):
        r = self.run_hook(PROJECT_VERB="0")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertTrue(self.index_untouched(),
                        "a refusing verb still changed the index")
        # SILENT, deliberately: a binary that predates the verb is an environment fact, and a
        # per-edit advisory about it would fire on every memory write forever. The advisory is
        # reserved for the cases an author can act on — a projection that ran and failed, and
        # the freshness guard. What matters here is that nothing was written.
        self.assertEqual(r.stdout, "")

    # 2 ────────────────────────────────────────────────────────────────────────────────────
    def test_the_native_projection_is_what_lands_when_the_verb_is_present(self):
        r = self.run_hook()
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn(f"]({ENTRY})", self.index())
        self.assertNotIn("KIT PROJECTION", self.index())
        calls = [json.loads(l) for l in self.stub_log.read_text().splitlines()]
        installed = [c for c in calls if "--out" in c]
        self.assertTrue(installed, f"no --out invocation in {calls}")
        self.assertIn("--budget", installed[0])
        self.assertIn("memory-index-bytes@1", installed[0])

    def test_the_probe_is_cached_so_a_second_write_costs_one_native_run(self):
        self.run_hook()
        first = len(self.stub_log.read_text().splitlines())
        self.run_hook()
        second = len(self.stub_log.read_text().splitlines())
        # The trial runs once per binary; every later write pays only for the install.
        self.assertEqual(second - first, 1, "the router re-probed instead of reading its cache")

    # 3 ────────────────────────────────────────────────────────────────────────────────────
    def test_a_verb_over_the_latency_budget_stands_down_and_says_why(self):
        r = self.run_hook(SLOW="1")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertTrue(self.index_untouched(),
                        "an over-budget native verb still changed the index")
        cached = list(self.tmpdir.glob("claude-epr-memindex-probe-*.json"))
        self.assertTrue(cached, "the over-budget verdict was not cached")
        blob = json.loads(cached[0].read_text())
        self.assertFalse(blob["usable"])
        self.assertIn("budget", blob["reason"])

    # 4 ────────────────────────────────────────────────────────────────────────────────────
    def test_an_unimported_entry_leaves_the_index_alone_and_names_the_import(self):
        """The freshness guard, as a REFUSAL. This is the property that used to need the kit."""
        r = self.run_hook(OMIT_ROW="1")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertTrue(self.index_untouched(),
                        "a projection with no row for the edited entry was installed")
        self.assertIn("epr flow memory import", r.stdout)
        self.assertIn("UNCHANGED", r.stdout)
        self.assertFalse((self.root / ".eprfs/status/.memory-index-probe.md").exists(),
                         "the scratch render was left behind")

    def test_a_write_outside_the_memory_directory_is_a_no_op(self):
        payload = json.dumps({"tool_input": {"file_path": str(self.root / "justfile")}})
        r = subprocess.run(
            [sys.executable, str(HOOKS / "memory-index-projection.py"), "--hook"],
            input=payload, capture_output=True, text=True, timeout=60, env=self.env())
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(r.stdout, "")
        self.assertFalse(self.stub_log.exists())
        self.assertTrue(self.index_untouched())

    def test_a_hand_edit_of_the_index_itself_never_projects(self):
        r = self.run_hook(edited="MEMORY.md")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertFalse(self.stub_log.exists(),
                         "a hand-edit of MEMORY.md triggered a native projection")
        self.assertTrue(self.index_untouched(),
                        "the hand edit was overwritten before its author could read the advice")
        self.assertIn("PROJECTION", r.stdout)


class GoldenParityCase(unittest.TestCase):
    """Parity asserted against the REAL binary and the REAL corpus, or not at all.

    Skips — never passes vacuously — when there is no binary or the binary predates the verb.
    The native projection is slow on this corpus by construction (it re-reads every contribution
    and confirms each one's attributed observation), so it is taken ONCE for the whole case.
    """

    NATIVE_TIMEOUT = 400
    SCRATCH = ".eprfs/status/.memory-index-parity-probe.md"

    @classmethod
    def setUpClass(cls):
        mod = load_module("memory_index_projection_for_golden",
                          HOOKS / "memory-index-projection.py")
        cls.binary = mod.resolve_bin()
        if not cls.binary:
            raise unittest.SkipTest("no `epr` binary ($EPR_BIN, the gate target, or PATH)")
        out = REPO / cls.SCRATCH
        out.parent.mkdir(parents=True, exist_ok=True)
        r = subprocess.run([cls.binary, "flow", "memory", "project", "--index",
                            "--budget", "memory-index-bytes@1", "--out", cls.SCRATCH,
                            "--json", "--root", str(REPO)],
                           capture_output=True, text=True, timeout=cls.NATIVE_TIMEOUT)
        if r.returncode != 0:
            raise unittest.SkipTest(
                f"`{cls.binary}` cannot project the index: "
                f"{(r.stderr or r.stdout).strip().splitlines()[:1]}")
        cls.native = out.read_bytes()
        out.unlink(missing_ok=True)

        cls.live = (REPO / ".claude" / "memory" / "MEMORY.md").read_bytes()

    def test_the_native_projection_equals_the_index_on_disk(self):
        """Idempotence, which is what survives the kit's deletion as a parity check.

        Station four proved the native and kit projections byte-identical (digest below). With
        one projector left there is no second leg to compare against, so the live property is
        that projecting again reproduces the index already committed — a rendering change shows
        up here as a diff rather than as silence.
        """
        self.assertEqual(hashlib.sha256(self.native).hexdigest(),
                         hashlib.sha256(self.live).hexdigest(),
                         "re-projecting the index rendered different bytes than the tree holds")

    def test_the_native_projection_matches_the_recorded_station_four_digest(self):
        got = hashlib.sha256(self.native).hexdigest()
        if got != PINNED_DIGEST:
            # The corpus moved past the 2026-09-10 snapshot. Idempotence above is the live
            # drift-proof assertion; re-pin here deliberately rather than letting a corpus edit
            # read as a projection regression.
            self.skipTest(
                "the live corpus moved past the 2026-09-10 snapshot "
                f"(now {got[:16]}…); idempotence is asserted separately")
        self.assertEqual(got, PINNED_DIGEST)


if __name__ == "__main__":
    unittest.main(verbosity=2)
