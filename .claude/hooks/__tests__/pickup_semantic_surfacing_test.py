#!/usr/bin/env python3
"""Pickup surfacing reads the NATIVE semantic provider and keeps its fold converging.

Governed-discovery station 4, task 4.6. `pickup-semantic-surfacing.py` no longer shells out to
`mempalace search`; once per Claude session it opens its own governed recall session
(`surfacing-<session>`), asks `epr flow memory recall search --provider semantic --json`, and
reads `epr flow memory index status --json` for the fold's lag. Properties pinned here:

  1. lag > 0 -> exactly ONE detached `epr flow memory index fold --max-files <contract
     limits.fold_files_per_run>` is spawned (its own session, output appended to
     `.eprfs/status/index/fold-spawn.log`), the hook never waits on it, and exits 0.
  2. a second prompt in the same session spawns nothing (the once-per-session gate).
  3. lag 0 spawns nothing.
  4. the top candidate under the 0.35 cosine floor -> silent.
  5. `epr` missing, or failing -> silent, exit 0 (surfacing never blocks a prompt).
  6. the DEGRADED banner prints only when the lag is past the measure's declared foldLag limit.
  7. the recall session is opened before the search, under `surfacing-<session>`.
  8. (fix round 1) every injected line prints its producer and short method CID, and the fold lag
     when behind; a candidate carrying no method is not surfaced.
  9. (fix round 1) the lag is read from the search answer; `index status` runs only as the
     fallback when the search gave no reading.
 10. (fix round 1) past 1 MiB, the spawn log keeps its last 256 KiB before the next append.

Every leg runs a stub `epr` (EPR_BIN, the resolver's first choice) on a temporary project dir
carrying the live contract and the live semantic-index measure.

Run: python3 -m unittest discover -s .claude/hooks/__tests__ -p '*_test.py'
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
HOOK = HOOKS / "pickup-semantic-surfacing.py"
CONTRACT_REL = ".epr-meta/elohim/algorithms/recall-contract.json"
MEASURE_REL = ".epr-meta/elohim/algorithms/recall-semantic-index.json"

# The stub. Every invocation appends its argv to $STUB_LOG. `index status --json` reports $LAG;
# `recall search` answers $CANDIDATES, each carrying `fold_lag` $SEARCH_LAG (default $LAG; SEARCH_FAIL=1
# makes the search refuse); `index fold` records whether it leads its own session,
# then sleeps $FOLD_SLEEP so a hook that waited on it would be caught by the clock.
# STUB_FAIL=1 makes every command exit 1 with non-JSON output.
STUB = '''#!/usr/bin/env python3
import json, os, sys, time
argv = sys.argv[1:]
with open(os.environ["STUB_LOG"], "a") as fh:
    fh.write(json.dumps(argv) + "\\n")
if os.environ.get("STUB_FAIL") == "1":
    print("epr: refused"); sys.exit(1)
if argv[:4] == ["flow", "memory", "index", "status"]:
    lag = os.environ.get("LAG", "0")
    print(json.dumps({"lag": None if lag == "null" else int(lag), "unreadable": 0}))
elif argv[:4] == ["flow", "memory", "index", "fold"]:
    with open(os.environ["FOLD_MARK"], "a") as fh:
        fh.write(json.dumps({"leader": os.getsid(0) == os.getpid(),
                             "stdin_tty": os.isatty(0)}) + "\\n")
    print("index fold (stub)")
    time.sleep(float(os.environ.get("FOLD_SLEEP", "0")))
elif argv[:4] == ["flow", "memory", "recall", "open"]:
    print(json.dumps({"operation": "open"}))
elif argv[:4] == ["flow", "memory", "recall", "search"]:
    if os.environ.get("SEARCH_FAIL") == "1":
        print("semantic: unavailable"); sys.exit(1)
    lag = os.environ.get("SEARCH_LAG", os.environ.get("LAG", "0"))
    candidates = json.loads(os.environ["CANDIDATES"])
    for candidate in candidates:
        candidate["fold_lag"] = None if lag == "null" else int(lag)
    print(json.dumps({"operation": "search", "retrieval": {"candidates": candidates}}))
else:
    print("unknown", argv, file=sys.stderr); sys.exit(2)
'''

HIT = {
    "path": "genesis/docs/plan.md",
    "score": 0.61,
    "producer": "semantic",
    "method": "bafyreigdsxgzho6gcbnfilr2ry5itsejmtaixvpf44ej6wgqccgxy6rn6e",
    "model": "bafkfixturemodel",
    "fold_lag": 3,
    "best_section": {"title": "Task 4.6: Fold-lag freshness", "lines": "137:147"},
}
LOW = dict(HIT, score=0.2)


def load_hook():
    spec = importlib.util.spec_from_file_location("pickup_semantic_surfacing", HOOK)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class SurfacingTest(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = Path(tempfile.mkdtemp(prefix="surfacing-"))
        self.root = self.tmp / "project"
        for rel in (CONTRACT_REL, MEASURE_REL):
            (self.root / rel).parent.mkdir(parents=True, exist_ok=True)
            shutil.copy(REPO / rel, self.root / rel)
        self.contract = json.loads((REPO / CONTRACT_REL).read_text())
        self.measure = json.loads((REPO / MEASURE_REL).read_text())
        self.stub = self.tmp / "epr"
        self.stub.write_text(STUB)
        self.stub.chmod(0o755)
        self.log = self.tmp / "stub.log"
        self.mark = self.tmp / "fold.mark"
        self.session = f"t46-{uuid.uuid4().hex[:12]}"

    def tearDown(self) -> None:
        for kind in ("prompts", "stash", "surfaced", "firstsearch"):
            with contextlib.suppress(OSError):
                Path(f"/tmp/claude-pickup-{kind}-{self.session}").unlink()
        # Let a detached stub fold finish before its directory goes.
        deadline = time.time() + 10
        while time.time() < deadline and self._fold_calls() and not self.mark.exists():
            time.sleep(0.05)
        shutil.rmtree(self.tmp, ignore_errors=True)

    def env(self, **extra: str) -> dict:
        env = dict(os.environ)
        env.update(
            EPR_BIN=str(self.stub),
            CLAUDE_PROJECT_DIR=str(self.root),
            STUB_LOG=str(self.log),
            FOLD_MARK=str(self.mark),
            CANDIDATES=json.dumps([HIT]),
            LAG="0",
            FOLD_SLEEP="0",
        )
        env.update(extra)
        return env

    def prompt(self, text: str = "where did we leave the fold lag work", **extra: str):
        payload = json.dumps({"session_id": self.session, "prompt": text})
        began = time.monotonic()
        done = subprocess.run(
            [sys.executable, str(HOOK), "--event", "prompt"],
            input=payload, capture_output=True, text=True, env=self.env(**extra), timeout=30,
        )
        return done, time.monotonic() - began

    def calls(self) -> list[list[str]]:
        if not self.log.exists():
            return []
        return [json.loads(line) for line in self.log.read_text().splitlines() if line]

    def _fold_calls(self) -> list[list[str]]:
        return [argv for argv in self.calls() if argv[:4] == ["flow", "memory", "index", "fold"]]

    def wait_for_fold(self) -> list[dict]:
        deadline = time.time() + 10
        while time.time() < deadline and not self.mark.exists():
            time.sleep(0.05)
        return [json.loads(line) for line in self.mark.read_text().splitlines() if line]

    # 1 + 2
    def test_lag_spawns_exactly_one_detached_fold_and_exits_0(self) -> None:
        done, elapsed = self.prompt(LAG="3", FOLD_SLEEP="6")
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertLess(elapsed, 5.0, "the hook must never wait on the fold it spawned")
        self.assertIn("PICKUP SURFACING", done.stdout)
        self.assertIn("Recall hints, not truth", done.stdout)
        self.assertNotIn("DEGRADED", done.stdout, "3 behind is within the declared bound")

        marks = self.wait_for_fold()
        folds = self._fold_calls()
        self.assertEqual(len(folds), 1, folds)
        per_run = str(self.contract["limits"]["fold_files_per_run"])
        argv = folds[0]
        self.assertEqual(argv[argv.index("--max-files") + 1], per_run, argv)
        self.assertEqual(marks, [{"leader": True, "stdin_tty": False}],
                         "the fold leads its own session, detached from the hook's stdin")
        spawn_log = self.root / ".eprfs/status/index/fold-spawn.log"
        deadline = time.time() + 10
        while time.time() < deadline and "index fold (stub)" not in spawn_log.read_text():
            time.sleep(0.05)
        self.assertIn("index fold (stub)", spawn_log.read_text(),
                      "the fold's output is appended to the spawn log")

        # A second prompt in the same session: the once-per-session gate holds.
        again, _ = self.prompt("what's next", LAG="3")
        self.assertEqual(again.returncode, 0)
        self.assertEqual(again.stdout, "")
        self.assertEqual(len(self._fold_calls()), 1)

    # 3
    def test_zero_lag_spawns_nothing(self) -> None:
        done, _ = self.prompt(LAG="0")
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertIn("PICKUP SURFACING", done.stdout)
        self.assertEqual(self._fold_calls(), [])

    # 4
    def test_below_the_cosine_floor_is_silent(self) -> None:
        done, _ = self.prompt(CANDIDATES=json.dumps([LOW]))
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(done.stdout, "")

    # 5
    def test_a_failing_epr_is_silent_and_exits_0(self) -> None:
        done, _ = self.prompt(STUB_FAIL="1", LAG="3")
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(done.stdout, "")
        self.assertEqual(done.stderr, "")

    def test_a_missing_epr_is_silent_and_exits_0(self) -> None:
        hook = load_hook()
        hook._epr_bin = lambda: None
        payload = {"session_id": self.session, "prompt": "where did we leave off"}
        before = os.environ.get("CLAUDE_PROJECT_DIR")
        os.environ["CLAUDE_PROJECT_DIR"] = str(self.root)
        try:
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                hook.handle_prompt(payload)
        finally:
            if before is None:
                os.environ.pop("CLAUDE_PROJECT_DIR", None)
            else:
                os.environ["CLAUDE_PROJECT_DIR"] = before
        self.assertEqual(out.getvalue(), "")
        self.assertFalse(self.log.exists(), "nothing was run")

    # 6
    def test_the_degraded_banner_prints_only_past_the_declared_bound(self) -> None:
        limit = int(self.measure["foldLag"]["limit"])
        within, _ = self.prompt(LAG=str(limit))
        self.assertEqual(within.returncode, 0, within.stderr)
        self.assertIn("PICKUP SURFACING", within.stdout)
        self.assertNotIn("DEGRADED", within.stdout)
        self.assertIn(f"fold {limit} files behind", within.stdout)

        self.session = f"t46-{uuid.uuid4().hex[:12]}"
        past, _ = self.prompt(LAG=str(limit + 1))
        self.assertEqual(past.returncode, 0, past.stderr)
        self.assertIn("DEGRADED", past.stdout)
        self.assertIn(f"{limit + 1} files behind", past.stdout)

    # 7
    def test_the_recall_session_is_opened_before_the_search(self) -> None:
        done, _ = self.prompt("/deliver where did we leave the fold", LAG="0")
        self.assertEqual(done.returncode, 0, done.stderr)
        recall = [argv for argv in self.calls() if argv[:3] == ["flow", "memory", "recall"]]
        self.assertEqual([argv[3] for argv in recall], ["open", "search"], recall)
        session = f"surfacing-{self.session}"
        opened, searched = recall
        self.assertEqual(opened[opened.index("--session") + 1], session)
        self.assertEqual(opened[opened.index("--need") + 1], "where did we leave the fold")
        self.assertEqual(searched[searched.index("--session") + 1], session)
        self.assertEqual(searched[searched.index("--provider") + 1], "semantic")
        self.assertEqual(searched[searched.index("--search-scope") + 1], ".")
        self.assertIn("--json", searched)
        self.assertIn("genesis/docs/plan.md", done.stdout)

    def status_calls(self) -> list[list[str]]:
        return [argv for argv in self.calls() if argv[:4] == ["flow", "memory", "index", "status"]]

    # 8
    def test_every_line_prints_its_producer_and_method(self) -> None:
        done, _ = self.prompt(LAG="3")
        self.assertEqual(done.returncode, 0, done.stderr)
        hit_lines = [line for line in done.stdout.splitlines() if line.strip().startswith("[cosine")]
        self.assertEqual(len(hit_lines), 1, done.stdout)
        self.assertIn("· semantic · method bafyreig…rn6e", hit_lines[0])
        self.assertIn("· fold 3 files behind", hit_lines[0])

        self.session = f"t46-{uuid.uuid4().hex[:12]}"
        current, _ = self.prompt(LAG="0")
        line = [l for l in current.stdout.splitlines() if l.strip().startswith("[cosine")][0]
        self.assertIn("· semantic · method bafyreig…rn6e", line)
        self.assertNotIn("files behind", line, "a current fold adds no lag clause")

    def test_a_candidate_with_no_method_is_not_surfaced(self) -> None:
        unmethodical = {k: v for k, v in HIT.items() if k != "method"}
        done, _ = self.prompt(CANDIDATES=json.dumps([unmethodical]))
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(done.stdout, "")

    # 9
    def test_the_lag_is_read_from_the_search_answer(self) -> None:
        done, _ = self.prompt(LAG="3")
        self.assertEqual(done.returncode, 0, done.stderr)
        self.wait_for_fold()
        self.assertEqual(self.status_calls(), [], "no separate status call when the answer carries the lag")
        self.assertEqual(len(self._fold_calls()), 1)

    def test_status_is_the_fallback_when_the_search_gave_no_reading(self) -> None:
        done, _ = self.prompt(LAG="4", SEARCH_FAIL="1")
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(done.stdout, "")
        self.assertEqual(len(self.status_calls()), 1)
        self.wait_for_fold()
        self.assertEqual(len(self._fold_calls()), 1, "the fallback lag still converges the fold")

    # 10
    def test_the_spawn_log_keeps_its_tail_past_one_mebibyte(self) -> None:
        log = self.root / ".eprfs/status/index/fold-spawn.log"
        log.parent.mkdir(parents=True, exist_ok=True)
        old_line = b"old spawn output line that retention may drop\n"
        body = old_line * ((1 << 20) // len(old_line) + 200)
        body += b"THE-NEWEST-OLD-LINE\n"
        log.write_bytes(body)
        done, _ = self.prompt(LAG="3")
        self.assertEqual(done.returncode, 0, done.stderr)
        self.wait_for_fold()
        text = log.read_bytes()
        self.assertLessEqual(len(text), (256 << 10) + 4096, len(text))
        self.assertIn(b"THE-NEWEST-OLD-LINE", text, "the tail survives")
        self.assertTrue(text.startswith(old_line), "the kept tail starts at a line boundary")
        self.assertIn(b"surfacing spawns index fold", text)


if __name__ == "__main__":
    unittest.main()
