#!/usr/bin/env python3
"""The drift-signal contract: a drift signal is a fold, and ONLY a fold.

Station one made each hook write BOTH a structured observation and its private JSON
accumulator, because the kit was still the producer of the accumulated counts. Station six
round (b) (2026-09-11) deleted `.claude/memory-kit/` once every accumulated count had a native
derivation, so the properties under test inverted. What is asserted now:

  1. `epr flow note --measure` ABSENT  -> the hook writes NOTHING. There is no second store to
     fall back to, and a hook that invented one would be re-growing the thing just removed.
  2. `epr flow note --measure` PRESENT -> exactly one observation per affected subject, with
     the declared measure id, subject and value.
  3. The SessionStart headline is `epr flow report --headline` and nothing else — no producer
     bridge, no kit re-run, no `(fallback: memory-kit)` line.
  4. The accumulated counts the hooks used to keep are DERIVED, asserted against the real
     binary in GoldenReportCase.

Run: python3 -m unittest discover -s .claude/hooks/__tests__ -p '*_test.py'
"""

from __future__ import annotations

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

PLAN_DOC = "genesis/docs/superpowers/plans/2026-01-01-fixture-plan.md"

# An `epr` stub. `--measure` appears in its `flow note --help` output only when
# MEASURE_VERB=1, so the same stub exercises both sides of the probe. Every invocation is
# appended to $STUB_LOG as one JSON argv line.
STUB = '''#!/usr/bin/env python3
import json, os, sys
argv = sys.argv[1:]
with open(os.environ["STUB_LOG"], "a") as fh:
    fh.write(json.dumps(argv) + "\\n")
has_measure = os.environ.get("MEASURE_VERB") == "1"
if argv[:2] == ["flow", "note"] and "--help" in argv:
    print("usage: epr flow note --on <x> --kind <k> --reason <r>" +
          (" --measure <id@version> --subject <path> --value <n>" if has_measure else ""))
    sys.exit(0)
if argv[:2] == ["flow", "note"]:
    if not has_measure:
        sys.exit(2)
    # mirror the real verb: --measure REFUSES --on, and requires --subject + numeric --value
    if "--on" in argv:
        print("--measure takes --subject, not --on", file=sys.stderr); sys.exit(2)
    if "--subject" not in argv or "--value" not in argv:
        print("note --measure needs --subject and --value", file=sys.stderr); sys.exit(2)
    try:
        float(argv[argv.index("--value") + 1])
    except ValueError:
        print("--value is not a number", file=sys.stderr); sys.exit(2)
    sys.exit(0)
if argv[:2] == ["flow"] and len(argv) == 1:
    print("usage: epr flow <project | note --on <x> --kind <k>" +
          (" --measure <id@version>" if has_measure else "") + ">")
    sys.exit(0)
if argv[:2] == ["flow", "report"]:
    # Deliberately OPAQUE: this stub answers whether the hook CHOSE the native path, never
    # what a report line looks like. Report formats are asserted against the real binary in
    # GoldenReportCase, or not at all.
    if os.environ.get("REPORT_VERB") == "1":
        print(os.environ.get("REPORT_TEXT", "native headline"))
        sys.exit(0)
    print("epr flow: unknown flow subcommand `report`", file=sys.stderr)
    sys.exit(2)
sys.exit(2)
'''


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    mod = importlib.util.module_from_spec(spec)
    sys.modules[name] = mod
    spec.loader.exec_module(mod)
    return mod


class DriftObservationCase(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix="drift-obs-"))
        self.addCleanup(shutil.rmtree, self.tmp, True)
        self.project = self.tmp / "project"
        (self.project / ".claude" / "hooks").mkdir(parents=True)
        (self.project / ".eprfs" / "status" / "lenses").mkdir(parents=True)
        shutil.copy(HOOKS / "_observation.py", self.project / ".claude" / "hooks")
        doc = self.project / PLAN_DOC
        doc.parent.mkdir(parents=True, exist_ok=True)
        doc.write_text("---\nstatus: landed\nlanded_commit: abc1234\n---\n\n# fixture\n")
        self.doc = doc
        self.log = self.tmp / "stub.log"
        self.stub = self.tmp / "epr"
        self.stub.write_text(STUB)
        self.stub.chmod(0o755)
        # An isolated TMPDIR keeps the probe's disk cache out of the real one.
        self.probe_tmp = self.tmp / "probe-tmp"
        self.probe_tmp.mkdir()

    def env(self, **extra) -> dict:
        e = dict(os.environ)
        e.update({
            "CLAUDE_PROJECT_DIR": str(self.project),
            "EPR_BIN": str(self.stub),
            "STUB_LOG": str(self.log),
            "TMPDIR": str(self.probe_tmp),
        })
        e.update(extra)
        return e

    def run_hook(self, hook: str, payload: dict, **envextra) -> subprocess.CompletedProcess:
        return subprocess.run(
            [sys.executable, str(HOOKS / hook)],
            input=json.dumps(payload), capture_output=True, text=True,
            env=self.env(**envextra), timeout=60,
        )

    def stub_calls(self) -> list[list[str]]:
        if not self.log.exists():
            return []
        return [json.loads(line) for line in self.log.read_text().splitlines() if line.strip()]

    def json_accumulators(self) -> list[Path]:
        """Every `*.json` a hook could have written anywhere under the fixture project.

        Asserted EMPTY rather than asserted-absent-by-name: naming the six files the kit used
        would pass for a hook that invented a seventh, which is the regression this guards.
        The fixture's own inputs (`cites-index.json`, the sovereignty jsonl ledger) are the
        two exceptions, and they are named.
        """
        allowed = {".eprfs/status/lenses/cites-index.json"}
        out = []
        for f in self.project.rglob("*.json"):
            rel = f.relative_to(self.project).as_posix()
            if rel in allowed or rel.startswith(".claude/data/"):
                continue
            out.append(f)
        return out

    # 1 ────────────────────────────────────────────────────────────────────────────────────
    def test_verb_absent_writes_nothing_at_all(self):
        """No verb, no fold, NO FALLBACK STORE. The signal is simply lost for that edit.

        That is the deliberate trade of station six round (b): a drift signal is worth a fold
        or it is worth nothing, and a private JSON accumulator nobody can address is what the
        whole replacement exists to end. The hook still exits 0 and still prints nothing.
        """
        r = self.run_hook("placement-drift-signal.py",
                          {"tool_name": "Write", "tool_input": {"file_path": str(self.doc)}},
                          MEASURE_VERB="0")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(r.stdout, "", "a hook must never narrate")
        self.assertEqual(self.json_accumulators(), [],
                         "no verb must not mean a re-grown private store")
        # It probed, and it never tried to append a real observation.
        self.assertTrue(any(c[:2] == ["flow", "note"] and "--help" in c for c in self.stub_calls()))
        self.assertFalse(any(c[:2] == ["flow", "note"] and "--measure" in c
                             for c in self.stub_calls()))

    # 2 ────────────────────────────────────────────────────────────────────────────────────
    def test_verb_present_appends_only_the_fold(self):
        """ONE store. `cleanup-pressure-ceiling@1` derives its count from these folds
        (`derive: distinct-subjects-since-reset`), so a JSON accumulator beside them would be
        a second, unaddressable history of the same events."""
        r = self.run_hook("placement-drift-signal.py",
                          {"tool_name": "Write", "tool_input": {"file_path": str(self.doc)}},
                          MEASURE_VERB="1")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(r.stdout, "", "the native path must print nothing extra")
        self.assertEqual(self.json_accumulators(), [],
                         "the fold is the ONLY store; no JSON beside it")
        notes = [c for c in self.stub_calls() if c[:2] == ["flow", "note"] and "--help" not in c]
        self.assertEqual(len(notes), 1, self.stub_calls())
        argv = notes[0]
        self.assertIn("--kind", argv)
        self.assertEqual(argv[argv.index("--kind") + 1], "observation")
        self.assertNotIn("--on", argv, "the native verb refuses --on alongside --measure")
        self.assertEqual(argv[argv.index("--measure") + 1], "placement-drift-due@1")
        self.assertEqual(argv[argv.index("--subject") + 1], PLAN_DOC)
        self.assertEqual(argv[argv.index("--value") + 1], "1")

    def test_verb_present_reopened_doc_observes_zero(self):
        self.doc.write_text("---\nstatus: proposed\n---\n\n# fixture\n")
        r = self.run_hook("placement-drift-signal.py",
                          {"tool_name": "Edit", "tool_input": {"file_path": str(self.doc)}},
                          MEASURE_VERB="1")
        self.assertEqual(r.returncode, 0, r.stderr)
        notes = [c for c in self.stub_calls() if c[:2] == ["flow", "note"] and "--help" not in c]
        self.assertEqual(len(notes), 1, self.stub_calls())
        self.assertEqual(notes[0][notes[0].index("--value") + 1], "0")

    def test_map_drift_signal_uses_its_own_measure(self):
        seed = self.project / "genesis/docs/content/elohim-protocol/architecture/seed.md"
        seed.parent.mkdir(parents=True, exist_ok=True)
        seed.write_text("# seed\n")
        r = self.run_hook("map-drift-signal.py",
                          {"tool_name": "Write", "tool_input": {"file_path": str(seed)}},
                          MEASURE_VERB="1")
        self.assertEqual(r.returncode, 0, r.stderr)
        notes = [c for c in self.stub_calls() if c[:2] == ["flow", "note"] and "--help" not in c]
        self.assertEqual(len(notes), 1, self.stub_calls())
        self.assertEqual(notes[0][notes[0].index("--measure") + 1], "map-currency-drift@1")

    # 3 ────────────────────────────────────────────────────────────────────────────────────
    def _budget(self, **envextra) -> str:
        mod = load_module("load_project_context_under_test", HOOKS / "load-project-context.py")
        saved = dict(os.environ)
        os.environ.update(self.env(**envextra))
        try:
            return mod.get_memory_budget(str(self.project))
        finally:
            os.environ.clear()
            os.environ.update(saved)

    def test_headline_prefers_native(self):
        out = self._budget(REPORT_VERB="1", REPORT_TEXT="memkit: NATIVE LINE")
        self.assertEqual(out, "memkit: NATIVE LINE")
        self.assertNotIn("fallback: memory-kit", out)

    def test_headline_is_empty_without_the_native_verb(self):
        """`epr flow report --headline` is the SOLE owner of the SessionStart headline.

        Station six round (a) removed the last-resort kit re-run; round (b) removed the
        producer bridge that ran before it. A tree without the verb gets NO headline — not a
        `(fallback: memory-kit)` line, and not a bridged value, because every slot is derived
        and bridging a derived value would double it.
        """
        out = self._budget(REPORT_VERB="0")
        self.assertEqual(out, "")
        self.assertNotIn("fallback: memory-kit", out)
        self.assertFalse([c for c in self.stub_calls()
                          if c[:2] == ["flow", "note"] and "--measure" in c],
                         "the budget path must not fold anything of its own")

    def test_headline_cache_is_written_only_from_the_native_path(self):
        mod = load_module("load_project_context_under_test", HOOKS / "load-project-context.py")
        cache = Path(mod._headline_cache_path(str(self.project)))
        cache.unlink(missing_ok=True)
        self._budget(REPORT_VERB="1", REPORT_TEXT="native")
        self.assertTrue(cache.is_file())
        self.assertIn("native", cache.read_text())
        # No native verb: nothing is cached, so a later consumer recomputes rather than
        # inheriting a kit-produced headline it cannot attribute.
        cache.unlink(missing_ok=True)
        self._budget(REPORT_VERB="0")
        self.assertFalse(cache.is_file())


    def test_map_walk_refresh_stamps_the_reset_not_a_per_subject_zero(self):
        """A MAP.md edit is a bulk clear, so it must stamp the ceiling's declared reset.

        The Rust half drops value-0 observations, so a per-subject zero on MAP.md would be
        silently inert and `map-currency-drift-ceiling@1` would keep counting every seed
        logged before the walk was refreshed. This is the Python half of that contract; it
        was unpinned, which is how the zero survived the move to the derived world.
        """
        walk = self.project / "genesis/docs/content/elohim-protocol/architecture/MAP.md"
        walk.parent.mkdir(parents=True, exist_ok=True)
        walk.write_text("# the walk\n")
        r = self.run_hook("map-drift-signal.py",
                          {"tool_name": "Edit", "tool_input": {"file_path": str(walk)}},
                          MEASURE_VERB="1")
        self.assertEqual(r.returncode, 0, r.stderr)
        notes = [c for c in self.stub_calls() if c[:2] == ["flow", "note"] and "--measure" in c]
        self.assertEqual(len(notes), 1, self.stub_calls())
        argv = notes[0]
        self.assertEqual(argv[argv.index("--measure") + 1], "map-currency-drift-reset@1")
        self.assertEqual(argv[argv.index("--subject") + 1], ".", "the reset is repository-wide")
        self.assertEqual(argv[argv.index("--value") + 1], "1")
        # and NO per-subject zero on the walk artifact
        self.assertNotIn("map-currency-drift@1", argv)
        self.assertNotEqual(argv[argv.index("--value") + 1], "0")
        # and no private accumulator was emptied, because there is no longer one to empty:
        # the reset observation IS the bulk clear, and the report counts from it forward.
        self.assertEqual(self.json_accumulators(), [])

    def test_a_seed_change_still_folds_its_own_subject(self):
        """The other arm: only MAP.md resets; a seed is one subject at value 1."""
        seed = self.project / "genesis/docs/content/elohim-protocol/architecture/seed.md"
        seed.parent.mkdir(parents=True, exist_ok=True)
        seed.write_text("# seed\n")
        self.run_hook("map-drift-signal.py",
                      {"tool_name": "Write", "tool_input": {"file_path": str(seed)}},
                      MEASURE_VERB="1")
        notes = [c for c in self.stub_calls() if c[:2] == ["flow", "note"] and "--measure" in c]
        self.assertEqual(len(notes), 1, self.stub_calls())
        argv = notes[0]
        self.assertEqual(argv[argv.index("--measure") + 1], "map-currency-drift@1")
        self.assertEqual(argv[argv.index("--subject") + 1],
                         "genesis/docs/content/elohim-protocol/architecture/seed.md")
        self.assertEqual(argv[argv.index("--value") + 1], "1")

    def test_no_rewired_hook_writes_a_json_accumulator(self):
        """The regression risk inverted: prove NONE of the six re-grows a private store.

        Run on BOTH sides of the probe. With the verb absent a hook has the strongest
        temptation to keep something; with it present a hook could still write "as well".
        Neither is allowed: `.claude/memory-kit/` was deleted at station six round (b) and the
        counts it held are derived from the fold plane.
        """
        lenses = self.project / ".eprfs" / "status" / "lenses"
        (self.project / "CLAUDE.md").write_text("# gospel\n")
        (lenses / "cites-index.json").write_text(json.dumps({"cites": {"src/**": ["e"]}}))
        src = self.project / "src" / "thing.rs"
        src.parent.mkdir(parents=True, exist_ok=True)
        src.write_text("fn main() {}\n")
        seed = self.project / "genesis/docs/content/elohim-protocol/architecture/seed.md"
        seed.parent.mkdir(parents=True, exist_ok=True)
        seed.write_text("# seed\n")
        sov = self.project / "note.md"
        sov.write_text("The learner is self-sovereign here.\n")

        cases = [
            ("placement-drift-signal.py", {"tool_input": {"file_path": str(self.doc)}}),
            ("map-drift-signal.py", {"tool_input": {"file_path": str(seed)}}),
            ("claude-md-drift-signal.py", {"tool_input": {"file_path": str(src)}}),
            ("claude-md-structural-signal.py",
             {"tool_input": {"command": f"mv {self.project}/a.md {self.project}/b.md"}}),
            ("memory-coherence-signal.py", {"tool_input": {"file_path": str(src)}}),
            ("sovereignty-guard-signal.py",
             {"tool_name": "Write", "tool_input": {"file_path": str(sov)}}),
        ]
        for verb in ("0", "1"):
            for hook, payload in cases:
                with self.subTest(hook=hook, measure_verb=verb):
                    r = self.run_hook(hook, payload, MEASURE_VERB=verb)
                    self.assertEqual(r.returncode, 0, r.stderr)
                    self.assertEqual(self.json_accumulators(), [],
                                     f"{hook} wrote a private store with MEASURE_VERB={verb}")
        self.assertFalse((self.project / ".claude" / "memory-kit").exists(),
                         "no hook may recreate the deleted kit directory")

    # every rewired hook, one native emission each ───────────────────────────────────────
    def test_claude_md_drift_signal_uses_its_own_measure(self):
        (self.project / "CLAUDE.md").write_text("# gospel\n")
        target = self.project / "src.rs"
        target.write_text("fn main() {}\n")
        self.run_hook("claude-md-drift-signal.py",
                      {"tool_name": "Edit", "tool_input": {"file_path": str(target)}},
                      MEASURE_VERB="1")
        notes = [c for c in self.stub_calls() if c[:2] == ["flow", "note"] and "--help" not in c]
        self.assertEqual([n[n.index("--measure") + 1] for n in notes],
                         ["claude-md-edit-signal@1"], self.stub_calls())
        self.assertEqual(self.json_accumulators(), [], "the fold is the only store")

    def test_claude_md_structural_signal_uses_its_own_measure(self):
        (self.project / "CLAUDE.md").write_text("# gospel\n")
        self.run_hook("claude-md-structural-signal.py",
                      {"tool_name": "Bash",
                       "tool_input": {"command": f"mv {self.project}/a.md {self.project}/b.md"}},
                      MEASURE_VERB="1")
        notes = [c for c in self.stub_calls() if c[:2] == ["flow", "note"] and "--help" not in c]
        self.assertTrue(notes, self.stub_calls())
        self.assertEqual({n[n.index("--measure") + 1] for n in notes},
                         {"claude-md-structural-signal@1"})
        self.assertEqual(self.json_accumulators(), [], "the fold is the only store")

    def test_memory_coherence_signal_uses_its_own_measure(self):
        (self.project / ".eprfs/status/lenses/cites-index.json").write_text(
            json.dumps({"cites": {"src/**": ["some-entry"]}}))
        target = self.project / "src" / "thing.rs"
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text("fn main() {}\n")
        self.run_hook("memory-coherence-signal.py",
                      {"tool_name": "Edit", "tool_input": {"file_path": str(target)}},
                      MEASURE_VERB="1")
        notes = [c for c in self.stub_calls() if c[:2] == ["flow", "note"] and "--help" not in c]
        self.assertEqual(len(notes), 1, self.stub_calls())
        self.assertEqual(notes[0][notes[0].index("--measure") + 1], "memory-coherence-drift@1")
        self.assertEqual(notes[0][notes[0].index("--subject") + 1], ".claude/memory/some-entry.md")
        self.assertEqual(self.json_accumulators(), [], "the fold is the only store")

    def test_sovereignty_guard_signal_uses_its_own_measure(self):
        doc = self.project / "note.md"
        doc.write_text("The learner is self-sovereign here.\n")
        r = self.run_hook("sovereignty-guard-signal.py",
                          {"tool_name": "Write", "tool_input": {"file_path": str(doc)}},
                          MEASURE_VERB="1")
        notes = [c for c in self.stub_calls() if c[:2] == ["flow", "note"] and "--help" not in c]
        self.assertEqual(len(notes), 1, self.stub_calls())
        self.assertEqual(notes[0][notes[0].index("--measure") + 1], "sovereignty-landings@1")
        self.assertEqual(self.json_accumulators(), [], "the fold is the only store")
        # The tally it escalates on is READ BACK from the bound, not kept: the stub answers
        # `flow report` with exit 2 here, so `bound_count` returns None and the message falls
        # back to THIS edit's landings rather than printing a number nobody measured.
        self.assertTrue(any(c[:3] == ["flow", "report", "--bound"] for c in self.stub_calls()),
                        f"the guard never asked for the accumulated count: {self.stub_calls()}")
        # the guard keeps its own PostToolUse message — only the private tally moved
        self.assertIn("sovereignty-guard", r.stdout)
        # ... and the jsonl landing ledger is untouched by this station
        self.assertTrue((self.project / ".claude/data/sovereignty-guard.jsonl").is_file())


    # the bridge is gone ──────────────────────────────────────────────────────────────────
    def test_no_hook_path_runs_a_producer_bridge(self):
        """`_observation` exposes no `bridge_headline`, and nothing calls one.

        The bridge folded kit-produced headline values onto their declared bounds while two of
        the five slots had no native producer. Both got one at station six round (b)
        (`memkit-report-tier-mb@1` -> `status: superseded`; `mempalace-surfaces-changed@1` ->
        `derive: files-newer-than`), at which point bridging would DOUBLE a derived value.
        Asserted as an absence because an absence is exactly what can be silently undone.
        """
        mod = load_module("observation_under_test", HOOKS / "_observation.py")
        self.assertFalse(hasattr(mod, "bridge_headline"))
        self.assertFalse(hasattr(mod, "parse_headline"))
        self.assertEqual(mod._HEADLINE_BRIDGE if hasattr(mod, "_HEADLINE_BRIDGE") else (), ())
        for hook in ("load-project-context.py", "delivery-gate.py"):
            self.assertNotIn("bridge_headline", (HOOKS / hook).read_text(),
                             f"{hook} still calls the removed bridge")

    def test_bound_count_reads_the_accumulation_from_the_report(self):
        """The inverse of `emit`: the one hook that needs an accumulated number asks for it."""
        mod = load_module("observation_under_test", HOOKS / "_observation.py")
        saved = dict(os.environ)
        os.environ.update(self.env(MEASURE_VERB="1", REPORT_VERB="1", REPORT_TEXT=json.dumps({
            "recipes": [{"outcomes": [
                {"bound": "sovereignty-landings-ceiling@1", "contributingFolds": 7}]}]})))
        try:
            mod.reset_cache()
            self.assertEqual(
                mod.bound_count("sovereignty-landings-ceiling", root=str(self.project)), 7)
            # an unreadable report is None, never 0 — "could not measure" is not "measured zero"
            os.environ["REPORT_VERB"] = "0"
            mod.reset_cache()
            self.assertIsNone(
                mod.bound_count("sovereignty-landings-ceiling", root=str(self.project)))
        finally:
            os.environ.clear()
            os.environ.update(saved)
            mod.reset_cache()


class ObservationHelperCase(unittest.TestCase):
    def test_missing_binary_costs_no_subprocess(self):
        mod = load_module("observation_under_test", HOOKS / "_observation.py")
        saved = dict(os.environ)
        saved_target = mod._GATE_TARGET_BIN
        os.environ["EPR_BIN"] = "/nonexistent/epr"
        os.environ["PATH"] = "/nonexistent"
        mod._GATE_TARGET_BIN = "/nonexistent/gate-target-epr"
        try:
            mod.reset_cache()
            self.assertIsNone(mod.resolve_bin())
            self.assertFalse(mod.available())
            self.assertFalse(mod.emit("x@1", "a/b.md", 1, reason="r"))
        finally:
            os.environ.clear()
            os.environ.update(saved)
            mod._GATE_TARGET_BIN = saved_target
            mod.reset_cache()


class GoldenReportCase(unittest.TestCase):
    """The report's LINE FORMATS and DERIVATION asserted against the real binary, or not at all.

    Nothing here is simulated: the registry under test is the repository's own
    `.claude/epr-meta` pair, the folds are appended by the real `epr flow note --measure`, and
    the assertions read the real `epr flow report --headline`. When the binary is missing, or
    predates `--measure`, the case SKIPS with the reason — a fabricated stub would let a format
    change land green, which is the failure mode this replaces.
    """

    PLAN_SUBJECTS = ("docs/plan-a.md", "docs/plan-b.md", "docs/plan-c.md")
    SEED_SUBJECTS = ("arch/seed-a.md", "arch/seed-b.md")

    @classmethod
    def setUpClass(cls):
        mod = load_module("observation_for_golden", HOOKS / "_observation.py")
        mod.reset_cache()
        cls.binary = mod.resolve_bin()
        if not cls.binary:
            raise unittest.SkipTest("no `epr` binary ($EPR_BIN, the gate target, or PATH)")
        probe = subprocess.run([cls.binary, "flow", "note", "--help"],
                               capture_output=True, text=True, timeout=20)
        if "--measure" not in (probe.stdout + probe.stderr):
            raise unittest.SkipTest(
                f"`{cls.binary}` predates `epr flow note --measure`; nothing to compare against")

    def setUp(self):
        self.root = Path(tempfile.mkdtemp(prefix="golden-report-"))
        self.addCleanup(shutil.rmtree, self.root, True)
        reg = self.root / ".claude" / "epr-meta"
        reg.mkdir(parents=True)
        for name in ("measures.yaml", "policies.yaml"):
            shutil.copy(REPO / ".claude" / "epr-meta" / name, reg / name)
        meta = self.root / ".epr-meta"
        meta.mkdir()
        (meta / "manifest.md").write_text(
            "---\nepr-meta-version: 1\nid: golden-fixture\nroot: true\n"
            "policy-recipe: .claude/epr-meta\n---\n")
        # A subject must be a real path in the tree, and a note is dated by the tree it was
        # written against (never by wall clock), so the fixture needs the files AND a HEAD.
        for rel in self.PLAN_SUBJECTS + self.SEED_SUBJECTS:
            f = self.root / rel
            f.parent.mkdir(parents=True, exist_ok=True)
            f.write_text("fixture\n")
        env = {**os.environ, "GIT_AUTHOR_NAME": "golden", "GIT_AUTHOR_EMAIL": "golden@test",
               "GIT_COMMITTER_NAME": "golden", "GIT_COMMITTER_EMAIL": "golden@test"}
        for cmd in (["git", "init", "-q"], ["git", "add", "-A"],
                    ["git", "commit", "-qm", "golden fixture"]):
            r = subprocess.run(cmd, cwd=self.root, env=env, capture_output=True, text=True)
            self.assertEqual(r.returncode, 0, r.stderr)

    def epr(self, *args) -> subprocess.CompletedProcess:
        return subprocess.run([self.binary, *args, "--root", str(self.root)],
                              capture_output=True, text=True, timeout=60)

    def observe(self, measure, subject, value="1", reason="golden fixture"):
        r = self.epr("flow", "note", "--kind", "observation", "--measure", measure,
                     "--subject", subject, "--value", value, "--reason", reason)
        self.assertEqual(r.returncode, 0, f"{measure} {subject}: {r.stderr}")

    def headline(self) -> str:
        r = self.epr("flow", "report", "--headline")
        self.assertEqual(r.returncode, 0, r.stderr)
        return r.stdout

    def line(self, label: str) -> str:
        out = self.headline()
        found = next((l for l in out.splitlines() if l.strip().startswith(f"{label}:")), None)
        self.assertIsNotNone(found, f"no `{label}:` line in:\n{out}")
        return found

    # every headline slot has a native producer ───────────────────────────────────────────
    def test_all_five_headline_slots_render_without_any_bridged_fold(self):
        """No observation is appended at all, and five slots still render.

        This is what retired the producer bridge: `cleanup` and `scope` derive, `mempalace`
        walks the tree, and `recall` (the retired `memkit` slot's successor, 2026-09-11) reads the
        latest recall-journey fold. A slot that needed a kit reading would report `skipped` here.
        """
        out = self.headline()
        for label in ("recall", "mempalace", "cleanup", "scope", "memory-budget"):
            self.assertTrue(
                any(l.strip().startswith(f"{label}:") for l in out.splitlines()),
                f"no `{label}:` line in:\n{out}")

    def test_a_retired_bound_says_retired_not_skipped(self):
        """`memkit-report-tier-mb@1` is `status: superseded`, and that is NOT the same claim
        as `skipped`. Skipped means nobody measured it; retired means there is nothing left to
        measure — the report tier it bounded was removed at station six round (b).
        Since 2026-09-11 the `memkit` slot is gone from the headline (its position is `recall`),
        so the retirement is read from the JSON `retired` list, never from a headline line and
        never from `outcomes`."""
        r = self.epr("flow", "report", "--bound", "memkit-report-tier-mb-ceiling", "--json")
        self.assertEqual(r.returncode, 0, r.stderr)
        payload = json.loads(r.stdout)
        recipe = payload["recipes"][0] if "recipes" in payload else payload
        retired = [row["bound"] for row in recipe.get("retired", [])]
        self.assertIn("memkit-report-tier-mb-ceiling@1", retired, payload)
        self.assertEqual(recipe.get("outcomes", []), [], "a retired bound is not an outcome")
        self.assertNotIn("skipped", json.dumps(recipe.get("retired")), recipe.get("retired"))
        self.assertNotIn("memkit:", self.headline())

    def test_a_bound_with_no_fold_is_skipped_never_zero(self):
        """A live bound nobody has observed. `sovereignty-landings-ceiling@1` reads folds
        (`derive: count-since-reset`) and this fixture has appended none."""
        r = self.epr("flow", "report", "--bound", "sovereignty-landings-ceiling", "--json")
        self.assertEqual(r.returncode, 0, r.stderr)
        outcomes = [o for rec in json.loads(r.stdout)["recipes"] for o in rec["outcomes"]]
        self.assertEqual(len(outcomes), 1, outcomes)
        self.assertEqual(outcomes[0]["outcome"], "skipped", outcomes[0])
        self.assertNotIn("observed", outcomes[0],
                         "a bound with no fold must carry no observed value, not a zero")

    def test_identical_folds_on_one_subject_do_not_change_the_reading(self):
        for rel in self.PLAN_SUBJECTS:
            self.observe("placement-drift-due@1", rel)
        first = self.headline()
        for rel in self.PLAN_SUBJECTS:
            self.observe("placement-drift-due@1", rel)
        self.assertEqual(self.headline(), first)

    # the derived cleanup bound ───────────────────────────────────────────────────────────
    def seed_distinct_subjects(self, reason="golden fixture"):
        """Five DISTINCT subjects across TWO of the measures the ceiling derives over."""
        for rel in self.PLAN_SUBJECTS:
            self.observe("placement-drift-due@1", rel, reason=reason)
        for rel in self.SEED_SUBJECTS:
            self.observe("map-currency-drift@1", rel, reason=reason)

    def test_cleanup_is_derived_from_distinct_subjects_across_measures(self):
        self.assertIn("skipped", self.line("cleanup"),
                      "no folds yet — a derived bound with nothing to count is skipped")
        self.seed_distinct_subjects()
        got = self.line("cleanup")
        self.assertIn("5", got, f"cleanup did not count 5 distinct subjects: {got}")
        self.assertIn("since the beginning", got, got)
        self.assertNotIn("skipped", got, got)

    def test_a_reset_observation_witnesses_zero(self):
        self.seed_distinct_subjects()
        self.assertIn("5", self.line("cleanup"))
        self.observe("cleanup-pressure-reset@1", ".", reason="golden: drain the accumulator")
        got = self.line("cleanup")
        self.assertRegex(got, r"\b0\b", f"the reset was not witnessed as zero: {got}")
        self.assertIn("since the last reset", got, got)
        self.assertNotIn("skipped", got, got)

    def test_only_new_distinct_subjects_count_after_a_reset(self):
        self.seed_distinct_subjects()
        self.observe("cleanup-pressure-reset@1", ".", reason="golden: drain the accumulator")
        # a byte-identical re-observation mints one CID: it is not a new landing
        self.seed_distinct_subjects()
        self.assertRegex(self.line("cleanup"), r"\b0\b",
                         "identical re-observations must not refill a drained accumulator")
        # a genuinely new landing does count
        fresh = self.root / "docs" / "plan-d.md"
        fresh.write_text("fresh\n")
        subprocess.run(["git", "add", "-A"], cwd=self.root, capture_output=True)
        subprocess.run(["git", "-c", "user.email=g@t", "-c", "user.name=g",
                        "commit", "-qm", "fresh"], cwd=self.root, capture_output=True)
        self.observe("placement-drift-due@1", "docs/plan-d.md", reason="a later landing")
        self.assertIn("1", self.line("cleanup"))

    def test_the_hooks_emitter_is_accepted_by_the_real_verb(self):
        """The argv `_observation.emit()` builds must be one the real binary accepts."""
        mod = load_module("observation_for_golden", HOOKS / "_observation.py")
        saved = dict(os.environ)
        os.environ["EPR_BIN"] = self.binary
        try:
            mod.reset_cache()
            self.assertTrue(mod.available())
            self.assertTrue(mod.emit("placement-drift-due@1", self.PLAN_SUBJECTS[0], 1,
                                     reason="golden: emitter argv", env={"head": "abc1234"},
                                     root=str(self.root)))
        finally:
            os.environ.clear()
            os.environ.update(saved)
            mod.reset_cache()
        got = self.line("cleanup")
        self.assertIn("1", got, f"the emitted fold did not reach the derived bound: {got}")
        self.assertNotIn("skipped", got, got)


if __name__ == "__main__":
    unittest.main()
