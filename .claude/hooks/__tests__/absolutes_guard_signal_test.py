#!/usr/bin/env python3
"""The absolutes guard's POST hook: net-new, prose-only, trust-surface-only, and it never blocks.

`absolutes-guard-signal.py` is the sovereignty POST hook's twin for absolute promises. What is
locked here:

  1. a Write that lands an uncited absolute on a trust surface prints one PostToolUse message
     listing each absolute with its line and the two honest repairs, appends one ledger row and
     folds exactly one `absolutes-landings@1` observation;
  2. NET-NEW only — an Edit that rewrites a line which already carried the absolute is silent;
  3. a resolving `(enforced: <path>)` citation exempts the sentence; a citation to a path that
     does not exist does not, and the message says the citation does not resolve;
  4. testimony (blockquote, quoted speech, `testimony: true`) is exempt;
  5. non-prose is never scanned: code comments, identifiers, single-word enum literals, a file
     off the trust surfaces;
  6. the escalation total is READ BACK from `absolutes-landings-ceiling` and the review prompt
     appears only at/over the threshold.

Every leg runs a stub `epr` (EPR_BIN) on a temporary project dir; the frame atoms are read from
this repository (the `_lib` loader's fallback).

Run: python3 -m unittest discover -s .claude/hooks/__tests__ -p 'absolutes_guard_signal_test.py'
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HOOKS = Path(__file__).resolve().parent.parent
HOOK = HOOKS / "absolutes-guard-signal.py"
LEDGER_REL = ".claude/data/absolutes-guard.jsonl"

STUB = '''#!/usr/bin/env python3
import json, os, sys
argv = sys.argv[1:]
with open(os.environ["STUB_LOG"], "a") as fh:
    fh.write(json.dumps(argv) + "\\n")
if argv[:2] == ["flow", "note"] and "--help" in argv:
    print("usage: epr flow note --measure <id@version> --subject <path> --value <n>")
    sys.exit(0)
if argv[:2] == ["flow", "note"]:
    sys.exit(0)
if argv[:1] == ["flow"] and len(argv) == 1:
    print("usage: epr flow <note --measure <id@version>>")
    sys.exit(0)
if argv[:3] == ["flow", "report", "--bound"]:
    count = os.environ.get("REPORT_COUNT")
    if count is None:
        sys.exit(2)
    print(json.dumps({"recipes": [{"outcomes": [{"bound": argv[3] + "@1",
                                                 "contributingFolds": int(count)}]}]}))
    sys.exit(0)
sys.exit(2)
'''


class AbsolutesGuardSignalCase(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix="absolutes-guard-"))
        self.addCleanup(shutil.rmtree, self.tmp, True)
        self.project = self.tmp / "project"
        (self.project / ".claude" / "data").mkdir(parents=True)
        (self.project / "genesis/a2o/features/protocol").mkdir(parents=True)
        (self.project / "genesis/docs/specs").mkdir(parents=True)
        (self.project / "tests").mkdir()
        (self.project / "tests/consent_test.py").write_text("# enforcer\n")
        self.log = self.tmp / "stub.log"
        self.stub = self.tmp / "epr"
        self.stub.write_text(STUB)
        self.stub.chmod(0o755)
        self.probe_tmp = self.tmp / "probe-tmp"
        self.probe_tmp.mkdir()

    def run_hook(self, payload: dict, **extra) -> subprocess.CompletedProcess:
        env = dict(os.environ)
        env.update({"CLAUDE_PROJECT_DIR": str(self.project), "EPR_BIN": str(self.stub),
                    "STUB_LOG": str(self.log), "TMPDIR": str(self.probe_tmp)})
        env.update(extra)
        return subprocess.run([sys.executable, str(HOOK)], input=json.dumps(payload),
                              capture_output=True, text=True, env=env, timeout=60)

    def write(self, rel: str, text: str, **extra) -> subprocess.CompletedProcess:
        path = self.project / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)
        return self.run_hook({"tool_name": "Write", "tool_input": {"file_path": str(path)}},
                             **extra)

    def edit(self, rel: str, before: str, old: str, new: str) -> subprocess.CompletedProcess:
        path = self.project / rel
        path.write_text(before.replace(old, new, 1))
        return self.run_hook({"tool_name": "Edit", "tool_input": {
            "file_path": str(path), "old_string": old, "new_string": new}})

    def notes(self) -> list[list[str]]:
        if not self.log.exists():
            return []
        calls = [json.loads(l) for l in self.log.read_text().splitlines() if l.strip()]
        return [c for c in calls if c[:2] == ["flow", "note"] and "--help" not in c]

    def ledger(self) -> list[dict]:
        led = self.project / LEDGER_REL
        if not led.exists():
            return []
        return [json.loads(l) for l in led.read_text().splitlines() if l.strip()]

    def message(self, r: subprocess.CompletedProcess) -> str:
        self.assertEqual(r.returncode, 0, r.stderr)
        if not r.stdout.strip():
            return ""
        return json.loads(r.stdout)["hookSpecificOutput"]["additionalContext"]

    # 1 ─────────────────────────────────────────────────────────────────────────────────────────
    def test_uncited_absolute_lands_with_line_repairs_ledger_and_fold(self):
        r = self.write("genesis/a2o/features/protocol/device.feature",
                       "Feature: device\n  Scenario: capabilities\n"
                       "    Then I see \"Device capabilities are always your own choice\"\n")
        msg = self.message(r)
        self.assertIn("[absolutes-guard]", msg)
        self.assertIn('L3 "always"', msg)
        self.assertIn("(enforced:", msg)
        self.assertIn("by default", msg)
        rows = self.ledger()
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0]["path"], "genesis/a2o/features/protocol/device.feature")
        self.assertEqual([h["line"] for h in rows[0]["hits"]], [3])
        self.assertIsNone(rows[0]["classification_cid"])
        notes = self.notes()
        self.assertEqual(len(notes), 1, notes)
        self.assertEqual(notes[0][notes[0].index("--measure") + 1], "absolutes-landings@1")
        self.assertNotIn("EVALUATE", msg, "below the ceiling the message does not escalate")

    # 2 ─────────────────────────────────────────────────────────────────────────────────────────
    def test_net_new_only(self):
        before = "# Spec\n\nIt is never sold.\n"
        (self.project / "genesis/docs/specs/s.md").write_text(before)
        r = self.edit("genesis/docs/specs/s.md", before, "It is never sold.", "It is never sold!")
        self.assertEqual(self.message(r), "", "rewriting a line that already carried it is quiet")
        r = self.edit("genesis/docs/specs/s.md", before, "# Spec\n", "# Spec\n\nKept forever.\n")
        self.assertIn('"forever"', self.message(r))
        self.assertEqual(len(self.ledger()), 1)

    # 3 ─────────────────────────────────────────────────────────────────────────────────────────
    def test_resolving_citation_exempts_and_a_dangling_one_is_named(self):
        r = self.write("genesis/docs/specs/c.md",
                       "No one can read it without consent (enforced: tests/consent_test.py).\n")
        self.assertEqual(self.message(r), "")
        r = self.write("genesis/docs/specs/d.md",
                       "No one can read it (enforced: tests/missing_test.py).\n")
        msg = self.message(r)
        self.assertIn('"no one can"', msg)
        self.assertIn("does not resolve", msg)

    # 4 ─────────────────────────────────────────────────────────────────────────────────────────
    def test_testimony_is_exempt(self):
        r = self.write("genesis/docs/specs/t.md",
                       "> I will never forget it.\n\nShe said \"we always help\".\n")
        self.assertEqual(self.message(r), "")
        r = self.write("genesis/docs/specs/u.md", "---\ntestimony: true\n---\n\nNever again.\n")
        self.assertEqual(self.message(r), "")

    # 5 ─────────────────────────────────────────────────────────────────────────────────────────
    def test_non_prose_is_never_scanned(self):
        r = self.write("app/elohim-elements/x/src/el.ts",
                       "// this is never shown\nconst mode = 'always';\n"
                       "console.warn('never reached here');\n")
        self.assertEqual(self.message(r), "")
        r = self.write("app/elohim-elements/x/src/el2.ts",
                       "const copy = { label: 'Always your own choice' };\n")
        self.assertIn('L1 "always"', self.message(r))
        r = self.write("docs/notes.md", "It is always fine.\n")
        self.assertEqual(self.message(r), "", "off the trust surfaces")
        r = self.write("genesis/docs/specs/code.md", "```\nnever\n```\nUse `always` here.\n")
        self.assertEqual(self.message(r), "")
        self.assertEqual(len(self.notes()), 1, "only the UI string literal folded")

    # 6 ─────────────────────────────────────────────────────────────────────────────────────────
    def test_escalation_reads_the_bound(self):
        r = self.write("genesis/docs/specs/e.md", "It is guaranteed.\n", REPORT_COUNT="5")
        msg = self.message(r)
        self.assertIn("EVALUATE", msg)
        self.assertIn("5 landing(s)", msg)
        r = self.write("genesis/docs/specs/f.md", "It is guaranteed.\n", REPORT_COUNT="4")
        self.assertNotIn("EVALUATE", self.message(r))

    def test_garbage_input_never_fails_the_tool(self):
        r = subprocess.run([sys.executable, str(HOOK)], input="not json", capture_output=True,
                           text=True, env={**os.environ, "CLAUDE_PROJECT_DIR": str(self.project)})
        self.assertEqual(r.returncode, 0)
        self.assertEqual(r.stdout, "")

    def test_malformed_payload_types_fail_open(self):
        for payload in [[], {"tool_input": ["invalid"]}, {"tool_input": {"file_path": 3}}]:
            with self.subTest(payload=payload):
                result = self.run_hook(payload)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(result.stdout, "")
                self.assertEqual(result.stderr, "")
        self.assertEqual(self.ledger(), [])


if __name__ == "__main__":
    unittest.main()
