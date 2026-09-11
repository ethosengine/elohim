#!/usr/bin/env python3
"""Task 2.2: the SessionStart headline and the per-turn run-plane line are projections of
`epr flow memory recall open --purpose bootstrap` at lenses `minimal` and `simple`.

Station two of the governed-discovery plan (2026-09-11): `load-project-context.py`'s
`get_habits_status` (a bespoke `habits-status.py --headline` re-scan) and `run-projection.py`'s
whole habits.yaml / flows.jsonl / commitments-stock derivation + its private cache are RETIRED.
Both hooks become thin renderers of ONE native recall session — declared in
`.claude/hooks/.epr-meta` (rule `bootstrapping-head-is-recall-open`) — never a second orientation.

What is asserted:

  1. FUNCTION-LEVEL, stubbed `epr` or a monkeypatched `resolve_bin`: a missing binary, a
     refusing binary, and a run past budget all produce exactly one `bootstrap: skipped —
     <reason>` line — honest absence, never a fallback renderer. Also: the exact argv shape
     each hook shells out with (`--purpose bootstrap`, the lens, `--session bootstrap-<id>`),
     and the session-id derivation (payload `session_id` wins; absent that, both hooks derive
     the SAME deterministic hash of project_dir + today's date, so one session label survives
     across the SessionStart and per-turn calls).
  2. GOLDEN, against the REAL gate binary (skipped when unavailable): the two properties the
     brief's own failing test names verbatim — the headline block is the `minimal` lens under
     1,600 bytes carrying `recipe bafk…`, `lens bafk…`, `top red:` and exactly one
     `  epr flow memory recall ` select line; the run-plane block is the `simple` lens and
     never re-derives (`re-derived this turn from habits.yaml` — the retired hook's own banner
     — must not appear). Golden tests use a session id unique per test run (never the bare
     `input="{}"` fallback) so a same-day re-run of this file never resumes a prior test's
     session and silently drops the `top red:` orientation line that a resumed `open` omits.

Run: EPR_BIN=/tmp/eprfs-gate-target/debug/epr python3 -m unittest discover \
       -s .claude/hooks/__tests__ -p 'bootstrap_projection_test.py'
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
import uuid
from datetime import date
from pathlib import Path

HOOKS = Path(__file__).resolve().parent.parent
REPO = HOOKS.parent.parent

# A controllable `epr` stub for the function-level cases. Behavior selected by env:
#   REFUSE=1   -> non-zero exit, a one-line stderr reason
#   SLOW=1     -> sleeps past whatever timeout the caller gave it
#   STUB_OUTPUT (env) -> stdout verbatim, default a plausible multi-line `open` rendering
# Every invocation is appended to $STUB_LOG as one JSON argv line.
STUB = '''#!/usr/bin/env python3
import json, os, sys, time
argv = sys.argv[1:]
with open(os.environ["STUB_LOG"], "a") as fh:
    fh.write(json.dumps(argv) + "\\n")
if os.environ.get("SLOW") == "1":
    time.sleep(float(os.environ.get("SLOW_SECONDS", "20")))
if os.environ.get("REFUSE") == "1":
    sys.stderr.write("epr flow: refused — fixture\\n")
    sys.exit(2)
default_out = (
    "Intent: fixture\\n"
    "Worthwhile finish: fixture\\n"
    "lens: minimal \\u00b7 cid bafkreicfixture\\n"
    "recipe bafkreiffixture \\u00b7 lens bafkreicfixture \\u00b7 selection: x \\u00b7 "
    "omissions: 0 \\u00b7 receipts: 0\\n"
    "Bootstrap: top red: fixture-habit \\u2014 the check\\n"
    "Continuation: 0 finding(s)\\n"
    "\\n"
    "Linked choices:\\n"
    "1. Inspect a -> b\\n"
    "  epr flow memory recall select --session x --edge 1\\n"
    "8th line never printed\\n"
)
sys.stdout.write(os.environ.get("STUB_OUTPUT", default_out))
'''


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, str(path))
    mod = importlib.util.module_from_spec(spec)
    sys.modules[name] = mod
    spec.loader.exec_module(mod)
    return mod


class _StubFixture(unittest.TestCase):
    """Shared plumbing for the function-level stub cases."""

    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix="bootstrap-proj-"))
        self.addCleanup(shutil.rmtree, self.tmp, True)
        self.project = self.tmp / "project"
        (self.project / ".claude" / "hooks").mkdir(parents=True)
        shutil.copy(HOOKS / "_observation.py", self.project / ".claude" / "hooks")
        self.log = self.tmp / "stub.log"
        self.stub = self.tmp / "epr"
        self.stub.write_text(STUB)
        self.stub.chmod(0o755)
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

    def run_hook(self, hook: str, payload: dict, argv: list | None = None,
                 **envextra) -> subprocess.CompletedProcess:
        return subprocess.run(
            [sys.executable, str(HOOKS / hook)] + (argv or []),
            input=json.dumps(payload), capture_output=True, text=True,
            env=self.env(**envextra), timeout=30,
        )

    def calls(self) -> list[list[str]]:
        if not self.log.exists():
            return []
        return [json.loads(line) for line in self.log.read_text().splitlines() if line.strip()]


# ── load-project-context.py (SessionStart headline) ─────────────────────────────────────────
class HeadlineStubCase(_StubFixture):
    def test_success_carries_the_minimal_lens_under_bootstrap(self):
        r = self.run_hook("load-project-context.py", {})
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("BOOTSTRAP:", r.stdout)
        block = r.stdout.split("BOOTSTRAP:", 1)[1]
        self.assertIn("recipe bafk", block)
        self.assertIn("top red:", block)
        argv = [c for c in self.calls() if c[:4] == ["flow", "memory", "recall", "open"]]
        self.assertEqual(len(argv), 1, self.calls())
        call = argv[0]
        self.assertIn("--purpose", call)
        self.assertEqual(call[call.index("--purpose") + 1], "bootstrap")
        self.assertIn("--lens", call)
        self.assertEqual(call[call.index("--lens") + 1], "minimal")
        self.assertIn("--session", call)

    def test_a_refusing_binary_prints_the_skipped_line_never_a_fallback(self):
        r = self.run_hook("load-project-context.py", {}, REFUSE="1")
        self.assertEqual(r.returncode, 0, r.stderr)
        block = r.stdout.split("BOOTSTRAP:", 1)[1]
        self.assertIn("bootstrap: skipped —", block)
        self.assertIn("fixture", block)
        # honest absence, never the retired habits-status re-derivation
        self.assertNotIn("WIP fence", block)
        self.assertNotIn("top red:", block)

    def test_a_run_past_budget_prints_the_skipped_line(self):
        mod = load_module("load_project_context_for_timeout", HOOKS / "load-project-context.py")
        mod.RECALL_TIMEOUT_S = 0.2
        saved = dict(os.environ)
        os.environ.update(self.env(SLOW="1", SLOW_SECONDS="5"))
        try:
            out = mod._epr_bootstrap_block(str(self.project), "timeout-fixture")
        finally:
            os.environ.clear()
            os.environ.update(saved)
        self.assertIn("bootstrap: skipped —", out)
        self.assertIn("budget", out)

    def test_missing_binary_prints_the_skipped_line(self):
        mod = load_module("load_project_context_for_missing_bin", HOOKS / "load-project-context.py")
        mod._observation_module = lambda project_dir: None  # simulates resolve_bin() == None
        out = mod._epr_bootstrap_block(str(self.project), "missing-fixture")
        self.assertEqual(out, "bootstrap: skipped — no epr binary resolved ($EPR_BIN, gate target, PATH)")

    def test_session_id_prefers_the_payload(self):
        mod = load_module("load_project_context_for_session_id", HOOKS / "load-project-context.py")
        got = mod._bootstrap_session_id({"session_id": "abc-123"}, str(self.project))
        self.assertEqual(got, "abc-123")

    def test_session_id_falls_back_to_a_hash_of_project_dir_and_date(self):
        mod = load_module("load_project_context_for_session_hash", HOOKS / "load-project-context.py")
        got = mod._bootstrap_session_id({}, str(self.project))
        expected = hashlib.sha256(
            f"{self.project}:{date.today().isoformat()}".encode()).hexdigest()[:16]
        self.assertEqual(got, expected)


# ── run-projection.py (per-turn run-plane) ───────────────────────────────────────────────────
class RunPlaneStubCase(_StubFixture):
    def test_success_uses_the_simple_lens_and_caps_at_six_lines(self):
        r = self.run_hook("run-projection.py", {}, argv=["--event", "prompt"])
        self.assertEqual(r.returncode, 0, r.stderr)
        lines = r.stdout.splitlines()
        self.assertLessEqual(len(lines), 6, r.stdout)
        self.assertNotIn("8th line never printed", r.stdout)
        argv = [c for c in self.calls() if c[:4] == ["flow", "memory", "recall", "open"]]
        self.assertEqual(len(argv), 1, self.calls())
        call = argv[0]
        self.assertEqual(call[call.index("--lens") + 1], "simple")
        self.assertEqual(call[call.index("--purpose") + 1], "bootstrap")

    def test_a_refusing_binary_prints_the_skipped_line_never_a_fallback(self):
        r = self.run_hook("run-projection.py", {}, argv=["--event", "prompt"], REFUSE="1")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("bootstrap: skipped —", r.stdout)
        self.assertNotIn("WIP fence", r.stdout)
        self.assertNotIn("re-derived this turn from habits.yaml", r.stdout)

    def test_no_event_flag_still_emits_the_block(self):
        """The brief's own failing test calls this hook with no --event at all."""
        r = self.run_hook("run-projection.py", {})
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("recipe bafk", r.stdout)

    def test_a_run_past_budget_prints_the_skipped_line(self):
        mod = load_module("run_projection_for_timeout", HOOKS / "run-projection.py")
        mod.TIMEOUT_S = 0.2
        saved = dict(os.environ)
        os.environ.update(self.env(SLOW="1", SLOW_SECONDS="5"))
        try:
            lines = mod.run_plane_lines(self.project, {})
        finally:
            os.environ.clear()
            os.environ.update(saved)
        self.assertEqual(len(lines), 1)
        self.assertIn("bootstrap: skipped —", lines[0])
        self.assertIn("budget", lines[0])

    def test_missing_binary_prints_the_skipped_line(self):
        mod = load_module("run_projection_for_missing_bin", HOOKS / "run-projection.py")
        mod.resolve_bin = lambda: None
        lines = mod.run_plane_lines(self.project, {})
        self.assertEqual(lines, ["bootstrap: skipped — no epr binary resolved "
                                 "($EPR_BIN, gate target, PATH)"])

    def test_session_id_prefers_the_payload(self):
        mod = load_module("run_projection_for_session_id", HOOKS / "run-projection.py")
        got = mod.bootstrap_session_id({"session_id": "abc-123"}, self.project)
        self.assertEqual(got, "abc-123")

    def test_session_id_falls_back_to_the_same_hash_load_project_context_uses(self):
        """The fallback formula must MATCH load-project-context.py's — the whole point of a
        shared session label across the SessionStart and per-turn calls."""
        run_mod = load_module("run_projection_for_hash_parity", HOOKS / "run-projection.py")
        got = run_mod.bootstrap_session_id({}, self.project)
        lpc_mod = load_module("load_project_context_for_hash_parity",
                              HOOKS / "load-project-context.py")
        expected = lpc_mod._bootstrap_session_id({}, str(self.project))
        self.assertEqual(got, expected)


# ── golden: the real gate binary, the real worktree ──────────────────────────────────────────
class BootstrapProjectionGoldenCase(unittest.TestCase):
    """Asserted against the REAL binary and the REAL corpus, or not at all. Every session id
    used here is unique per run so a same-day re-run of this file never resumes a session a
    prior run already opened (a resumed `open` omits the `top red:` orientation line)."""

    @classmethod
    def setUpClass(cls):
        mod = load_module("obs_for_bootstrap_golden", HOOKS / "_observation.py")
        cls.binary = mod.resolve_bin()
        if not cls.binary:
            raise unittest.SkipTest("no `epr` binary ($EPR_BIN, the gate target, or PATH)")
        r = subprocess.run([cls.binary, "flow", "memory", "recall", "--help"],
                           capture_output=True, text=True, timeout=10)
        if "open" not in (r.stdout + r.stderr):
            raise unittest.SkipTest(f"`{cls.binary}` predates `flow memory recall open`")
        cls.tag = uuid.uuid4().hex[:12]

    def env(self, session_id: str) -> dict:
        e = dict(os.environ)
        e["CLAUDE_PROJECT_DIR"] = str(REPO)
        e["EPR_BIN"] = self.binary
        e["_BOOTSTRAP_TEST_SESSION"] = session_id  # not read by the hook; documents intent
        return e

    def _rmsession(self, session_id: str):
        shutil.rmtree(REPO / ".eprfs" / "status" / "recall" / f"bootstrap-{session_id}",
                      ignore_errors=True)

    def test_headline_block_is_the_minimal_lens_of_recall_open(self):
        session_id = f"golden-headline-{self.tag}"
        self.addCleanup(self._rmsession, session_id)
        out = subprocess.run(
            [sys.executable, str(HOOKS / "load-project-context.py")],
            input=json.dumps({"session_id": session_id}), capture_output=True, text=True,
            env=self.env(session_id), timeout=60).stdout
        self.assertIn("BOOTSTRAP:", out)
        block = out.split("BOOTSTRAP:", 1)[1]
        self.assertLess(len(block.encode()), 1600)
        self.assertIn("recipe bafk", block)
        self.assertIn("lens bafk", block)
        self.assertIn("top red:", block)
        self.assertEqual(block.count("\n  epr flow memory recall "), 1)

    def test_run_plane_is_the_simple_lens_and_names_no_second_renderer(self):
        session_id = f"golden-runplane-{self.tag}"
        self.addCleanup(self._rmsession, session_id)
        out = subprocess.run(
            [sys.executable, str(HOOKS / "run-projection.py"), "--event", "prompt"],
            input=json.dumps({"session_id": session_id}), capture_output=True, text=True,
            env=self.env(session_id), timeout=60).stdout
        self.assertIn("lens bafk", out)
        self.assertNotIn("re-derived this turn from habits.yaml", out)

    def test_both_hooks_derive_the_same_session_label_for_the_same_payload_id(self):
        """Continuity: an explicit `session_id` in the payload makes both hooks address the
        SAME recall session, so the per-turn `open` resumes the orientation the SessionStart
        `open` already established rather than starting a second, disconnected one. Observable
        proof: a FRESH `open` carries the `Bootstrap: top red:` line; a RESUMED `open` on the
        same session does not (the native executor's own resume semantics) — so seeing the
        line on the first call and NOT on the second, same-session call is continuity, not a
        coincidence of two independent fresh sessions."""
        session_id = f"golden-parity-{self.tag}"
        self.addCleanup(self._rmsession, session_id)
        session_dir = REPO / ".eprfs" / "status" / "recall" / f"bootstrap-{session_id}"
        self.assertFalse(session_dir.exists(), "a stale fixture session was not cleaned up")

        headline_out = subprocess.run(
            [sys.executable, str(HOOKS / "load-project-context.py")],
            input=json.dumps({"session_id": session_id}), capture_output=True, text=True,
            env=self.env(session_id), timeout=60).stdout
        self.assertIn("top red:", headline_out.split("BOOTSTRAP:", 1)[1])
        self.assertTrue(session_dir.is_dir(),
                        "the SessionStart open did not create the expected session directory")

        run_out = subprocess.run(
            [sys.executable, str(HOOKS / "run-projection.py"), "--event", "prompt"],
            input=json.dumps({"session_id": session_id}), capture_output=True, text=True,
            env=self.env(session_id), timeout=60).stdout
        self.assertIn("lens bafk", run_out)
        self.assertNotIn("top red:", run_out,
                         "the per-turn open did not resume the SessionStart session — it "
                         "looks like a second, independent orientation was derived")


if __name__ == "__main__":
    unittest.main(verbosity=2)
