#!/usr/bin/env python3
"""The sovereignty guard's POST hook keeps its budget; its Family-2 shadow probe only abstains.

Plan task C9 (Lane C), rulings R-C4 (amended) and R-C8. Two detached children of
`sovereignty-guard-signal.py`, each spawned with `Popen(start_new_session=True)` (the surfacing
hook's pattern) so the author never waits on either:

  * `--classify <rel> <session> <ts>` — the NATIVE classification. The parent classifies in
    Python and appends the ledger row with `classification_cid: null, source: "pending"`; the
    child runs `epr govern --new --content-stdin` over the landed bytes and rewrites THAT row's
    `classification_cid` with `source: "native"`, or `"python-degraded"` when the evaluator does
    not run.
  * `--probe <rel> <session> [<tool>]` — the Family-2 shadow probe. Spawned only for a `.md`
    Write/Edit whose net-new bytes reach the atom's `probe_min_net_new_bytes`, that no keyword
    matched, and that is not testimony (the ontology atom's `testimony_exempt`). The child runs
    `flow memory index fold --max-files 1 --scope <rel>` -> `flow memory recall open --session
    frame-probe-<sid>` -> ONE `flow memory recall search --provider semantic` scoped to the landed
    file's directory, keeps only candidates whose path is the landed file, and on a score at or
    above `cosine_floor_permille / 1000` appends an abstain row and folds
    `frame-probe-abstain@1`. It never prints; it gives up at 30 s (ruling R-C10: detached, so
    the author never waits; measured under load the whole chain took 11.3 s). The session's
    recall open is paid once: a later probe in the same hook session finds the session's
    continuation on disk and goes straight to the search.

The ledger row's `classification_cid` is a WHOLE-DOCUMENT classification (`--new` over the landed
bytes) while `net_new`/`phrases` are the edit's delta, so the row says so:
`classification_scope: "document"` (ruling R-C14, review W1). Every pending row carries a random
`nonce` the classify child is handed, and the child rewrites the row by that nonce — two landings
of one path in one second each get their own classification (review W4).

Every leg runs a stub `epr` (EPR_BIN) on a temporary project dir; the frame atoms are read from
this repository (the `_lib` loader's fallback).

Run: python3 -m unittest discover -s .claude/hooks/__tests__ -p 'frame_probe_test.py'
"""

from __future__ import annotations

import contextlib
import importlib.util
import io
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
import unittest
import uuid
from pathlib import Path

HOOKS = Path(__file__).resolve().parent.parent
REPO = HOOKS.parent.parent
HOOK = HOOKS / "sovereignty-guard-signal.py"
LEDGER_REL = ".claude/data/sovereignty-guard.jsonl"
SOV_REF = "epr:validator-sovereignty-ontology-guard"

sys.path.insert(0, str(REPO / ".claude" / "scripts"))
from _lib import frame_atoms  # noqa: E402

_ATOM, FRAME_REF = frame_atoms.load_frames(REPO)[SOV_REF]
SIGNAL = _ATOM["recall_signal"]
FLOOR = SIGNAL["cosine_floor_permille"] / 1000
MIN_BYTES = SIGNAL["probe_min_net_new_bytes"]
QUERY = " / ".join(SIGNAL["phrases"])
CLASSIFICATION = "bafyreifhbla6a66gg7u34dbpodxcp4h6pcuqxtxv2jkvsynwfjgnicbpxi"

# The stub. Every invocation appends its argv to $STUB_LOG.
#   govern --json ... --content-stdin   GOVERN_VERB=1 answers one frame-judged verdict naming
#                                        $GOVERN_FRAME_REF / $GOVERN_CID after $GOVERN_SLEEP s;
#                                        anything else exits 2 ("did not run").
#   flow memory index fold               sleeps $FOLD_SLEEP.
#   flow memory recall open              writes <root>/.eprfs/status/recall/<session>/
#                                        continuation.json (as the real verb does), answers
#                                        {"operation": "open"}.
#   flow memory recall search            sleeps $SEARCH_SLEEP, answers $CANDIDATES under
#                                        retrieval.candidates, each carrying fold_lag $LAG.
#   flow / flow note --help              advertise `--measure` (the fold emitter's probe).
#   flow note --measure ...              accepted.
STUB = '''#!/usr/bin/env python3
import json, os, sys, time
argv = sys.argv[1:]
with open(os.environ["STUB_LOG"], "a") as fh:
    fh.write(json.dumps(argv) + "\\n")
if argv[:1] == ["govern"]:
    sys.stdin.read()
    time.sleep(float(os.environ.get("GOVERN_SLEEP", "0")))
    if os.environ.get("GOVERN_VERB") != "1":
        sys.exit(2)
    evidence = {"frameRef": os.environ.get("GOVERN_FRAME_REF", ""),
                "classificationCid": os.environ.get("GOVERN_CID", ""), "verdict": "abstain"}
    print(json.dumps({"decision": "permit", "winningClass": "dispatch",
                      "ruleId": "sovereignty-ontology-guard",
                      "verdicts": [{"class": "dispatch", "ruleId": "sovereignty-ontology-guard",
                                    "reason": "stub", "evidence": evidence}]}))
    sys.exit(0)
if argv[:4] == ["flow", "memory", "index", "fold"]:
    time.sleep(float(os.environ.get("FOLD_SLEEP", "0")))
    print("index fold (stub)")
    sys.exit(0)
if argv[:4] == ["flow", "memory", "recall", "open"]:
    # like the real verb: the session's state lives at .eprfs/status/recall/<session>/
    root = argv[argv.index("--root") + 1]
    session = argv[argv.index("--session") + 1]
    state = os.path.join(root, ".eprfs", "status", "recall", session)
    os.makedirs(state, exist_ok=True)
    with open(os.path.join(state, "continuation.json"), "w") as fh:
        fh.write(json.dumps({"session": session}))
    print(json.dumps({"operation": "open"}))
    sys.exit(0)
if argv[:4] == ["flow", "memory", "recall", "search"]:
    time.sleep(float(os.environ.get("SEARCH_SLEEP", "0")))
    candidates = json.loads(os.environ.get("CANDIDATES", "[]"))
    for candidate in candidates:
        candidate["fold_lag"] = int(os.environ.get("LAG", "0"))
    print(json.dumps({"operation": "search", "retrieval": {"candidates": candidates}}))
    sys.exit(0)
if argv[:2] == ["flow", "note"] and "--help" in argv:
    print("usage: epr flow note --measure <id@version> --subject <path> --value <n>")
    sys.exit(0)
if argv[:2] == ["flow", "note"]:
    sys.exit(0)
if argv == ["flow"] or argv[:1] == ["flow"] and len(argv) == 1:
    print("usage: epr flow <note --measure <id@version>>")
    sys.exit(0)
sys.exit(2)
'''


def candidate(path: str, score: float) -> dict:
    return {
        "path": path,
        "score": score,
        "producer": "semantic",
        "method": "bafyreigdsxgzho6gcbnfilr2ry5itsejmtaixvpf44ej6wgqccgxy6rn6e",
        "best_section": {"title": "fixture", "lines": "1:9"},
    }


def silent_prose(size: int = MIN_BYTES + 120) -> str:
    """Keyword-silent prose of at least `size` bytes — no phrase from the atom appears."""
    line = "Each household keeps the ledger of what it tends and answers for it to its peers.\n"
    return line * (size // len(line) + 1)


def load_hook():
    spec = importlib.util.spec_from_file_location(f"sov_signal_{uuid.uuid4().hex[:6]}", HOOK)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class FrameProbeCase(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = Path(tempfile.mkdtemp(prefix="frame-probe-"))
        self.project = self.tmp / "project"
        (self.project / ".claude" / "data").mkdir(parents=True)
        self.stub = self.tmp / "epr"
        self.stub.write_text(STUB)
        self.stub.chmod(0o755)
        self.log = self.tmp / "stub.log"
        self.probe_tmp = self.tmp / "probe-tmp"
        self.probe_tmp.mkdir()
        self.session = f"fp-{uuid.uuid4().hex[:10]}"

    def tearDown(self) -> None:
        time.sleep(0.2)  # let a detached child finish its last write before the dir goes
        shutil.rmtree(self.tmp, ignore_errors=True)

    # ── fixtures ────────────────────────────────────────────────────────────────────────
    def env(self, **extra: str) -> dict:
        env = dict(os.environ)
        env.update(CLAUDE_PROJECT_DIR=str(self.project), EPR_BIN=str(self.stub),
                   STUB_LOG=str(self.log), TMPDIR=str(self.probe_tmp))
        env.update(extra)
        return env

    def write_doc(self, rel: str, text: str) -> Path:
        doc = self.project / rel
        doc.parent.mkdir(parents=True, exist_ok=True)
        doc.write_text(text)
        return doc

    def run_parent(self, rel: str, text: str, tool: str = "Write", **extra: str):
        doc = self.write_doc(rel, text)
        payload = {"tool_name": tool, "session_id": self.session,
                   "tool_input": {"file_path": str(doc)}}
        began = time.monotonic()
        done = subprocess.run([sys.executable, str(HOOK)], input=json.dumps(payload),
                              capture_output=True, text=True, env=self.env(**extra), timeout=30)
        return done, time.monotonic() - began

    def run_child(self, *args: str, stdin: str = "", **extra: str):
        began = time.monotonic()
        done = subprocess.run([sys.executable, str(HOOK), *args], input=stdin,
                              capture_output=True, text=True, env=self.env(**extra), timeout=60)
        return done, time.monotonic() - began

    def calls(self) -> list[list[str]]:
        if not self.log.exists():
            return []
        return [json.loads(line) for line in self.log.read_text().splitlines() if line.strip()]

    def rows(self) -> list[dict]:
        led = self.project / LEDGER_REL
        if not led.is_file():
            return []
        return [json.loads(line) for line in led.read_text().splitlines() if line.strip()]

    def wait_until(self, predicate, seconds: float = 10.0) -> bool:
        deadline = time.time() + seconds
        while time.time() < deadline:
            if predicate():
                return True
            time.sleep(0.05)
        return predicate()

    def recall_calls(self) -> list[list[str]]:
        return [c for c in self.calls() if c[:3] == ["flow", "memory", "recall"]
                or c[:4] == ["flow", "memory", "index", "fold"]]

    def notes(self) -> list[list[str]]:
        return [c for c in self.calls() if c[:2] == ["flow", "note"] and "--help" not in c]

    def parent_inprocess(self, rel: str, text: str, tool: str = "Write") -> tuple[list, str]:
        """Run the parent in-process with the spawner captured — which children it WOULD start."""
        doc = self.write_doc(rel, text)
        hook = load_hook()
        spawned: list = []
        hook.spawn_child = lambda repo, args, content=None: spawned.append(list(args))
        payload = {"tool_name": tool, "session_id": self.session,
                   "tool_input": {"file_path": str(doc)}}
        saved = dict(os.environ)
        os.environ.update(self.env())
        out = io.StringIO()
        real_stdin, sys.stdin = sys.stdin, io.StringIO(json.dumps(payload))
        try:
            with contextlib.redirect_stdout(out):
                hook.main([])
        finally:
            sys.stdin = real_stdin
            os.environ.clear()
            os.environ.update(saved)
        return spawned, out.getvalue()

    # ── R-C8: the native classification leaves the hook's critical path ─────────────────
    def test_hook_returns_under_budget_with_pending_row_and_spawns_child(self) -> None:
        done, elapsed = self.run_parent("note.md", "The learner is self-sovereign here.\n",
                                        GOVERN_VERB="1", GOVERN_SLEEP="3",
                                        GOVERN_FRAME_REF=FRAME_REF, GOVERN_CID=CLASSIFICATION)
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertLess(elapsed, 2.0, "the PostToolUse budget is 2 s; the hook never waits on govern")
        rows = self.rows()
        self.assertEqual(len(rows), 1, rows)
        self.assertEqual(rows[0]["source"], "pending")
        self.assertIsNone(rows[0]["classification_cid"])
        self.assertEqual(rows[0]["frame_ref"], FRAME_REF)
        self.assertIn("classification pending", done.stdout)
        # the detached child carries the classification to completion after the hook exited
        self.assertTrue(self.wait_until(lambda: self.rows()[0]["source"] != "pending"),
                        self.rows())
        self.assertEqual(self.rows()[0]["source"], "native")
        self.assertEqual(self.rows()[0]["classification_cid"], CLASSIFICATION)

    def test_child_rewrites_row_with_native_classification_cid(self) -> None:
        text = "The learner is self-sovereign here.\n"
        self.write_doc("note.md", text)
        led = self.project / LEDGER_REL
        other = {"ts": "2026-09-24T00:00:00+00:00", "path": "other.md", "tool": "Write",
                 "net_new": 1, "phrases": ["self-sovereign"], "frame_ref": FRAME_REF,
                 "classification_cid": None, "classification_scope": "document",
                 "source": "pending", "verdict": "abstain", "nonce": "0a0a0a0a0a0a0a0a"}
        mine = dict(other, ts="2026-09-24T00:00:01+00:00", path="note.md",
                    nonce="1b1b1b1b1b1b1b1b")
        led.write_text(json.dumps(other) + "\n" + json.dumps(mine) + "\n")
        done, _ = self.run_child("--classify", "note.md", "sid-1", mine["ts"], mine["nonce"],
                                 stdin=text,
                                 GOVERN_VERB="1", GOVERN_FRAME_REF=FRAME_REF,
                                 GOVERN_CID=CLASSIFICATION)
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(done.stdout, "")
        first, second = self.rows()
        self.assertEqual(first, other, "a row the child was not spawned for is untouched")
        self.assertEqual(second["classification_cid"], CLASSIFICATION)
        self.assertEqual(second["source"], "native")
        self.assertEqual({k: v for k, v in second.items()
                          if k not in ("classification_cid", "source")},
                         {k: v for k, v in mine.items()
                          if k not in ("classification_cid", "source")})
        govern = [c for c in self.calls() if c[:1] == ["govern"]]
        self.assertEqual(len(govern), 1, self.calls())
        self.assertIn("--new", govern[0])
        self.assertIn("--content-stdin", govern[0])
        self.assertEqual(govern[0][govern[0].index("--path") + 1], "note.md")
        self.assertEqual(govern[0][govern[0].index("--session") + 1], "sid-1")

    def test_child_leaves_python_degraded_when_binary_absent(self) -> None:
        led = self.project / LEDGER_REL
        mine = {"ts": "2026-09-24T00:00:01+00:00", "path": "note.md", "tool": "Write",
                "net_new": 1, "phrases": ["self-sovereign"], "frame_ref": FRAME_REF,
                "classification_cid": None, "classification_scope": "document",
                "source": "pending", "verdict": "abstain", "nonce": "2c2c2c2c2c2c2c2c"}
        led.write_text(json.dumps(mine) + "\n")
        hook = load_hook()
        hook.epr_client.resolve_binary = lambda: None
        saved = dict(os.environ)
        os.environ.update(self.env())
        try:
            hook.classify_child(["note.md", "sid-1", mine["ts"], mine["nonce"]],
                                io.StringIO("The learner is self-sovereign here.\n"))
        finally:
            os.environ.clear()
            os.environ.update(saved)
        (row,) = self.rows()
        self.assertIsNone(row["classification_cid"])
        self.assertEqual(row["source"], "python-degraded")
        self.assertEqual(self.calls(), [], "no binary, nothing was run")

    # ── C9: when the probe is spawned ──────────────────────────────────────────────────
    def test_probe_not_spawned_when_keyword_hit(self) -> None:
        text = silent_prose() + "The learner is self-sovereign here.\n"
        spawned, _ = self.parent_inprocess("docs/note.md", text)
        self.assertEqual([a[0] for a in spawned], ["--classify"], spawned)

    def test_probe_spawned_for_a_keyword_silent_write_past_the_byte_floor(self) -> None:
        spawned, out = self.parent_inprocess("docs/note.md", silent_prose())
        self.assertEqual(spawned, [["--probe", "docs/note.md", self.session, "Write"]])
        self.assertEqual(out, "")
        small, _ = self.parent_inprocess("docs/small.md", silent_prose(40)[:MIN_BYTES - 1])
        self.assertEqual(small, [], "under probe_min_net_new_bytes nothing is probed")
        other, _ = self.parent_inprocess("docs/note.txt", silent_prose())
        self.assertEqual(other, [], "only .md is probed")

    def test_probe_not_spawned_under_public_observer_or_testimony_true(self) -> None:
        exempt = frame_atoms.load_ontology(REPO)["testimony_exempt"]
        prefix = exempt["path_prefixes"][0]
        observed, _ = self.parent_inprocess(prefix + "a-witness.md", silent_prose())
        self.assertEqual(observed, [], "the public observer's testimony is never probed")
        key = exempt["frontmatter_key"]
        testimony = f"---\ntitle: witness\n{key}: true\n---\n\n" + silent_prose()
        declared, _ = self.parent_inprocess("docs/witness.md", testimony)
        self.assertEqual(declared, [], f"`{key}: true` frontmatter is testimony")
        not_testimony = f"---\ntitle: witness\n{key}: false\n---\n\n" + silent_prose()
        probed, _ = self.parent_inprocess("docs/plain.md", not_testimony)
        self.assertEqual([a[0] for a in probed], ["--probe"])

    # ── C9: what the probe child does ──────────────────────────────────────────────────
    def test_probe_runs_fold_then_open_then_one_search_scoped_to_dir(self) -> None:
        self.write_doc("docs/deep/note.md", silent_prose())
        done, _ = self.run_child("--probe", "docs/deep/note.md", "sid-7", "Write",
                                 CANDIDATES=json.dumps([candidate("docs/deep/note.md", 0.1)]))
        self.assertEqual(done.returncode, 0, done.stderr)
        calls = self.recall_calls()
        self.assertEqual([c[:4] for c in calls],
                         [["flow", "memory", "index", "fold"],
                          ["flow", "memory", "recall", "open"],
                          ["flow", "memory", "recall", "search"]], calls)
        fold, opened, searched = calls
        self.assertEqual(fold[fold.index("--max-files") + 1], "1")
        self.assertEqual(fold[fold.index("--scope") + 1], "docs/deep/note.md",
                         "the fold takes the landed file itself, not whichever file is behind")
        session = "frame-probe-sid-7"
        self.assertEqual(opened[opened.index("--session") + 1], session)
        self.assertEqual(opened[opened.index("--need") + 1], QUERY)
        self.assertEqual(searched[searched.index("--session") + 1], session)
        self.assertEqual(searched[searched.index("--provider") + 1], "semantic")
        self.assertEqual(searched[searched.index("--query") + 1], QUERY)
        self.assertEqual(searched[searched.index("--search-scope") + 1], "docs/deep")
        self.assertIn("--json", searched)

    def test_hit_at_or_above_floor_writes_abstain_row_and_folds(self) -> None:
        self.write_doc("docs/note.md", silent_prose())
        found = [candidate("docs/elsewhere.md", 0.95), candidate("docs/note.md", FLOOR)]
        done, _ = self.run_child("--probe", "docs/note.md", "sid-8", "Edit",
                                 CANDIDATES=json.dumps(found), LAG="2")
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(done.stdout, "")
        (row,) = self.rows()
        self.assertEqual(row["path"], "docs/note.md")
        self.assertEqual(row["tool"], "Edit")
        self.assertEqual(row["verdict"], "abstain")
        self.assertEqual(row["frame_ref"], FRAME_REF)
        self.assertIsNone(row["classification_cid"])
        self.assertEqual(row["phrases"], [])
        probe = row["probe"]
        self.assertEqual(set(probe), {"cosine", "floor", "verdict", "producer", "method",
                                      "fold_lag"})
        self.assertEqual(probe["verdict"], "abstain")
        self.assertAlmostEqual(probe["cosine"], FLOOR)
        self.assertAlmostEqual(probe["floor"], FLOOR)
        self.assertEqual(probe["producer"], "semantic")
        self.assertTrue(probe["method"].startswith("bafy"))
        self.assertEqual(probe["fold_lag"], 2)
        (note,) = self.notes()
        self.assertEqual(note[note.index("--measure") + 1], "frame-probe-abstain@1")
        self.assertEqual(note[note.index("--subject") + 1], "docs/note.md")
        self.assertEqual(note[note.index("--value") + 1], "1")

    def test_below_floor_writes_nothing(self) -> None:
        self.write_doc("docs/note.md", silent_prose())
        found = [candidate("docs/elsewhere.md", 0.95), candidate("docs/note.md", FLOOR - 0.01)]
        done, _ = self.run_child("--probe", "docs/note.md", "sid-9", "Write",
                                 CANDIDATES=json.dumps(found))
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(self.rows(), [])
        self.assertEqual(self.notes(), [])
        self.assertEqual(len([c for c in self.calls() if c[:4] == ["flow", "memory", "recall",
                                                                    "search"]]), 1)

    def test_author_sees_nothing(self) -> None:
        found = [candidate("docs/note.md", 0.9)]
        done, elapsed = self.run_parent("docs/note.md", silent_prose(),
                                        CANDIDATES=json.dumps(found))
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(done.stdout, "", "the probe never reaches the author's context")
        self.assertEqual(done.stderr, "")
        self.assertLess(elapsed, 2.0)
        # ... and yet it ran, detached, and abstained into the ledger
        self.assertTrue(self.wait_until(lambda: len(self.rows()) == 1 and self.notes()),
                        (self.rows(), self.calls()))
        self.assertEqual(self.rows()[0]["probe"]["verdict"], "abstain")

    def test_child_honours_30s_deadline(self) -> None:
        # Ruling R-C10: 8 s was measured short under load (fold 0.9-3.1 s + open 4.3-5.9 s +
        # search 2.3 s; 11.3 s end to end). The child is detached, so the author never waits.
        self.assertEqual(load_hook().PROBE_DEADLINE_S, 30.0)
        self.write_doc("docs/note.md", silent_prose())
        found = [candidate("docs/note.md", 0.9)]
        done, elapsed = self.run_child("--probe", "docs/note.md", "sid-10", "Write",
                                       CANDIDATES=json.dumps(found), SEARCH_SLEEP="60")
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertLess(elapsed, 31.5, "the probe gives up at its 30 s deadline")
        self.assertGreater(elapsed, 29.0, "the search had the budget's remainder, not less")
        self.assertEqual(self.rows(), [])
        self.assertEqual(self.notes(), [])


    # ── review M3 (ruling R-C10): one recall open per hook session ─────────────────────────
    def test_m3_second_probe_in_a_session_skips_open(self) -> None:
        self.write_doc("docs/note.md", silent_prose())
        found = json.dumps([candidate("docs/note.md", 0.1)])
        self.run_child("--probe", "docs/note.md", "sid-m3", "Write", CANDIDATES=found)
        state = self.project / ".eprfs/status/recall/frame-probe-sid-m3/continuation.json"
        self.assertTrue(state.is_file(), "the first probe opened the session")
        first = [c[:4] for c in self.recall_calls()]
        self.assertEqual(first, [["flow", "memory", "index", "fold"],
                                 ["flow", "memory", "recall", "open"],
                                 ["flow", "memory", "recall", "search"]])
        self.log.unlink()
        self.run_child("--probe", "docs/note.md", "sid-m3", "Edit", CANDIDATES=found)
        second = self.recall_calls()
        self.assertEqual([c[:4] for c in second],
                         [["flow", "memory", "index", "fold"],
                          ["flow", "memory", "recall", "search"]],
                         "the session's continuation is on disk: the open is not paid again")
        search = second[-1]
        self.assertEqual(search[search.index("--session") + 1], "frame-probe-sid-m3")
        # A different hook session opens its own.
        self.log.unlink()
        self.run_child("--probe", "docs/note.md", "sid-other", "Write", CANDIDATES=found)
        self.assertIn(["flow", "memory", "recall", "open"],
                      [c[:4] for c in self.recall_calls()])
        # A session directory with no continuation (an open that failed before hydrating) is
        # not a session: the open is paid.
        self.log.unlink()
        broken = self.project / ".eprfs/status/recall/frame-probe-sid-broken"
        broken.mkdir(parents=True)
        (broken / "continuation.json").write_text("")
        self.run_child("--probe", "docs/note.md", "sid-broken", "Write", CANDIDATES=found)
        self.assertIn(["flow", "memory", "recall", "open"],
                      [c[:4] for c in self.recall_calls()])

    # ── review W1 (ruling R-C14): the row says its classification covers the document ─────────
    def test_w1_row_declares_whole_document_classification_scope(self) -> None:
        self.parent_inprocess("docs/note.md", "The learner is self-sovereign here.\n")
        (row,) = self.rows()
        self.assertEqual(row["classification_scope"], "document")
        self.assertEqual(row["source"], "pending")

    # ── review W4 (ruling R-C14): pending rows are rewritten by nonce ───────────────────────
    def test_w4_two_landings_in_one_second_rewrite_their_own_rows(self) -> None:
        # The parent hands the row's own nonce to its child.
        spawned, _ = self.parent_inprocess("docs/note.md", "The learner is self-sovereign here.\n")
        (row,) = self.rows()
        self.assertRegex(row["nonce"], r"^[0-9a-f]{16}$")
        self.assertEqual(spawned[0][0], "--classify")
        self.assertEqual(spawned[0][-1], row["nonce"], spawned)
        again, _ = self.parent_inprocess("docs/note.md", "The learner is self-sovereign here.\n")
        self.assertNotEqual(self.rows()[1]["nonce"], row["nonce"], "a nonce per landing")

        # Two landings of one path in the same second: (path, ts) cannot tell them apart. The
        # children finish in the OPPOSITE order to the landings; each must still fill its own.
        led = self.project / LEDGER_REL
        base = {"ts": "2026-09-24T00:00:05+00:00", "path": "note.md", "tool": "Edit",
                "net_new": 1, "phrases": ["self-sovereign"], "frame_ref": FRAME_REF,
                "classification_cid": None, "classification_scope": "document",
                "source": "pending", "verdict": "abstain"}
        first = dict(base, nonce="aaaaaaaaaaaaaaaa")
        second = dict(base, nonce="bbbbbbbbbbbbbbbb")
        led.write_text(json.dumps(first) + "\n" + json.dumps(second) + "\n")
        cid_first = "bafyreiaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        cid_second = "bafyreibbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
        for landing, cid in ((first, cid_first), (second, cid_second))[::-1]:
            done, _ = self.run_child("--classify", "note.md", "sid-w4", landing["ts"],
                                     landing["nonce"], stdin="The learner is self-sovereign.\n",
                                     GOVERN_VERB="1", GOVERN_FRAME_REF=FRAME_REF, GOVERN_CID=cid)
            self.assertEqual(done.returncode, 0, done.stderr)
        one, two = self.rows()
        self.assertEqual((one["nonce"], one["classification_cid"], one["source"]),
                         (first["nonce"], cid_first, "native"))
        self.assertEqual((two["nonce"], two["classification_cid"], two["source"]),
                         (second["nonce"], cid_second, "native"))
        # A child with no nonce, or one that names no row, rewrites nothing.
        led.write_text(json.dumps(first) + "\n")
        self.run_child("--classify", "note.md", "sid-w4", first["ts"], stdin="x\n",
                       GOVERN_VERB="1", GOVERN_FRAME_REF=FRAME_REF, GOVERN_CID=cid_first)
        self.run_child("--classify", "note.md", "sid-w4", first["ts"], "cccccccccccccccc",
                       stdin="x\n", GOVERN_VERB="1", GOVERN_FRAME_REF=FRAME_REF,
                       GOVERN_CID=cid_first)
        self.assertEqual(self.rows(), [first])


if __name__ == "__main__":
    unittest.main()
