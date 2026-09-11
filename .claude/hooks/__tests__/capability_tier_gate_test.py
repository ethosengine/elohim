#!/usr/bin/env python3
"""Task 2.4: `.claude/hooks/capability-tier-gate.py` — destructive git needs a declared tier or
a team check.

Why (operator, 2026-09-11): a Haiku subagent of another session ran `git reset --hard` on the
shared `dev` checkout, dropping three commits and wiping other lanes' uncommitted work. This
hook reads the declared table at `.claude/epr-meta/policies.yaml` row
`destructive-git-requires-tier@1` (patterns, tier-order, tier-floor, unknown-tier, remedy) and
denies a destructive-git Bash command below the declared tier floor.

What is asserted (brief 1.a-f, plus the `rm -rf ./target` negative):

  a. `git reset --hard abc` under `CLAUDE_MODEL=claude-haiku-4-5` -> deny; output names the
     policy id and carries the remedy text.
  b. same command under `CLAUDE_MODEL=claude-opus-5` (at the declared floor) -> silent allow.
  c. no env, no actor claim for this session -> unknown tier -> deny (fail-closed).
  d. no env, but the actor sidecar's LATEST claim for `CLAUDE_SESSION_ID` is
     `agent:implementer@claude-fable-5-1` (above the floor) -> silent allow.
  e. `git reset --soft abc`, `git status`, `git log` -> not in the destructive set -> silent
     allow, no policy/actor read attempted.
  f. a malformed policy row (no `parameters` block) -> `capability-tier-gate: skipped — <reason>`
     on stderr, exit 0 — fail-open, but never silent about it.
  g. `rm -rf ./target` is never caught by the `rm -rf .` pattern (word boundary after the dot);
     `rm -rf .` and `rm -rf ./` both are.

Deny shape mirrors cargo-disk-guard.py's exact convention: one `hookSpecificOutput` JSON object
on stdout (`permissionDecision: deny`), process exit 0 — the harness reads the JSON, not the
process exit code, for a structured PreToolUse deny.

Run: python3 -m unittest discover -s .claude/hooks/__tests__ -p 'capability_tier_gate_test.py'
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HOOKS = Path(__file__).resolve().parent.parent
REPO = HOOKS.parent.parent
HOOK = HOOKS / "capability-tier-gate.py"

# A self-contained fixture registry mirroring the real committed row's shape (patterns,
# tier-order, tier-floor, unknown-tier, remedy) so tests (d) and (f) never depend on the live
# repo registry drifting under them.
FIXTURE_POLICY = """\
epr-meta-policies-version: 1
policies:
  - id: destructive-git-requires-tier
    version: 1
    class: deny
    parameters:
      patterns:
        - "git reset --hard"
        - "git reset --merge"
        - "git checkout -- ."
        - "git checkout ."
        - "git restore ."
        - "git clean -f"
        - "git push --force"
        - "git push -f"
        - "git branch -D"
        - "git branch -f"
        - "git update-ref"
        - "git stash drop"
        - "git stash clear"
        - "git worktree remove --force"
        - "rm -rf ."
      tier-floor: claude-opus-5
      tier-order: [claude-haiku-4-5, claude-sonnet-5, claude-opus-5, claude-fable-5-1, gpt-5.6-sol]
      unknown-tier: deny
      remedy: >
        check with the team first: ask the controller/operator to run this, or claim your tier
        (epr actor claim --as agent:<role>@<model> --session $CLAUDE_SESSION_ID) if you are at or
        above the floor
"""

MALFORMED_POLICY = """\
epr-meta-policies-version: 1
policies:
  - id: destructive-git-requires-tier
    version: 1
    class: deny
    parameters: {}
"""


def _actor_claim_line(claimed: str, session: str) -> str:
    return json.dumps({
        "cid": "bafyreiafakecidfortestonly0000000000000000000000000000000",
        "record": {
            "kind": "claim",
            "claimed": claimed,
            "session": session,
            "claimedAt": "2026-09-11T22:00:00Z",
        },
    })


def _write_project(tmp: Path, policy_text: str, actor_lines: "list[str] | None" = None) -> None:
    (tmp / ".claude" / "epr-meta").mkdir(parents=True, exist_ok=True)
    (tmp / ".claude" / "epr-meta" / "policies.yaml").write_text(policy_text)
    if actor_lines is not None:
        (tmp / ".eprfs" / "status").mkdir(parents=True, exist_ok=True)
        (tmp / ".eprfs" / "status" / "actors.jsonl").write_text(
            "\n".join(actor_lines) + ("\n" if actor_lines else "")
        )


def run_hook(command: str, project_dir: Path, env_extra: "dict | None" = None):
    payload = {"tool_name": "Bash", "tool_input": {"command": command}, "cwd": str(project_dir)}
    env = dict(os.environ)
    for k in ("CLAUDE_MODEL", "ANTHROPIC_MODEL", "CLAUDE_SESSION_ID"):
        env.pop(k, None)
    env["CLAUDE_PROJECT_DIR"] = str(project_dir)
    if env_extra:
        env.update(env_extra)
    return subprocess.run(
        [sys.executable, str(HOOK)],
        input=json.dumps(payload),
        capture_output=True,
        text=True,
        env=env,
        timeout=10,
    )


class CapabilityTierGateCase(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.proj = Path(self._tmp.name)
        _write_project(self.proj, FIXTURE_POLICY, actor_lines=[])

    # ── a: haiku below the floor -> deny, names policy + remedy ──────────────────────────
    def test_a_haiku_below_floor_denied(self):
        r = run_hook("git reset --hard abc", self.proj, {"CLAUDE_MODEL": "claude-haiku-4-5"})
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("destructive-git-requires-tier", r.stdout)
        self.assertIn("permissionDecision", r.stdout)
        self.assertIn("deny", r.stdout)
        self.assertIn("check with the team first", r.stdout)
        out = json.loads(r.stdout)
        self.assertEqual(
            out["hookSpecificOutput"]["permissionDecision"], "deny"
        )

    # ── b: opus at the floor -> silent allow ──────────────────────────────────────────────
    def test_b_opus_at_floor_allowed(self):
        r = run_hook("git reset --hard abc", self.proj, {"CLAUDE_MODEL": "claude-opus-5"})
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(r.stdout.strip(), "")
        self.assertEqual(r.stderr.strip(), "")

    # ── c: no env, no actor claim -> unknown -> deny (fail-closed) ───────────────────────
    def test_c_unknown_tier_denied(self):
        r = run_hook(
            "git reset --hard abc", self.proj, {"CLAUDE_SESSION_ID": "session-with-no-claim"}
        )
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("destructive-git-requires-tier", r.stdout)
        out = json.loads(r.stdout)
        self.assertEqual(out["hookSpecificOutput"]["permissionDecision"], "deny")
        self.assertIn("unknown", out["hookSpecificOutput"]["permissionDecisionReason"])

    # ── d: actor sidecar's latest claim for CLAUDE_SESSION_ID is above the floor ─────────
    def test_d_actor_claim_above_floor_allowed(self):
        session = "sess-d"
        _write_project(
            self.proj,
            FIXTURE_POLICY,
            actor_lines=[
                _actor_claim_line("agent:reader@claude-sonnet-5", session),  # superseded
                _actor_claim_line("agent:implementer@claude-fable-5-1", session),  # latest wins
            ],
        )
        r = run_hook("git reset --hard abc", self.proj, {"CLAUDE_SESSION_ID": session})
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(r.stdout.strip(), "")

    def test_d2_actor_claim_from_a_different_session_never_leaks(self):
        # the LATEST claim is for a different session -> this session still resolves unknown
        _write_project(
            self.proj,
            FIXTURE_POLICY,
            actor_lines=[_actor_claim_line("agent:implementer@claude-fable-5-1", "other-session")],
        )
        r = run_hook("git reset --hard abc", self.proj, {"CLAUDE_SESSION_ID": "sess-d"})
        self.assertEqual(r.returncode, 0, r.stderr)
        out = json.loads(r.stdout)
        self.assertEqual(out["hookSpecificOutput"]["permissionDecision"], "deny")

    # ── e: not in the destructive set -> silent allow, nothing read ──────────────────────
    def test_e_non_destructive_commands_pass_through(self):
        for cmd in ("git reset --soft abc", "git status", "git log"):
            with self.subTest(cmd=cmd):
                r = run_hook(cmd, self.proj, {"CLAUDE_MODEL": "claude-haiku-4-5"})
                self.assertEqual(r.returncode, 0, r.stderr)
                self.assertEqual(r.stdout.strip(), "")
                self.assertEqual(r.stderr.strip(), "")

    def test_e2_commands_with_no_git_or_rm_never_touch_the_registry(self):
        # Point CLAUDE_PROJECT_DIR at a directory with NO .claude/epr-meta at all: a command
        # that never mentions git/rm must still pass silently (nothing to read).
        empty = Path(tempfile.mkdtemp())
        self.addCleanup(lambda: __import__("shutil").rmtree(empty, ignore_errors=True))
        r = run_hook("echo hello", empty, {"CLAUDE_MODEL": "claude-haiku-4-5"})
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(r.stdout.strip(), "")
        self.assertEqual(r.stderr.strip(), "")

    # ── f: malformed row -> skipped on stderr, exit 0, never silent ──────────────────────
    def test_f_malformed_row_skips_loudly(self):
        _write_project(self.proj, MALFORMED_POLICY, actor_lines=[])
        r = run_hook("git reset --hard abc", self.proj, {"CLAUDE_MODEL": "claude-haiku-4-5"})
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(r.stdout.strip(), "")
        self.assertTrue(
            r.stderr.strip().startswith("capability-tier-gate: skipped —"), r.stderr
        )

    def test_f2_missing_registry_skips_loudly(self):
        empty = Path(tempfile.mkdtemp())
        self.addCleanup(lambda: __import__("shutil").rmtree(empty, ignore_errors=True))
        r = run_hook("git reset --hard abc", empty, {"CLAUDE_MODEL": "claude-haiku-4-5"})
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(r.stdout.strip(), "")
        self.assertTrue(
            r.stderr.strip().startswith("capability-tier-gate: skipped —"), r.stderr
        )

    # ── g: the trailing-dot word-boundary precision ───────────────────────────────────────
    def test_g_rm_rf_target_not_caught(self):
        r = run_hook("rm -rf ./target", self.proj, {"CLAUDE_MODEL": "claude-haiku-4-5"})
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(r.stdout.strip(), "")

    def test_g2_rm_rf_dot_and_dot_slash_are_caught(self):
        for cmd in ("rm -rf .", "rm -rf ./"):
            with self.subTest(cmd=cmd):
                r = run_hook(cmd, self.proj, {"CLAUDE_MODEL": "claude-haiku-4-5"})
                self.assertEqual(r.returncode, 0, r.stderr)
                out = json.loads(r.stdout)
                self.assertEqual(out["hookSpecificOutput"]["permissionDecision"], "deny")

    def test_g3_checkout_dot_boundary(self):
        r = run_hook("git checkout -- .github", self.proj, {"CLAUDE_MODEL": "claude-haiku-4-5"})
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(r.stdout.strip(), "", "a '.' pattern must not match '.github'")

    # ── the real committed row also works end to end ──────────────────────────────────────
    def test_h_real_repo_registry_denies_haiku(self):
        r = run_hook("git reset --hard abc", REPO, {"CLAUDE_MODEL": "claude-haiku-4-5"})
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("destructive-git-requires-tier", r.stdout)

    def test_h2_real_repo_registry_allows_opus(self):
        r = run_hook("git reset --hard abc", REPO, {"CLAUDE_MODEL": "claude-opus-5"})
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(r.stdout.strip(), "")


if __name__ == "__main__":
    unittest.main()
