#!/usr/bin/env python3
"""Bootstrap hooks share the P1 envelope identity and the session-pinned recall executor.
Stub tests check rendering and honest refusal. Native proof uses private fixture stores.
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
from unittest.mock import patch
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
if sys.argv[1:3] == ["bin", "status"]:
    print(json.dumps({"resolved": __file__, "resolvedCid": "bafkreifixture"}))
    sys.exit(0)
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
        library = self.project / ".claude" / "scripts" / "_lib"
        library.mkdir(parents=True)
        shutil.copy(HOOKS.parent / "scripts" / "_lib" / "epr_client.py", library)
        shutil.copy(HOOKS.parent / "scripts" / "_lib" / "session.py", library)
        registry = self.project / "genesis" / "agentic"
        registry.mkdir(parents=True)
        shutil.copy(HOOKS.parents[1] / "genesis" / "agentic" / "bin-tools.json", registry / "bin-tools.json")
        (library / "__init__.py").write_text("")
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
            "ELOHIM_SESSION_ID": "fixture-p1",
            "ELOHIM_SESSION_OWNER_PID": str(os.getpid()),
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
        self.assertEqual(out, "bootstrap: skipped — no epr binary resolved ($EPR_BIN or PATH)")

    def test_session_id_uses_p1_and_ignores_the_payload(self):
        obs = load_module("obs_for_lpc_session_id", HOOKS / "_observation.py")
        with patch.dict(os.environ, self.env()):
            got = obs.bootstrap_session_id(str(self.project), {"session_id": "abc-123"})
        self.assertEqual(got, "fixture-p1")

    def test_session_id_without_payload_still_uses_p1(self):
        obs = load_module("obs_for_lpc_session_key", HOOKS / "_observation.py")
        with patch.dict(os.environ, self.env()):
            self.assertEqual(obs.bootstrap_session_id(str(self.project), {}), "fixture-p1")


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
        self.assertEqual(lines, ["bootstrap: skipped — no epr binary resolved ($EPR_BIN or PATH)"])

    def test_session_id_uses_p1_and_ignores_the_payload(self):
        obs = load_module("obs_for_rp_session_id", HOOKS / "_observation.py")
        with patch.dict(os.environ, self.env()):
            got = obs.bootstrap_session_id(self.project, {"session_id": "abc-123"})
        self.assertEqual(got, "fixture-p1")

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


class BootstrapProjectionGoldenCase(unittest.TestCase):
    """Real hooks and executable; all records and pins remain in a disposable fixture."""

    def test_hooks_share_a_pinned_executor_across_rebuild(self):
        binary = os.environ.get("EPR_BIN")
        if not binary or not Path(binary).is_file():
            self.skipTest("set EPR_BIN to the candidate executable")
        fixtures = load_module("envelope_bootstrap_fixture", REPO / "genesis/agentic/session_envelope_test.py")
        with tempfile.TemporaryDirectory(prefix="bootstrap-native-") as base:
            fixture = fixtures.Fixture(base)
            fixture.setup()
            fixture.hooks()


if __name__ == "__main__":
    unittest.main(verbosity=2)
