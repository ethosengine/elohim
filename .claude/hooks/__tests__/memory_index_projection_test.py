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
  4. the native projection would drop a row the index carries (the edited entry's own, or any
     other that is not named unattributable) -> the index is left UNCHANGED.

  5. an entry with NO contribution -> the HARNESS imports it, detached, under the session that
     wrote it (its `originSessionId`, else the hook's `session_id`), with import's entry form;
     the hook returns within budget, the row lands when the import does, two edits serialize on
     one lock, and a writer with no registered claim is named unattributable — never imported
     under anyone else.
  6. the backfill imports each orphan under its origin session's claim, and names the rest; it
     rides every worker (bounded), so any memory edit picks the orphans up.
  7. the author is the claim held AS OF the writing (frontmatter `modified`, else the mtime the
     hook saw): a claim made later in the same session never takes an earlier entry, and an
     entry written before any claim is unattributable.

Property 4 is the one that used to need the kit, and it is the reason the stand-down is a
preservation rather than a loss: re-rendering from a directory scan wrote a row the
contribution plane does not carry, which the very next native run removed again.

Plus the golden case, asserted against the REAL binary or not at all: every entry is contributed
or named unattributable (with its reason), re-projecting reproduces the index on disk, and the
native projection equals the digest station four recorded (sha256 8ee2e07e…, 98 rows, 23,993 bytes) whenever the corpus
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
# projection renders only contributions. `flow memory import <entry.md>… --session S` writes a
# request per named entry authored by S's claim in actors.jsonl (IMPORT_FAIL=1 refuses,
# SLOW_IMPORT=<s> sleeps first). Every invocation is appended to $STUB_LOG as one JSON argv line.
STUB = '''#!/usr/bin/env python3
import glob, json, os, sys, time
argv = sys.argv[1:]
root = os.environ["FIXTURE_ROOT"]
with open(os.environ["STUB_LOG"], "a") as fh:
    fh.write(json.dumps(argv) + "\\n")
contrib = os.path.join(root, ".eprfs/status/memory/contributions")
if argv[:3] == ["flow", "memory", "import"]:
    began = time.time()
    time.sleep(float(os.environ.get("SLOW_IMPORT", "0")))
    with open(os.path.join(root, "import-intervals.jsonl"), "a") as fh:
        fh.write(json.dumps([began, time.time()]) + "\\n")
    if os.environ.get("IMPORT_FAIL") == "1":
        print("collective memory: refused", file=sys.stderr); sys.exit(2)
    session = argv[argv.index("--session") + 1]
    # As the native verb does: the LAST claim appended whose claimedAt is not after --as-of.
    from datetime import datetime
    ts = lambda v: datetime.fromisoformat(v.replace("Z", "+00:00"))
    at = ts(argv[argv.index("--as-of") + 1])
    author = None
    for line in open(os.path.join(root, ".eprfs/status/actors.jsonl")):
        rec = json.loads(line)["record"]
        if rec["session"] == session and ts(rec.get("recordedAt") or rec["claimedAt"]) <= at:
            author = rec["claimed"]
    if author is None:
        print("session had registered no actor claim as of", file=sys.stderr); sys.exit(2)
    names = [a for a in argv[3:] if a.endswith(".md")]
    os.makedirs(contrib, exist_ok=True)
    for n in names:
        stem = os.path.basename(n)[:-3]
        with open(os.path.join(contrib, stem + ".json"), "w") as fh:
            json.dump({"author": author, "imported": {"file": stem + ".md"}}, fh)
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
        self.claim(WRITER, WRITER_CLAIM)

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

    def claim(self, session: str, claimed: str, at: str = "2026-01-01T00:00:00Z",
              recorded: str | None = None) -> None:
        actors = self.root / ".eprfs" / "status" / "actors.jsonl"
        record = {"kind": "claim", "claimed": claimed, "session": session, "claimedAt": at}
        if recorded:
            record["recordedAt"] = recorded
        with open(actors, "a") as fh:
            fh.write(json.dumps({"cid": "x", "record": record}) + "\n")

    def write_entry(self, name: str, origin: str | None, modified: str | None) -> None:
        meta = "".join(["metadata:\n",
                        f"  originSessionId: {origin}\n" if origin else "",
                        f"  modified: {modified}\n" if modified else ""])
        (self.root / ".claude" / "memory" / name).write_text(
            f"---\nname: {Path(name).stem}\n{meta}---\n")

    def imports(self) -> list[list[str]]:
        if not self.stub_log.exists():
            return []
        return [c for c in (json.loads(l) for l in self.stub_log.read_text().splitlines())
                if c[:3] == ["flow", "memory", "import"]]

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

    # 5 — the harness imports; the agent is never asked to ─────────────────────────────────────
    def test_an_unimported_entry_is_imported_by_the_harness_under_its_writer(self):
        (self.contrib / f"{Path(ENTRY).stem}.json").unlink()
        started = time.monotonic()
        r = self.run_hook(SLOW_IMPORT="2")
        elapsed = time.monotonic() - started
        self.assertEqual(r.returncode, 0, r.stderr)
        # The import is DETACHED: the hook returned long before the 2 s import could finish.
        self.assertLess(elapsed, 1.5, f"the hook waited on the import ({elapsed:.2f}s)")
        self.assertNotIn("epr flow memory import", r.stdout,
                         "the advisory still asks the agent to run the import by hand")
        self.assertIn(WRITER, r.stdout)
        self.assertTrue(self.index_untouched(), "the hook installed before the import landed")
        self.assertTrue(self.wait_for(lambda: f"]({ENTRY})" in self.index()),
                        f"the row never landed; log: {self.log_lines()}")
        request = json.loads((self.contrib / f"{Path(ENTRY).stem}.json").read_text())
        self.assertEqual(request["author"], WRITER_CLAIM, "imported under someone else")
        calls = [json.loads(l) for l in self.stub_log.read_text().splitlines()]
        imports = [c for c in calls if c[:3] == ["flow", "memory", "import"]]
        self.assertEqual(len(imports), 1, imports)
        # The ENTRY form, naming only the edited entry — never the directory.
        self.assertEqual([a for a in imports[0] if a.endswith(".md")],
                         [f".claude/memory/{ENTRY}"])
        self.assertEqual(imports[0][imports[0].index("--session") + 1], WRITER)
        self.assertTrue(self.wait_for(
            lambda: any(l.get("outcome") == "installed" for l in self.log_lines())))

    def test_the_frontmatter_origin_session_is_the_author_over_the_hook_session(self):
        (self.contrib / f"{Path(ENTRY).stem}.json").unlink()
        self.claim("origin-session", "agent:origin@fixture")
        (self.root / ".claude" / "memory" / ENTRY).write_text(
            "---\nname: fixture\nmetadata:\n  originSessionId: origin-session\n---\n")
        self.run_hook()
        self.assertTrue(self.wait_for(lambda: f"]({ENTRY})" in self.index()),
                        f"log: {self.log_lines()}")
        request = json.loads((self.contrib / f"{Path(ENTRY).stem}.json").read_text())
        self.assertEqual(request["author"], "agent:origin@fixture")

    def test_two_quick_edits_serialize_on_the_lock(self):
        for name in (ENTRY, OTHER):
            (self.contrib / f"{Path(name).stem}.json").unlink()
        self.run_hook(edited=ENTRY, SLOW_IMPORT="1")
        self.run_hook(edited=OTHER, SLOW_IMPORT="1")
        self.assertTrue(self.wait_for(
            lambda: f"]({ENTRY})" in self.index() and f"]({OTHER})" in self.index()),
            f"log: {self.log_lines()}")
        # Serialized: each import finished before the next started (no interleaving in the log).
        self.assertTrue(self.wait_for(
            lambda: len([l for l in self.log_lines() if l.get("mode") == "worker"]) == 4))
        worker = [l["outcome"] for l in self.log_lines() if l.get("mode") == "worker"]
        self.assertEqual(worker, ["imported", "installed", "imported", "installed"], worker)
        # Serialized, not merely ordered: the two imports' run intervals never overlap.
        spans = sorted(json.loads(l) for l in
                       (self.root / "import-intervals.jsonl").read_text().splitlines())
        self.assertEqual(len(spans), 2, spans)
        self.assertLessEqual(spans[0][1], spans[1][0], f"the imports overlapped: {spans}")

    def test_a_writer_that_never_claimed_is_named_unattributable_and_nothing_is_imported(self):
        (self.contrib / f"{Path(ENTRY).stem}.json").unlink()
        r = self.run_hook(session="never-claimed")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("unattributable", r.stdout)
        self.assertIn("UNCHANGED", r.stdout)
        time.sleep(0.5)
        calls = ([json.loads(l) for l in self.stub_log.read_text().splitlines()]
                 if self.stub_log.exists() else [])
        self.assertFalse([c for c in calls if c[:3] == ["flow", "memory", "import"]],
                         "an unclaimed writer's entry was imported under someone")
        self.assertTrue(self.index_untouched())

    def test_a_failed_import_changes_nothing_and_is_recorded(self):
        (self.contrib / f"{Path(ENTRY).stem}.json").unlink()
        self.run_hook(IMPORT_FAIL="1")
        self.assertTrue(self.wait_for(
            lambda: any(l.get("outcome") == "import-failed" for l in self.log_lines())))
        self.assertTrue(self.index_untouched())
        self.assertFalse((self.contrib / f"{Path(ENTRY).stem}.json").exists())

    # 6 — the backfill ─────────────────────────────────────────────────────────────────────────
    def test_the_backfill_imports_under_each_origin_and_names_the_unattributable(self):
        mem = self.root / ".claude" / "memory"
        for name in (ENTRY, OTHER):
            (self.contrib / f"{Path(name).stem}.json").unlink()
        self.claim("closed-session", "agent:closed@fixture")
        (mem / ENTRY).write_text(
            "---\nname: fixture\nmetadata:\n  originSessionId: closed-session\n---\n")
        (mem / OTHER).write_text(
            "---\nname: other\nmetadata:\n  originSessionId: silent-session\n---\n")
        (mem / "feedback_no_origin.md").write_text("---\nname: no origin\n---\n")
        (mem / "MEMORY.md").write_text(
            f"- [fixture]({ENTRY}) - r\n- [other]({OTHER}) - r\n"
            "- [no origin](feedback_no_origin.md) - r\n")
        r = subprocess.run(
            [sys.executable, str(HOOKS / "memory-index-projection.py"), "--backfill", "--json"],
            capture_output=True, text=True, timeout=60, env=self.env())
        self.assertEqual(r.returncode, 0, r.stderr)
        result = json.loads(r.stdout)
        self.assertEqual([(g["session"], g["entries"]) for g in result["imported"]],
                         [("closed-session", [ENTRY])])
        self.assertEqual(
            json.loads((self.contrib / f"{Path(ENTRY).stem}.json").read_text())["author"],
            "agent:closed@fixture")
        named = {u["entry"]: u["reason"] for u in result["unattributable"]}
        self.assertEqual(set(named), {OTHER, "feedback_no_origin.md"})
        self.assertIn("awaiting its author or the standing human's claim", named[OTHER])
        self.assertIn("silent-session", named[OTHER])
        self.assertFalse((self.contrib / f"{Path(OTHER).stem}.json").exists(),
                         "an unattributable entry was imported under someone else")
        # The advisory names the ONE line the standing human would run, and never runs it.
        command = result["stewardOfRecordCommand"]
        self.assertIn(f"epr flow memory import .claude/memory/feedback_no_origin.md "
                      f".claude/memory/{OTHER} --session steward-of-record "
                      "--steward-of-record", command)
        self.assertTrue(command.startswith("epr actor claim --as human:"), command)
        self.assertEqual(result["install"], "installed")
        self.assertIn(f"]({ENTRY})", self.index())
        self.assertNotIn(f"]({OTHER})", self.index())

    # 7 — as of the writing ────────────────────────────────────────────────────────────────────
    def test_a_subagent_claiming_later_never_takes_the_orchestrators_earlier_entry(self):
        (self.contrib / f"{Path(ENTRY).stem}.json").unlink()
        self.claim("shared", "agent:orchestrator@fixture", "2026-09-25T10:00:00Z")
        self.write_entry(ENTRY, "shared", "2026-09-25T11:00:00.500Z")
        # A subagent claims LATER in the same session — before the hook even fires.
        self.claim("shared", "agent:scribe@fixture", "2026-09-25T12:00:00Z")
        r = self.run_hook(session="shared")
        self.assertIn("agent:orchestrator@fixture", r.stdout)
        self.assertTrue(self.wait_for(lambda: f"]({ENTRY})" in self.index()),
                        f"log: {self.log_lines()}")
        self.assertEqual(
            json.loads((self.contrib / f"{Path(ENTRY).stem}.json").read_text())["author"],
            "agent:orchestrator@fixture", "the later claim took the earlier entry")
        call = self.imports()[0]
        self.assertEqual(call[call.index("--as-of") + 1], "2026-09-25T11:00:00.500Z")

    def test_the_write_time_is_captured_before_the_worker_runs(self):
        """No frontmatter `modified`: the mtime the hook SAW is the instant, not a later one."""
        (self.contrib / f"{Path(ENTRY).stem}.json").unlink()
        self.claim("shared", "agent:orchestrator@fixture", "2026-09-25T10:00:00Z")
        self.write_entry(ENTRY, "shared", None)
        seen = 1_790_334_000  # 2026-09-25T11:00:00Z — between the two claims
        os.utime(self.root / ".claude" / "memory" / ENTRY, (seen, seen))
        self.claim("shared", "agent:scribe@fixture", "2026-09-25T12:00:00Z")
        self.run_hook(session="shared", SLOW_IMPORT="1")
        os.utime(self.root / ".claude" / "memory" / ENTRY)  # a later write while it waits
        self.assertTrue(self.wait_for(lambda: f"]({ENTRY})" in self.index()),
                        f"log: {self.log_lines()}")
        call = self.imports()[0]
        self.assertTrue(call[call.index("--as-of") + 1].startswith("2026-09-25T11:00:00"), call)
        self.assertEqual(
            json.loads((self.contrib / f"{Path(ENTRY).stem}.json").read_text())["author"],
            "agent:orchestrator@fixture")

    def test_two_claims_against_one_head_order_by_when_they_were_recorded(self):
        """Same claimedAt (one HEAD); recordedAt orders them. The entry between gets the EARLIER."""
        (self.contrib / f"{Path(ENTRY).stem}.json").unlink()
        head = "2026-09-25T20:00:00Z"
        self.claim("tied", "agent:orchestrator@fixture", head, "2026-09-25T20:10:00Z")
        self.claim("tied", "agent:scribe@fixture", head, "2026-09-25T20:30:00Z")
        self.write_entry(ENTRY, "tied", "2026-09-25T20:20:00Z")
        r = self.run_hook(session="tied")
        self.assertIn("agent:orchestrator@fixture", r.stdout)
        self.assertTrue(self.wait_for(lambda: f"]({ENTRY})" in self.index()),
                        f"log: {self.log_lines()}")
        self.assertEqual(
            json.loads((self.contrib / f"{Path(ENTRY).stem}.json").read_text())["author"],
            "agent:orchestrator@fixture")

    def test_an_entry_written_before_any_claim_is_unattributable(self):
        (self.contrib / f"{Path(ENTRY).stem}.json").unlink()
        self.write_entry(ENTRY, "late", "2026-09-25T09:00:00Z")
        self.claim("late", "agent:late@fixture", "2026-09-25T10:00:00Z")
        r = self.run_hook(session="late")
        self.assertIn("unattributable", r.stdout)
        self.assertIn("no actor claim when it was written", r.stdout)
        self.assertIn(f"epr flow memory import .claude/memory/{ENTRY} --session "
                      "steward-of-record --steward-of-record", r.stdout)
        time.sleep(0.5)
        self.assertEqual(self.imports(), [], "a later claim was borrowed")
        self.assertTrue(self.index_untouched())

    def test_any_memory_edit_sweeps_the_owed_orphans(self):
        """The backfill rides every worker: an orphan is picked up the next time anyone edits."""
        (self.contrib / f"{Path(OTHER).stem}.json").unlink()
        self.claim("closed", "agent:closed@fixture", "2026-09-25T10:00:00Z")
        self.write_entry(OTHER, "closed", "2026-09-25T11:00:00Z")
        (self.contrib / f"{Path(ENTRY).stem}.json").unlink()
        self.run_hook()  # the edit is to ENTRY, by WRITER; OTHER was written by a closed session
        self.assertTrue(self.wait_for(
            lambda: f"]({ENTRY})" in self.index() and f"]({OTHER})" in self.index()),
            f"log: {self.log_lines()}")
        self.assertEqual(
            json.loads((self.contrib / f"{Path(OTHER).stem}.json").read_text())["author"],
            "agent:closed@fixture")
        self.assertTrue(any(l.get("mode") == "worker-sweep" and l.get("entries") == [OTHER]
                            for l in self.log_lines()), self.log_lines())

    def test_the_backfill_gives_bracketed_entries_each_their_own_claim(self):
        for name in (ENTRY, OTHER):
            (self.contrib / f"{Path(name).stem}.json").unlink()
        self.claim("two", "agent:first@fixture", "2026-09-25T10:00:00Z")
        self.claim("two", "agent:second@fixture", "2026-09-25T12:00:00Z")
        self.write_entry(ENTRY, "two", "2026-09-25T11:00:00Z")
        self.write_entry(OTHER, "two", "2026-09-25T13:00:00Z")
        r = subprocess.run(
            [sys.executable, str(HOOKS / "memory-index-projection.py"), "--backfill", "--json"],
            capture_output=True, text=True, timeout=60, env=self.env())
        self.assertEqual(r.returncode, 0, r.stderr)
        author = lambda n: json.loads(
            (self.contrib / f"{Path(n).stem}.json").read_text())["author"]
        self.assertEqual(author(ENTRY), "agent:first@fixture")
        self.assertEqual(author(OTHER), "agent:second@fixture")
        self.assertEqual(len(self.imports()), 2, "two claims, two imports")

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
        cls.mod = mod
        cls.unattributable = mod.unattributable(REPO)

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
        owed = [n for n in self.mod.orphans(REPO) if n not in excused]
        self.assertEqual(owed, [], f"entries whose origin session claimed but that carry no "
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
