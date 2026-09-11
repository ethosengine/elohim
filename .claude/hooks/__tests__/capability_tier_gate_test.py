#!/usr/bin/env python3
"""Task 2.4, fix round 1 (adversarial review, measured with 30 payloads): the substring-matching
gate was bypassable. `.claude/hooks/capability-tier-gate.py` now does TOKENISED subcommand
classification — segment the command, unwrap `bash -c`/`sh -c`/`command`/`env`/an absolute path
to git, strip git's GLOBAL options before reading the subcommand, and classify by subcommand +
flags — reading the classification rules from `.claude/epr-meta/policies.yaml` row
`destructive-git-requires-tier@1` (`parameters.rules` + `parameters.rm-force-recursive-targets`).

Reviewer's measured bypass matrix (all of these must now DENY):
  git reset -q --hard | git -C /projects/elohim reset --hard (the incident vector: a hook in a
  worktree must catch a reset aimed at ANY checkout) | git clean -xfd | git checkout HEAD -- . |
  git restore --staged --worktree . | rm -rf /projects/elohim | rm -rf * | rm -rf ..
  Never in the old set at all (must now DENY): git rebase | git filter-branch | checkout -B |
  switch -C | reflog expire | gc --prune=now | branch -f.
  Required ALLOWED: git reset --soft, git status, git log, echo git reset --hard x (prose false
  positive), rm -rf ./target, git clean -n.
  Also asserted: a tampered contentHash pin denies EVERY git/rm candidate (fail-closed, distinct
  from a merely malformed row, which stays `skipped —`); an internal error is spoken
  (`capability-tier-gate: skipped — internal error: <ExceptionType>`), never a bare silent exit;
  `ANTHROPIC_MODEL` is no longer consulted at all; the deny remedy carries no self-claim hint.

Run: python3 -m unittest discover -s .claude/hooks/__tests__ -p 'capability_tier_gate_test.py'
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
REPO = HOOKS.parent.parent
HOOK = HOOKS / "capability-tier-gate.py"

# A self-contained fixture registry mirroring the real committed row's shape (the tokenised
# `rules:` + `rm-force-recursive-targets:` — NOT the retired `patterns:` list) so every test here
# is independent of the live repo registry drifting under it.
FIXTURE_POLICY_BODY = """\
epr-meta-policies-version: 1
policies:
  - id: destructive-git-requires-tier
    version: 1
    class: deny
    parameters:
      rules:
        - { sub: reset, any_flags: ["--hard", "--merge"] }
        - { sub: checkout, any_args: [".", "--", "-B", "-f", "--force"] }
        - { sub: restore, any_args: [".", "--worktree", "--staged"] }
        - { sub: clean, flag_letters: ["f"] }
        - { sub: push, any_flags: ["-f", "--force", "--force-with-lease"] }
        - { sub: branch, any_flags: ["-D", "-f", "--force", "--delete"] }
        - { sub: switch, any_flags: ["-C", "--force-create"] }
        - { sub: rebase }
        - { sub: filter-branch }
        - { sub: reflog, any_args: ["expire", "delete"] }
        - { sub: gc, any_flags: ["--prune"] }
        - { sub: update-ref }
        - { sub: stash, any_args: ["drop", "clear"] }
        - { sub: worktree, any_args: ["remove"], any_flags: ["--force", "-f"] }
      rm-force-recursive-targets: [".", "./", "..", "*", "~", "/"]
      tier-floor: claude-opus-5
      tier-order: [claude-haiku-4-5, claude-sonnet-5, claude-opus-5, claude-fable-5-1, gpt-5.6-sol]
      unknown-tier: deny
      remedy: "check with the team first: ask the controller/operator to run this"
"""

MALFORMED_POLICY = """\
epr-meta-policies-version: 1
policies:
  - id: destructive-git-requires-tier
    version: 1
    class: deny
    parameters: {}
"""


def _content_hash(body_text: str) -> str:
    """Compute the real pin for FIXTURE_POLICY_BODY the same way epr-meta-pin.py does, via the
    shared _lib.epr_meta canonicalization — so the fixture row is a genuinely PINNED row, not a
    hand-typed guess."""
    import yaml  # local import: only needed to build the fixture, not by the hook under test

    sys.path.insert(0, str(REPO / ".claude" / "scripts"))
    from _lib import epr_meta as em  # noqa: E402

    data = yaml.safe_load(body_text)
    row = data["policies"][0]
    return em.policy_content_hash(row)


def _pinned_fixture_policy() -> str:
    h = _content_hash(FIXTURE_POLICY_BODY)
    lines = FIXTURE_POLICY_BODY.splitlines()
    out = []
    for line in lines:
        out.append(line)
        if line.strip() == "version: 1":
            out.append(f"    contentHash: {h}")
    return "\n".join(out) + "\n"


FIXTURE_POLICY = _pinned_fixture_policy()


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
        # A fake "other checkout" root so the `-C <path>` / absolute-rm cases have a real
        # `.git` to find, independent of the live repo.
        self.other_checkout = self.proj / "other-checkout"
        (self.other_checkout / ".git").mkdir(parents=True)

    # ── table-driven: every reviewer-measured bypass must now DENY ───────────────────────────
    BYPASSES_MUST_DENY = [
        "git reset -q --hard",
        "git clean -xfd",
        "git checkout HEAD -- .",
        "git restore --staged --worktree .",
        "rm -rf *",
        "rm -rf ..",
        "rm -rf ~",
        "rm -rf /",
        # never-in-the-old-set-at-all class:
        "git rebase main",
        "git filter-branch --tree-filter x",
        "git checkout -B mybranch",
        "git switch -C mybranch",
        "git reflog expire --all",
        "git gc --prune=now",
        "git branch -f main HEAD~1",
        "git update-ref refs/heads/main HEAD~1",
        "git stash drop",
        "git stash clear",
        # unwrapping
        'bash -c "git reset --hard"',
        "sh -c 'git clean -xfd'",
        "command git reset --hard",
        "env FOO=bar git reset --hard",
        "/usr/bin/git reset --hard",
        # chained segments — the destructive one is not first
        "git status && git reset --hard",
        "git log ; git push --force",
    ]

    def test_bypass_matrix_all_denied(self):
        for cmd in self.BYPASSES_MUST_DENY:
            with self.subTest(cmd=cmd):
                r = run_hook(cmd, self.proj, {"CLAUDE_MODEL": "claude-haiku-4-5"})
                self.assertEqual(r.returncode, 0, r.stderr)
                self.assertTrue(r.stdout.strip(), f"expected a deny for: {cmd}")
                out = json.loads(r.stdout)
                self.assertEqual(
                    out["hookSpecificOutput"]["permissionDecision"], "deny", cmd
                )

    def test_the_incident_vector_reset_aimed_at_another_checkout(self):
        # `git -C <other checkout> reset --hard` — a hook running in ONE worktree must catch a
        # reset aimed at ANY checkout, regardless of the -C target.
        cmd = f"git -C {self.other_checkout} reset --hard"
        r = run_hook(cmd, self.proj, {"CLAUDE_MODEL": "claude-haiku-4-5"})
        self.assertEqual(r.returncode, 0, r.stderr)
        out = json.loads(r.stdout)
        self.assertEqual(out["hookSpecificOutput"]["permissionDecision"], "deny")

    def test_rm_rf_on_an_absolute_checkout_root_denied(self):
        cmd = f"rm -rf {self.other_checkout}"
        r = run_hook(cmd, self.proj, {"CLAUDE_MODEL": "claude-haiku-4-5"})
        self.assertEqual(r.returncode, 0, r.stderr)
        out = json.loads(r.stdout)
        self.assertEqual(out["hookSpecificOutput"]["permissionDecision"], "deny")

    # ── table-driven: required ALLOWED (silent) ──────────────────────────────────────────────
    MUST_ALLOW = [
        "git reset --soft HEAD~1",
        "git status",
        "git log",
        "echo git reset --hard x",
        "rm -rf ./target",
        "git clean -n",
        "git checkout main",  # ordinary branch checkout, no path/force marker
        "git gc",  # routine maintenance, no --prune
        "git reflog show",
        "printf 'git reset --hard'",
        "grep 'git reset --hard' file.txt",
        "cat notes-about-git-reset---hard.txt",
    ]

    def test_allowed_matrix_all_silent(self):
        for cmd in self.MUST_ALLOW:
            with self.subTest(cmd=cmd):
                r = run_hook(cmd, self.proj, {"CLAUDE_MODEL": "claude-haiku-4-5"})
                self.assertEqual(r.returncode, 0, r.stderr)
                self.assertEqual(r.stdout.strip(), "", f"unexpected deny for: {cmd}")
                self.assertEqual(r.stderr.strip(), "", f"unexpected stderr for: {cmd}")

    def test_commands_with_no_git_or_rm_never_touch_the_registry(self):
        empty = Path(tempfile.mkdtemp())
        self.addCleanup(lambda: shutil.rmtree(empty, ignore_errors=True))
        r = run_hook("echo hello", empty, {"CLAUDE_MODEL": "claude-haiku-4-5"})
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(r.stdout.strip(), "")
        self.assertEqual(r.stderr.strip(), "")

    # ── tier resolution ───────────────────────────────────────────────────────────────────────
    def test_haiku_below_floor_denied(self):
        r = run_hook("git reset --hard abc", self.proj, {"CLAUDE_MODEL": "claude-haiku-4-5"})
        self.assertEqual(r.returncode, 0, r.stderr)
        out = json.loads(r.stdout)
        self.assertEqual(out["hookSpecificOutput"]["permissionDecision"], "deny")
        self.assertIn("destructive-git-requires-tier", out["hookSpecificOutput"]["permissionDecisionReason"])
        self.assertIn("check with the team first", out["hookSpecificOutput"]["permissionDecisionReason"])

    def test_remedy_carries_no_self_claim_hint(self):
        r = run_hook("git reset --hard abc", self.proj, {"CLAUDE_MODEL": "claude-haiku-4-5"})
        out = json.loads(r.stdout)
        reason = out["hookSpecificOutput"]["permissionDecisionReason"]
        self.assertNotIn("epr actor claim", reason)
        self.assertNotIn("self-claim", reason)

    def test_opus_at_floor_allowed(self):
        r = run_hook("git reset --hard abc", self.proj, {"CLAUDE_MODEL": "claude-opus-5"})
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(r.stdout.strip(), "")

    def test_unknown_tier_denied(self):
        r = run_hook(
            "git reset --hard abc", self.proj, {"CLAUDE_SESSION_ID": "session-with-no-claim"}
        )
        self.assertEqual(r.returncode, 0, r.stderr)
        out = json.loads(r.stdout)
        self.assertEqual(out["hookSpecificOutput"]["permissionDecision"], "deny")
        self.assertIn("unknown", out["hookSpecificOutput"]["permissionDecisionReason"])

    def test_anthropic_model_env_is_never_consulted(self):
        # ANTHROPIC_MODEL alone (no CLAUDE_MODEL, no actor claim) must resolve to `unknown`,
        # never to the value of ANTHROPIC_MODEL — dropping it closes the blanket-bypass class.
        r = run_hook(
            "git reset --hard abc", self.proj, {"ANTHROPIC_MODEL": "claude-opus-5"}
        )
        self.assertEqual(r.returncode, 0, r.stderr)
        out = json.loads(r.stdout)
        self.assertEqual(out["hookSpecificOutput"]["permissionDecision"], "deny")
        self.assertIn("unknown", out["hookSpecificOutput"]["permissionDecisionReason"])

    def test_actor_claim_above_floor_allowed(self):
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

    def test_actor_claim_from_a_different_session_never_leaks(self):
        _write_project(
            self.proj,
            FIXTURE_POLICY,
            actor_lines=[_actor_claim_line("agent:implementer@claude-fable-5-1", "other-session")],
        )
        r = run_hook("git reset --hard abc", self.proj, {"CLAUDE_SESSION_ID": "sess-d"})
        self.assertEqual(r.returncode, 0, r.stderr)
        out = json.loads(r.stdout)
        self.assertEqual(out["hookSpecificOutput"]["permissionDecision"], "deny")

    # ── contentHash pin verification ─────────────────────────────────────────────────────────
    def test_tampered_row_denies_every_git_rm_candidate(self):
        # A valid-shaped row whose contentHash no longer matches its body — even a NORMALLY
        # allowed command (git status) must now be denied, since a tampered table cannot be
        # trusted to say what ISN'T destructive either.
        tampered = FIXTURE_POLICY.replace(
            "rm-force-recursive-targets: [\".\", \"./\", \"..\", \"*\", \"~\", \"/\"]",
            "rm-force-recursive-targets: [\".\"]",  # body changed AFTER the hash was computed
        )
        _write_project(self.proj, tampered, actor_lines=[])
        for cmd in ("git reset --hard abc", "git status"):
            with self.subTest(cmd=cmd):
                r = run_hook(cmd, self.proj, {"CLAUDE_MODEL": "claude-opus-5"})
                self.assertEqual(r.returncode, 0, r.stderr)
                out = json.loads(r.stdout)
                self.assertEqual(out["hookSpecificOutput"]["permissionDecision"], "deny")
                self.assertIn("fails its pin", out["hookSpecificOutput"]["permissionDecisionReason"])

    def test_pin_verified_row_with_no_content_hash_at_all_denies(self):
        no_pin = FIXTURE_POLICY_BODY  # the unpinned body (no contentHash line injected)
        _write_project(self.proj, no_pin, actor_lines=[])
        r = run_hook("git reset --hard abc", self.proj, {"CLAUDE_MODEL": "claude-opus-5"})
        self.assertEqual(r.returncode, 0, r.stderr)
        out = json.loads(r.stdout)
        self.assertEqual(out["hookSpecificOutput"]["permissionDecision"], "deny")
        self.assertIn("fails its pin", out["hookSpecificOutput"]["permissionDecisionReason"])

    # ── malformed row: honest skip, distinct from a pin failure ──────────────────────────────
    def test_malformed_row_skips_loudly(self):
        _write_project(self.proj, MALFORMED_POLICY, actor_lines=[])
        r = run_hook("git reset --hard abc", self.proj, {"CLAUDE_MODEL": "claude-haiku-4-5"})
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(r.stdout.strip(), "")
        self.assertTrue(
            r.stderr.strip().startswith("capability-tier-gate: skipped —"), r.stderr
        )
        self.assertNotIn("fails its pin", r.stderr)

    def test_missing_registry_skips_loudly(self):
        empty = Path(tempfile.mkdtemp())
        self.addCleanup(lambda: shutil.rmtree(empty, ignore_errors=True))
        r = run_hook("git reset --hard abc", empty, {"CLAUDE_MODEL": "claude-haiku-4-5"})
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(r.stdout.strip(), "")
        self.assertTrue(
            r.stderr.strip().startswith("capability-tier-gate: skipped —"), r.stderr
        )

    # ── internal error path is spoken, never a bare silent exit ──────────────────────────────
    def test_internal_error_is_spoken_not_silently_swallowed(self):
        # A registry whose `policies:` value is a STRING, not a list -> data.get("policies") or
        # [] iterates over characters, each a str not a dict -> handled gracefully (skip, not a
        # crash) by _load_policy_row's isinstance guards. To force a genuine internal error we
        # feed the hook a stdin payload that isn't valid JSON at all.
        payload = "{not valid json"
        env = dict(os.environ)
        env["CLAUDE_PROJECT_DIR"] = str(self.proj)
        for k in ("CLAUDE_MODEL", "ANTHROPIC_MODEL", "CLAUDE_SESSION_ID"):
            env.pop(k, None)
        env["CLAUDE_MODEL"] = "claude-haiku-4-5"
        r = subprocess.run(
            [sys.executable, str(HOOK)], input=payload, capture_output=True, text=True,
            env=env, timeout=10,
        )
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(r.stdout.strip(), "")
        self.assertTrue(
            r.stderr.strip().startswith("capability-tier-gate: skipped — internal error:"),
            r.stderr,
        )

    # ── the real committed row also works end to end ─────────────────────────────────────────
    def test_real_repo_registry_denies_haiku(self):
        r = run_hook("git reset --hard abc", REPO, {"CLAUDE_MODEL": "claude-haiku-4-5"})
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("destructive-git-requires-tier", r.stdout)

    def test_real_repo_registry_allows_opus(self):
        r = run_hook("git reset --hard abc", REPO, {"CLAUDE_MODEL": "claude-opus-5"})
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(r.stdout.strip(), "")

    def test_real_repo_registry_bypass_matrix_still_denied(self):
        for cmd in ("git reset -q --hard", "git clean -xfd", "git checkout HEAD -- .",
                    "rm -rf *", "git rebase main", "git switch -C x"):
            with self.subTest(cmd=cmd):
                r = run_hook(cmd, REPO, {"CLAUDE_MODEL": "claude-haiku-4-5"})
                self.assertEqual(r.returncode, 0, r.stderr)
                out = json.loads(r.stdout)
                self.assertEqual(out["hookSpecificOutput"]["permissionDecision"], "deny", cmd)


if __name__ == "__main__":
    unittest.main()
