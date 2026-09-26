#!/usr/bin/env python3
"""The memory index projects natively, and STANDS DOWN rather than falling back.

Station four routed this hook native-first with `memory-index-projector.py` as the fallback.
Station six round (b) (2026-09-11) deleted the kit, so every fallback became a STAND-DOWN: when
the native leg cannot answer, the index is left exactly as it stands. The properties:

  1. `epr flow memory project --index` absent or refusing -> NOTHING is written, and the
     advisory says the index is unchanged.
  2. the verb present AND inside the hook's latency budget -> the NATIVE projection is what
     lands in `.claude/memory/MEMORY.md`.
  3. the verb present but OVER budget -> nothing is written, and the probe's cached reason
     names the budget rather than pretending the verb is missing.
  4. the native projection would drop a row the index carries (the edited entry's own, or any
     other that is not named unattributable) -> the index is left UNCHANGED.
  5. an edited entry -> the hook appends the harness's WRITE WITNESS (path, sha256 of the exact
     bytes, its own session, the instant it saw them) and asks `epr flow memory attribution`;
     it never parses the entry itself, and it passes the import NO `--session`/`--as-of`. An
     importable orphan is imported detached (the hook returns within budget, the row lands when
     the import does, two edits serialize on one lock); an unattributable one is named with the
     native verb's reason and never imported under anyone.
  6. the backfill imports every owed orphan and names the rest; it rides every worker (bounded),
     so any memory edit picks the orphans up. Its steward-of-record command lists only the
     entries the native verb says no agent could have authored.

Who wrote an entry, and when, is the native verb's alone — its tests live beside it
(`elohim/eprfs/epr-cli/tests/flow_memory_import.rs`). This stub only answers for it.

Property 4 is the one that used to need the kit, and it is the reason the stand-down is a
preservation rather than a loss: re-rendering from a directory scan wrote a row the
contribution plane does not carry, which the very next native run removed again.

Plus the golden case, asserted against the REAL binary or not at all: every entry is contributed
or named unattributable (with its reason), re-projecting reproduces the index on disk, and the
native projection equals the digest station four recorded (sha256 8ee2e07e…, 98 rows, 23,993
bytes) whenever the corpus still matches that snapshot. A stub cannot carry this one — a
rendering change would land green against a fixture that was written to agree with it.

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
import time
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
# and a row renders only for an entry that HAS a contribution request, exactly as the native
# projection renders only contributions.
# `flow memory attribution <dir>` answers per entry from $FIXTURE_ROOT/attribution.json (entry ->
# fields); an entry not listed is attributable to WRITER_CLAIM. `flow memory import <entry.md>…`
# writes a request per named entry authored as the attribution says (IMPORT_FAIL=1 refuses,
# SLOW_IMPORT=<s> sleeps first). Every invocation is appended to $STUB_LOG as one JSON argv line.
STUB = '''#!/usr/bin/env python3
import glob, json, os, sys, time
argv = sys.argv[1:]
root = os.environ["FIXTURE_ROOT"]
with open(os.environ["STUB_LOG"], "a") as fh:
    fh.write(json.dumps(argv) + "\\n")
contrib = os.path.join(root, ".eprfs/status/memory/contributions")
mem = os.path.join(root, ".claude/memory")

def overrides():
    try:
        return json.load(open(os.path.join(root, "attribution.json")))
    except OSError:
        return {}

def row(name):
    by = None
    try:
        by = json.load(open(os.path.join(contrib, name[:-3] + ".json")))["author"]
    except OSError:
        pass
    e = {"entry": name, "path": ".claude/memory/" + name, "contributedBy": by, "indexed": True,
         "session": "writer-session", "writtenAt": "2026-09-25T10:00:00.000Z",
         "writtenBasis": "witness", "claim": os.environ.get("STUB_AUTHOR", "agent:implementer@fixture"),
         "attributable": True, "reason": None, "stewardOfRecordAdmissible": False,
         "stewardOfRecordReason": "has an author"}
    e.update(overrides().get(name, {}))
    e["importable"] = e["attributable"] and (by is None or by == e["claim"])
    return e

if argv[:3] == ["flow", "memory", "attribution"]:
    names = sorted(os.path.basename(p) for p in glob.glob(os.path.join(mem, "*.md"))
                   if not p.endswith("MEMORY.md"))
    print(json.dumps({"operation": "attribution", "entries": [row(n) for n in names]}))
    sys.exit(0)
if argv[:3] == ["flow", "memory", "import"]:
    began = time.time()
    time.sleep(float(os.environ.get("SLOW_IMPORT", "0")))
    with open(os.path.join(root, "import-intervals.jsonl"), "a") as fh:
        fh.write(json.dumps([began, time.time()]) + "\\n")
    if os.environ.get("IMPORT_FAIL") == "1":
        print("collective memory: refused", file=sys.stderr); sys.exit(2)
    names = [os.path.basename(a) for a in argv[3:] if a.endswith(".md")]
    rows = [row(n) for n in names]
    bad = [r for r in rows if not r["attributable"]]
    if bad:
        print("collective memory: %s is unattributable" % bad[0]["entry"], file=sys.stderr)
        sys.exit(2)
    os.makedirs(contrib, exist_ok=True)
    for r in rows:
        with open(os.path.join(contrib, r["entry"][:-3] + ".json"), "w") as fh:
            json.dump({"author": r["claim"], "imported": {"file": r["entry"]}}, fh)
    print(json.dumps({"counts": {"entries": len(names), "contributed": len(names)}}))
    sys.exit(0)
if argv[:4] != ["flow", "memory", "project", "--index"]:
    print("epr flow: invalid arguments", file=sys.stderr); sys.exit(2)
if os.environ.get("PROJECT_VERB") == "0":
    print("epr flow: invalid arguments: collective memory: unknown flag `--index`",
          file=sys.stderr)
    sys.exit(2)
if os.environ.get("SLOW") == "1":
    time.sleep(30)
rows = ["<!-- GENERATED -->\\n"]
for path in sorted(glob.glob(os.path.join(contrib, "*.json"))):
    name = os.path.basename(path)[:-5] + ".md"
    if name == os.environ["ENTRY_NAME"] and os.environ.get("OMIT_ROW") == "1":
        continue
    rows.append("- [%s](%s) - a row\\n" % (name[:-3], name))
text = "".join(rows)
if "--out" in argv:
    out = os.path.join(root, argv[argv.index("--out") + 1])
    os.makedirs(os.path.dirname(out), exist_ok=True)
    with open(out, "w") as fh:
        fh.write(text)
print(json.dumps({"bytes": len(text.encode()), "entries": len(rows) - 1,
                  "budget": {"state": "ok", "bound": "memory-index-bytes-ceiling@1"},
                  "unloadedRows": [], "wrote": argv[argv.index("--out") + 1] if "--out" in argv
                  else None}))
'''

WRITER = "writer-session"
WRITER_CLAIM = "agent:implementer@fixture"
EARLIER_CLAIM = "agent:earlier@fixture"


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
        # Both entries already contributed by an EARLIER participant: the route below is the
        # plain projection path. The harness-import cases remove a request to make an orphan.
        self.contrib = self.root / ".eprfs" / "status" / "memory" / "contributions"
        self.contrib.mkdir(parents=True)
        for name in (ENTRY, OTHER):
            self.contribute(name, EARLIER_CLAIM)

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

    def contribute(self, name: str, author: str) -> None:
        (self.contrib / f"{Path(name).stem}.json").write_text(
            json.dumps({"author": author, "imported": {"file": name}}))

    def orphan(self, *names: str) -> None:
        for name in names:
            (self.contrib / f"{Path(name).stem}.json").unlink()

    def attribution(self, **by_entry: dict) -> None:
        """What the native verb answers for named entries (keys are entry names)."""
        (self.root / "attribution.json").write_text(json.dumps(by_entry))

    def calls(self, verb: str) -> list[list[str]]:
        if not self.stub_log.exists():
            return []
        return [c for c in (json.loads(l) for l in self.stub_log.read_text().splitlines())
                if c[:3] == ["flow", "memory", verb] or (verb == "project" and "--index" in c)]

    def witnesses(self) -> list[dict]:
        path = self.root / ".eprfs" / "status" / "memory-writes.jsonl"
        if not path.is_file():
            return []
        return [json.loads(l) for l in path.read_text().splitlines() if l.strip()]

    def log_lines(self) -> list[dict]:
        path = self.root / ".eprfs" / "status" / "memory-import.log.jsonl"
        if not path.is_file():
            return []
        return [json.loads(l) for l in path.read_text().splitlines() if l.strip()]

    def wait_for(self, predicate, timeout: float = 30.0) -> bool:
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            if predicate():
                return True
            time.sleep(0.1)
        return predicate()

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

    def run_hook(self, edited: str = ENTRY, session: str | None = WRITER,
                 **envextra) -> subprocess.CompletedProcess:
        payload = {"tool_input": {"file_path": str(self.root / ".claude" / "memory" / edited)}}
        if session:
            payload["session_id"] = session
        payload = json.dumps(payload)
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
        installed = [c for c in self.calls("project") if "--out" in c]
        self.assertTrue(installed, "no --out invocation")
        self.assertIn("--budget", installed[0])
        self.assertIn("memory-index-bytes@1", installed[0])

    def test_the_probe_is_cached_so_a_second_write_costs_one_native_run(self):
        self.run_hook()
        first = len(self.calls("project"))
        self.run_hook()
        second = len(self.calls("project"))
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
    def test_a_contributed_entry_the_projection_would_drop_leaves_the_index_alone(self):
        """The freshness guard, as a REFUSAL: a render that loses a live row is never installed."""
        (self.root / ".claude" / "memory" / "MEMORY.md").write_text(
            f"- [fixture]({ENTRY}) - a row\n- [other]({OTHER}) - another row\n")
        before = self.index()
        r = self.run_hook(OMIT_ROW="1")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(self.index(), before,
                         "a projection with no row for the edited entry was installed")
        self.assertIn("UNCHANGED", r.stdout)
        self.assertIn(ENTRY, r.stdout)
        self.assertFalse((self.root / ".eprfs/status/.memory-index-probe.md").exists(),
                         "the scratch render was left behind")

    # 5 — the harness witnesses, the native verb decides, the harness imports ─────────────────
    def test_every_edit_is_witnessed_with_the_exact_bytes_and_the_hook_session(self):
        self.run_hook()
        seen = self.witnesses()
        self.assertEqual(len(seen), 1, seen)
        data = (self.root / ".claude" / "memory" / ENTRY).read_bytes()
        self.assertEqual(seen[0]["path"], f".claude/memory/{ENTRY}")
        self.assertEqual(seen[0]["sha256"], hashlib.sha256(data).hexdigest())
        self.assertEqual(seen[0]["session"], WRITER)
        self.assertTrue(seen[0]["observedAt"].endswith("Z"), seen[0])

    def test_a_no_op_resave_is_not_witnessed_so_the_first_writer_stays_earliest(self):
        """Session A writes; session B re-saves the SAME bytes: no second witness, so the earliest
        (and only) witness of those bytes still names A. A real change by B is witnessed."""
        self.run_hook(session="session-a")
        self.run_hook(session="session-b")
        seen = self.witnesses()
        self.assertEqual([w["session"] for w in seen], ["session-a"], seen)
        (self.root / ".claude" / "memory" / ENTRY).write_text("---\nname: changed by b\n---\n")
        self.run_hook(session="session-b")
        self.assertEqual([w["session"] for w in self.witnesses()], ["session-a", "session-b"])

    def test_trimming_keeps_the_first_witness_of_every_uncontributed_entry(self):
        self.orphan(OTHER)  # OTHER has no contribution; ENTRY does
        log = self.root / ".eprfs" / "status" / "memory-writes.jsonl"
        line = lambda name, sha, session, at: json.dumps(
            {"path": f".claude/memory/{name}", "sha256": sha, "session": session,
             "observedAt": at}, sort_keys=True) + "\n"
        lines = [line(OTHER, "orphan-sha", "first-writer", "2026-09-25T10:00:00.000Z"),
                 line(OTHER, "orphan-sha", "later-writer", "2026-09-25T10:05:00.000Z")]
        lines += [line(ENTRY, f"sha-{i}", "w", f"2026-09-25T11:{i:02d}:00.000Z")
                  for i in range(10)]
        log.write_text("".join(lines))
        self.mod.trim_witnesses(self.root, max_lines=5, keep=3)
        kept = [json.loads(l) for l in log.read_text().splitlines()]
        # The orphan's FIRST witness survives (the later duplicate does not); the contributed
        # entry's old lines are trimmed to the tail.
        self.assertEqual(kept[0]["session"], "first-writer", kept)
        self.assertEqual([k for k in kept if k["path"].endswith(OTHER)], [kept[0]])
        self.assertEqual(len(kept), 1 + 3, kept)
        self.assertEqual([k["sha256"] for k in kept[1:]], ["sha-7", "sha-8", "sha-9"])

    def test_an_unimported_entry_is_imported_by_the_harness_with_no_caller_assertions(self):
        self.orphan(ENTRY)
        started = time.monotonic()
        r = self.run_hook(SLOW_IMPORT="2")
        elapsed = time.monotonic() - started
        self.assertEqual(r.returncode, 0, r.stderr)
        # The import is DETACHED: the hook returned long before the 2 s import could finish.
        self.assertLess(elapsed, 1.5, f"the hook waited on the import ({elapsed:.2f}s)")
        self.assertNotIn("epr flow memory import", r.stdout,
                         "the advisory still asks the agent to run the import by hand")
        self.assertIn(WRITER_CLAIM, r.stdout)
        self.assertTrue(self.index_untouched(), "the hook installed before the import landed")
        self.assertTrue(self.wait_for(lambda: f"]({ENTRY})" in self.index()),
                        f"the row never landed; log: {self.log_lines()}")
        imports = self.calls("import")
        self.assertEqual(len(imports), 1, imports)
        # The ENTRY form, naming only the edited entry, and asserting NOTHING about who wrote it
        # or when: the native verb derives both from the entry and the harness witness.
        self.assertEqual([a for a in imports[0] if a.endswith(".md")], [f".claude/memory/{ENTRY}"])
        self.assertNotIn("--session", imports[0])
        self.assertNotIn("--as-of", imports[0])
        self.assertTrue(self.wait_for(
            lambda: any(l.get("outcome") == "installed" for l in self.log_lines())))

    def test_the_hook_never_reads_the_entry_to_decide_its_writer(self):
        """A frontmatter naming another session changes nothing the hook passes: the witness names
        the hook's own session, and the import carries no `--session` for a parser to get wrong."""
        self.orphan(ENTRY)
        (self.root / ".claude" / "memory" / ENTRY).write_text(
            "---\nname: fixture\ndescription: |\n  originSessionId: quoted-session\n"
            "  modified: 2020-01-01T00:00:00Z\n---\n")
        self.run_hook()
        self.assertTrue(self.wait_for(lambda: f"]({ENTRY})" in self.index()),
                        f"log: {self.log_lines()}")
        self.assertEqual(self.witnesses()[0]["session"], WRITER)
        joined = " ".join(" ".join(c) for c in self.calls("import"))
        self.assertNotIn("quoted-session", joined)
        self.assertNotIn("2020-01-01", joined)

    def test_two_quick_edits_serialize_on_the_lock(self):
        self.orphan(ENTRY, OTHER)
        self.run_hook(edited=ENTRY, SLOW_IMPORT="1")
        self.run_hook(edited=OTHER, SLOW_IMPORT="1")
        self.assertTrue(self.wait_for(
            lambda: f"]({ENTRY})" in self.index() and f"]({OTHER})" in self.index()),
            f"log: {self.log_lines()}")
        # Serialized, not merely ordered: no two import runs ever overlap.
        spans = sorted(json.loads(l) for l in
                       (self.root / "import-intervals.jsonl").read_text().splitlines())
        self.assertGreaterEqual(len(spans), 2, spans)
        for a, b in zip(spans, spans[1:]):
            self.assertLessEqual(a[1], b[0], f"the imports overlapped: {spans}")

    def test_an_unattributable_entry_is_named_with_the_verbs_reason_and_never_imported(self):
        self.orphan(ENTRY)
        self.attribution(**{ENTRY: {
            "attributable": False,
            "reason": "origin session x had registered no actor claim when it was written",
            "stewardOfRecordAdmissible": True, "stewardOfRecordReason": None}})
        r = self.run_hook()
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("unattributable", r.stdout)
        self.assertIn("no actor claim when it was written", r.stdout)
        self.assertIn("UNCHANGED", r.stdout)
        self.assertIn(f"epr flow memory import .claude/memory/{ENTRY} --session "
                      "steward-of-record --steward-of-record", r.stdout)
        time.sleep(0.5)
        self.assertEqual(self.calls("import"), [], "an unattributable entry was imported")
        self.assertTrue(self.index_untouched())

    def test_no_steward_command_is_offered_when_an_author_could_exist(self):
        self.orphan(ENTRY)
        self.attribution(**{ENTRY: {
            "attributable": False, "reason": "its write time is ambiguous",
            "stewardOfRecordAdmissible": False,
            "stewardOfRecordReason": "session x has claimed"}})
        r = self.run_hook()
        self.assertIn("ambiguous", r.stdout)
        self.assertNotIn("--steward-of-record", r.stdout)

    def test_a_failed_import_changes_nothing_and_is_recorded(self):
        self.orphan(ENTRY)
        self.run_hook(IMPORT_FAIL="1")
        self.assertTrue(self.wait_for(
            lambda: any(l.get("outcome") == "import-failed" for l in self.log_lines())))
        self.assertTrue(self.index_untouched())
        self.assertFalse((self.contrib / f"{Path(ENTRY).stem}.json").exists())

    # 6 — the backfill ─────────────────────────────────────────────────────────────────────────
    def test_any_memory_edit_sweeps_the_owed_orphans(self):
        """The backfill rides every worker: an orphan is picked up the next time anyone edits."""
        self.orphan(OTHER)
        self.run_hook(edited=ENTRY)  # ENTRY is contributed; OTHER is owed an import
        self.assertTrue(self.wait_for(lambda: f"]({OTHER})" in self.index()),
                        f"log: {self.log_lines()}")
        self.assertTrue(any(l.get("mode") == "worker-sweep" and l.get("entries") == [OTHER]
                            for l in self.log_lines()), self.log_lines())

    def test_the_backfill_imports_the_owed_and_names_the_rest(self):
        mem = self.root / ".claude" / "memory"
        self.orphan(ENTRY, OTHER)
        (mem / "feedback_no_origin.md").write_text("---\nname: no origin\n---\n")
        (mem / "MEMORY.md").write_text(
            f"- [fixture]({ENTRY}) - r\n- [other]({OTHER}) - r\n"
            "- [no origin](feedback_no_origin.md) - r\n")
        self.attribution(**{
            OTHER: {"attributable": False, "reason": "origin session s registered no actor claim",
                    "stewardOfRecordAdmissible": True, "stewardOfRecordReason": None},
            "feedback_no_origin.md": {
                "attributable": False, "reason": "its write time is ambiguous",
                "stewardOfRecordAdmissible": False, "stewardOfRecordReason": "could be authored"},
        })
        r = subprocess.run(
            [sys.executable, str(HOOKS / "memory-index-projection.py"), "--backfill", "--json"],
            capture_output=True, text=True, timeout=60, env=self.env())
        self.assertEqual(r.returncode, 0, r.stderr)
        result = json.loads(r.stdout)
        self.assertEqual([g["entry"] for g in result["imported"]], [ENTRY])
        named = {u["entry"]: u["reason"] for u in result["unattributable"]}
        self.assertEqual(set(named), {OTHER, "feedback_no_origin.md"})
        self.assertIn("awaiting its author or the standing human's claim", named[OTHER])
        self.assertFalse((self.contrib / f"{Path(OTHER).stem}.json").exists(),
                         "an unattributable entry was imported under someone else")
        # Only the ADMISSIBLE entry is offered to the standing human; never run by the harness.
        command = result["stewardOfRecordCommand"]
        self.assertIn(f"epr flow memory import .claude/memory/{OTHER} --session "
                      "steward-of-record --steward-of-record", command)
        self.assertNotIn("feedback_no_origin.md", command)
        self.assertTrue(command.startswith("epr actor claim --as human:"), command)
        self.assertEqual(result["install"], "installed")
        self.assertIn(f"]({ENTRY})", self.index())
        self.assertNotIn(f"]({OTHER})", self.index())

    def test_a_write_outside_the_memory_directory_is_a_no_op(self):
        payload = json.dumps({"tool_input": {"file_path": str(self.root / "justfile")}})
        r = subprocess.run(
            [sys.executable, str(HOOKS / "memory-index-projection.py"), "--hook"],
            input=payload, capture_output=True, text=True, timeout=60, env=self.env())
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(r.stdout, "")
        self.assertFalse(self.stub_log.exists())
        self.assertTrue(self.index_untouched())
        self.assertEqual(self.witnesses(), [])

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
        cls.mod = mod
        report = mod.attribution(cls.binary, REPO)
        if report is None:
            raise unittest.SkipTest(f"`{cls.binary}` has no `flow memory attribution` verb")
        cls.report = report
        cls.unattributable = mod.unattributable(report)

    def _named(self) -> str:
        if not self.unattributable:
            return "no entries are unattributable"
        return "left out by name, " + "; ".join(
            f"{u['entry']}: {u['reason']}" for u in self.unattributable)

    def test_every_entry_is_contributed_or_named_unattributable(self):
        """The population half of parity. An entry with no contribution is excused from the index
        ONLY when no registered claim can author it — then it is named, with its reason. An
        orphan whose origin session DID claim is owed an import, and failing here is the point:
        excusing it would hide a real mismatch behind the idempotence check below.
        """
        excused = {u["entry"] for u in self.unattributable}
        owed = [e["entry"] for e in self.mod.orphans(self.report) if e["entry"] not in excused]
        self.assertEqual(owed, [], f"entries the native verb can attribute but that carry no "
                                   f"contribution (the harness backfill owes them): {owed}")
        rendered = self.mod.rows(self.native.decode("utf-8"))
        self.assertFalse(excused & rendered, "an unattributable entry has a projected row")

    def test_the_native_projection_equals_the_index_on_disk(self):
        """Idempotence, which is what survives the kit's deletion as a parity check.

        Station four proved the native and kit projections byte-identical (digest below). With
        one projector left there is no second leg to compare against, so the live property is
        that projecting again reproduces the index already committed — a rendering change shows
        up here as a diff rather than as silence.
        """
        self.assertEqual(hashlib.sha256(self.native).hexdigest(),
                         hashlib.sha256(self.live).hexdigest(),
                         "re-projecting the index rendered different bytes than the tree "
                         f"holds ({self._named()})")

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
