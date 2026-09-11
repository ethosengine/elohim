#!/usr/bin/env python3
"""Task 2.2: the SessionStart headline and the per-turn run-plane line are projections of
`epr flow memory recall open --purpose bootstrap` at lenses `minimal` and `simple`.

Station two of the governed-discovery plan (2026-09-11): `load-project-context.py`'s
`get_habits_status` (a bespoke `habits-status.py --headline` re-scan) and `run-projection.py`'s
whole habits.yaml / flows.jsonl / commitments-stock derivation + its private cache are RETIRED.
Both hooks become thin renderers of ONE native recall session — declared in
`.claude/hooks/.epr-meta` (two exact-filename rules,
`bootstrapping-head-is-recall-open-run-projection` and
`bootstrapping-head-is-recall-open-load-project-context`) — never a second orientation.

**Fix round 1 (2026-09-11 review)** changed the shape this file asserts against:

  1. `.claude/hooks/.epr-meta`'s single `write: "*project*.py"` rule ALSO matched
     `memory-index-projection.py` (fnmatch has no alternation; `"project"` is a substring of
     `"projection"`) — split into two exact-filename rules. Asserted at the resolver level in
     this task's own commit-time proof, not re-asserted here (this file has no `.epr-meta`
     fixture of its own).
  2. The session-id fallback formula moved to ONE place — `_observation.bootstrap_session_id
     (project_dir, payload)` — imported by both hooks instead of duplicated. Both hooks now
     use `_observation_module()` (a defensive try/except import) rather than a hard top-level
     `from _observation import resolve_bin`.
  3. `load-project-context.py`'s BOOTSTRAP block moved INSIDE the `hookSpecificOutput.
     additionalContext` JSON wrapper (the documented SessionStart landing path) instead of a
     second plain-text print after it. `run-projection.py --event session` (registered
     `async: true`) is now a NO-OP — nothing derived, nothing printed — since an async hook's
     stdout is not read by anything; `--event prompt` (and no `--event` at all) is unaffected.

What is asserted:

  1. FUNCTION-LEVEL, stubbed `epr` or a monkeypatched `_observation_module`: a missing binary,
     a refusing binary, and a run past budget all produce exactly one `bootstrap: skipped —
     <reason>` line — honest absence, never a fallback renderer. Also: the exact argv shape
     each hook shells out with (`--purpose bootstrap`, the lens, `--session bootstrap-<id>`),
     the session-id derivation (payload `session_id` wins; absent that, both hooks route
     through the SAME shared `_observation.bootstrap_session_id`), and that `--event session`
     is a true no-op for `run-projection.py`.
  2. GOLDEN, against the REAL gate binary (skipped when unavailable): the two properties the
     brief's own failing test names verbatim — the headline block (now read out of the JSON
     wrapper's `additionalContext`) is the `minimal` lens under 1,600 bytes carrying
     `recipe bafk…`, `lens bafk…`, `top red:` and exactly one `  epr flow memory recall `
     select line; the run-plane block is the `simple` lens and never re-derives
     (`re-derived this turn from habits.yaml` — the retired hook's own banner — must not
     appear). Golden tests use a session id unique per test run (never the bare `input="{}"`
     fallback) so a same-day re-run of this file never resumes a prior test's session and
     silently drops the `top red:` orientation line that a resumed `open` omits.

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


def _bootstrap_block(stdout: str) -> str:
    """Extract the BOOTSTRAP section from `load-project-context.py`'s stdout — since fix round
    1 that stdout is ONE JSON `hookSpecificOutput` object, and the block lives inside its
    `additionalContext` string (real newlines only appear after `json.loads` unescapes them;
    splitting the RAW, still-JSON-encoded stdout on "BOOTSTRAP:" would see literal `\\n`
    two-character escapes instead)."""
    ctx = json.loads(stdout)["hookSpecificOutput"]["additionalContext"]
    return ctx.split("BOOTSTRAP:", 1)[1]


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
        block = _bootstrap_block(r.stdout)
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

    def test_the_block_lands_inside_the_hookspecificoutput_json_wrapper(self):
        """Fix round 1, finding 4: SessionStart is synchronous here, so the JSON wrapper IS
        the documented landing path — the block must be reachable by parsing JSON, not by
        splitting the raw process stdout on a plain-text marker."""
        r = self.run_hook("load-project-context.py", {})
        self.assertEqual(r.returncode, 0, r.stderr)
        parsed = json.loads(r.stdout)  # raises if stdout is not ONE valid JSON document
        self.assertEqual(parsed["hookSpecificOutput"]["hookEventName"], "SessionStart")
        self.assertIn("BOOTSTRAP:", parsed["hookSpecificOutput"]["additionalContext"])

    def test_a_refusing_binary_prints_the_skipped_line_never_a_fallback(self):
        r = self.run_hook("load-project-context.py", {}, REFUSE="1")
        self.assertEqual(r.returncode, 0, r.stderr)
        block = _bootstrap_block(r.stdout)
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
            out = mod._epr_bootstrap_block(str(self.project), {"session_id": "timeout-fixture"})
        finally:
            os.environ.clear()
            os.environ.update(saved)
        self.assertIn("bootstrap: skipped —", out)
        self.assertIn("budget", out)

    def test_missing_binary_prints_the_skipped_line(self):
        mod = load_module("load_project_context_for_missing_bin", HOOKS / "load-project-context.py")
        mod._observation_module = lambda project_dir: None  # simulates the module being absent
        out = mod._epr_bootstrap_block(str(self.project), {"session_id": "missing-fixture"})
        self.assertEqual(out, "bootstrap: skipped — no epr binary resolved ($EPR_BIN, gate target, PATH)")

    def test_session_id_prefers_the_payload_via_the_shared_helper(self):
        obs = load_module("obs_for_lpc_session_id", HOOKS / "_observation.py")
        got = obs.bootstrap_session_id(str(self.project), {"session_id": "abc-123"})
        self.assertEqual(got, "abc-123")

    def test_session_id_falls_back_to_a_hash_of_project_dir_and_date(self):
        obs = load_module("obs_for_lpc_session_hash", HOOKS / "_observation.py")
        got = obs.bootstrap_session_id(str(self.project), {})
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

    def test_event_session_is_a_true_no_op(self):
        """Fix round 1, finding 4: `--event session` is registered `async: true` — nothing
        reads its stdout, so this hook must not spend a subprocess call on it."""
        r = self.run_hook("run-projection.py", {}, argv=["--event", "session"])
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(r.stdout, "")
        self.assertEqual(self.calls(), [], "the session event must not shell out at all")

    def test_a_refusing_binary_prints_the_skipped_line_never_a_fallback(self):
        r = self.run_hook("run-projection.py", {}, argv=["--event", "prompt"], REFUSE="1")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("bootstrap: skipped —", r.stdout)
        self.assertNotIn("WIP fence", r.stdout)
        self.assertNotIn("re-derived this turn from habits.yaml", r.stdout)

    def test_no_event_flag_still_emits_the_block(self):
        """The brief's own failing test calls this hook with no --event at all — behaves like
        `prompt`, not `session`."""
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

    def test_missing_observation_module_prints_the_skipped_line(self):
        """Fix round 1, finding 3: the top-level hard import is gone — a missing/broken
        `_observation` module must degrade to the skip line, not raise."""
        mod = load_module("run_projection_for_missing_obs", HOOKS / "run-projection.py")
        mod._observation_module = lambda: None
        lines = mod.run_plane_lines(self.project, {})
        self.assertEqual(lines, ["bootstrap: skipped — no epr binary resolved "
                                 "($EPR_BIN, gate target, PATH)"])

    def test_session_id_prefers_the_payload_via_the_shared_helper(self):
        obs = load_module("obs_for_rp_session_id", HOOKS / "_observation.py")
        got = obs.bootstrap_session_id(self.project, {"session_id": "abc-123"})
        self.assertEqual(got, "abc-123")

    def test_both_hooks_route_to_the_one_shared_bootstrap_session_id(self):
        """Fix round 1, finding 2: the session-id fallback formula used to be duplicated
        verbatim in both hook files. Regression guard — kept per the review's instruction:
        neither hook may keep a local copy of the formula; both must resolve, through their own
        defensive `_observation_module()` accessor, to the identical shared function."""
        run_mod = load_module("run_projection_for_parity", HOOKS / "run-projection.py")
        lpc_mod = load_module("load_project_context_for_parity", HOOKS / "load-project-context.py")
        run_obs = run_mod._observation_module()
        lpc_obs = lpc_mod._observation_module(str(self.project))
        self.assertIsNotNone(run_obs)
        self.assertIsNotNone(lpc_obs)
        self.assertFalse(hasattr(run_mod, "bootstrap_session_id"),
                         "run-projection.py must not keep its own copy of the session-id formula")
        self.assertFalse(hasattr(lpc_mod, "_bootstrap_session_id"),
                         "load-project-context.py must not keep its own copy of the formula")
        got_run = run_obs.bootstrap_session_id(self.project, {})
        got_lpc = lpc_obs.bootstrap_session_id(str(self.project), {})
        self.assertEqual(got_run, got_lpc)


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
        block = _bootstrap_block(out)
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
        self.assertIn("top red:", _bootstrap_block(headline_out))
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
