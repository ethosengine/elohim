#!/usr/bin/env python3
"""Task 2.4, fix round 2 (adversarial re-review, 30+ new payloads): round 1's tokeniser required
the unwrapped HEAD token to literally be `git`/`rm`/a shell — every indirect invocation (`eval`,
`$(…)`/backtick, `xargs`, a piped `xargs -I{} bash -c '{}'`, bare parens/braces/keywords, `bash
-lc`/`-x -c`, `nohup`/`setsid`/`stdbuf`/`time`, an interpreter `-c` string) resolved to a
non-git/rm head and was silently ALLOWED. `.claude/hooks/capability-tier-gate.py` now does DENY
ON AMBIGUITY: a wider wrapper/keyword strip set, flexible shell `-c` detection, and — when still
indirect after that — an unconditional deny naming the exact reason
"indirect invocation carrying a destructive token; run it plainly or ask the controller".

Also this round: declared-class gaps closed (`push +refspec`/`--delete`, `reset --keep`, git
subcommand `rm -r .`/`*`, `checkout --orphan`, `symbolic-ref` writes); over-blocks relaxed
(`restore`/`checkout` on a specific path, `branch --delete` without force, `rebase
--abort|--continue|--skip|--quit`); and the REAL session-id env var for this harness
(`CLAUDE_CODE_SESSION_ID`, read before the round-1 `CLAUDE_SESSION_ID` fallback) so the actor
sidecar lookup actually resolves a claimed tier.

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

# A self-contained fixture registry mirroring the real committed row's shape (round 2's `rules:`
# + `rm-force-recursive-targets:`) so every test here is independent of the live repo registry
# drifting under it.
FIXTURE_POLICY_BODY = """\
epr-meta-policies-version: 1
policies:
  - id: destructive-git-requires-tier
    version: 1
    class: deny
    parameters:
      rules:
        - { sub: reset, any_flags: ["--hard", "--merge", "--keep"] }
        - { sub: checkout, any_args: [".", "*", ":/", "-B", "-f", "--force", "--orphan"] }
        - { sub: restore, any_args: [".", "*", ":/"] }
        - { sub: clean, flag_letters: ["f"] }
        - { sub: push, any_flags: ["-f", "--force", "--force-with-lease", "--delete", "-d"] }
        - { sub: push, any_arg_prefix: ["+"] }
        - { sub: branch, any_flags: ["-D", "-f", "--force"] }
        - { sub: switch, any_flags: ["-C", "--force-create"] }
        - { sub: rebase, exempt_if_any_args: ["--abort", "--continue", "--skip", "--quit"] }
        - { sub: filter-branch }
        - { sub: reflog, any_args: ["expire", "delete"] }
        - { sub: gc, any_flags: ["--prune"] }
        - { sub: update-ref }
        - { sub: stash, any_args: ["drop", "clear"] }
        - { sub: worktree, any_args: ["remove"], any_flags: ["--force", "-f"] }
        - { sub: rm, flag_letters: ["r"], any_args: [".", "*"] }
        - { sub: symbolic-ref, min_positional_args: 2 }
        - { sub: symbolic-ref, any_flags: ["-d", "--delete"] }
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
    import yaml

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
    for k in ("CLAUDE_MODEL", "ANTHROPIC_MODEL", "CLAUDE_SESSION_ID", "CLAUDE_CODE_SESSION_ID"):
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


def _deny_reason(result) -> str:
    out = json.loads(result.stdout)
    return out["hookSpecificOutput"]["permissionDecisionReason"]


class CapabilityTierGateCase(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.proj = Path(self._tmp.name)
        _write_project(self.proj, FIXTURE_POLICY, actor_lines=[])

    # ── round 1 regressions: still denied ────────────────────────────────────────────────────
    ROUND1_BYPASSES = [
        "git reset -q --hard",
        "git clean -xfd",
        "git checkout HEAD -- .",
        "git restore --staged --worktree .",
        "rm -rf *",
        "rm -rf ..",
        "git rebase main",
        "git filter-branch --tree-filter x",
        "git checkout -B mybranch",
        "git switch -C mybranch",
        "git reflog expire --all",
        "git gc --prune=now",
        "git branch -f main HEAD~1",
    ]

    def test_round1_bypasses_still_denied(self):
        for cmd in self.ROUND1_BYPASSES:
            with self.subTest(cmd=cmd):
                r = run_hook(cmd, self.proj, {"CLAUDE_MODEL": "claude-haiku-4-5"})
                self.assertEqual(r.returncode, 0, r.stderr)
                self.assertEqual(json.loads(r.stdout)["hookSpecificOutput"]["permissionDecision"],
                                  "deny", cmd)

    # ── round 2: the head-token bypass class — ALL must now DENY (some directly, some indirect)
    ROUND2_DIRECT = {
        # unwrapped through the wider wrapper/keyword strip set: DIRECT classification
        '(git reset --hard)': "git reset",
        '{ git reset --hard; }': "git reset",
        "if true; then git reset --hard; fi": "git reset",
        "for i in 1 2 3; do git reset --hard; done": "git reset",
        'bash -lc "git reset --hard"': "git reset",
        'bash -x -c "git reset --hard"': "git reset",
        "nohup git reset --hard": "git reset",
        "setsid git reset --hard": "git reset",
        "stdbuf -o0 git reset --hard": "git reset",
        "time git reset --hard": "git reset",
        'eval "git reset --hard"': "git reset",
        "eval git reset --hard": "git reset",
    }

    ROUND2_INDIRECT = [
        "$(echo git) reset --hard",
        "`echo git` reset --hard",
        "xargs git reset --hard",
        'echo "git reset --hard" | xargs -I{} bash -c \'{}\'',
        'python3 -c "import subprocess; subprocess.run(\'git reset --hard\')"',
    ]

    def test_round2_direct_unwrap_matrix_denied(self):
        for cmd, expect_substr in self.ROUND2_DIRECT.items():
            with self.subTest(cmd=cmd):
                r = run_hook(cmd, self.proj, {"CLAUDE_MODEL": "claude-haiku-4-5"})
                self.assertEqual(r.returncode, 0, r.stderr)
                out = json.loads(r.stdout)
                self.assertEqual(out["hookSpecificOutput"]["permissionDecision"], "deny", cmd)
                self.assertIn(expect_substr, out["hookSpecificOutput"]["permissionDecisionReason"], cmd)

    def test_round2_indirect_matrix_denied_with_the_exact_reason(self):
        for cmd in self.ROUND2_INDIRECT:
            with self.subTest(cmd=cmd):
                r = run_hook(cmd, self.proj, {"CLAUDE_MODEL": "claude-haiku-4-5"})
                self.assertEqual(r.returncode, 0, r.stderr)
                reason = _deny_reason(r)
                self.assertIn(
                    "indirect invocation carrying a destructive token; run it plainly or ask "
                    "the controller",
                    reason,
                    cmd,
                )

    def test_indirect_denies_regardless_of_tier(self):
        # DENY ON AMBIGUITY bypasses tier resolution entirely — even opus cannot clear it.
        r = run_hook("xargs git reset --hard", self.proj, {"CLAUDE_MODEL": "claude-opus-5"})
        self.assertEqual(r.returncode, 0, r.stderr)
        out = json.loads(r.stdout)
        self.assertEqual(out["hookSpecificOutput"]["permissionDecision"], "deny")

    # ── declared-class gaps closed this round ────────────────────────────────────────────────
    DECLARED_GAPS = [
        "git push origin +main",
        "git push --delete origin branchname",
        "git reset --keep HEAD~1",
        "git rm -rf .",
        "git rm -rf *",
        "git checkout --orphan newbranch",
        "git symbolic-ref HEAD refs/heads/main",
        "git symbolic-ref -d HEAD",
    ]

    def test_declared_class_gaps_denied(self):
        for cmd in self.DECLARED_GAPS:
            with self.subTest(cmd=cmd):
                r = run_hook(cmd, self.proj, {"CLAUDE_MODEL": "claude-haiku-4-5"})
                self.assertEqual(r.returncode, 0, r.stderr)
                out = json.loads(r.stdout)
                self.assertEqual(out["hookSpecificOutput"]["permissionDecision"], "deny", cmd)

    # ── over-blocks relaxed this round ───────────────────────────────────────────────────────
    RELAXED_ALLOWED = [
        "git restore --staged file.txt",
        "git checkout -- file.txt",
        "git branch --delete oldbranch",
        "git rebase --abort",
        "git rebase --continue",
        "git rebase --skip",
        "git rebase --quit",
        "git symbolic-ref HEAD",  # read-only, 1 positional arg, no -d
        "git push origin main",  # ordinary push
        "git worktree remove somepath",  # no force flag
    ]

    def test_relaxed_cases_allowed(self):
        for cmd in self.RELAXED_ALLOWED:
            with self.subTest(cmd=cmd):
                r = run_hook(cmd, self.proj, {"CLAUDE_MODEL": "claude-haiku-4-5"})
                self.assertEqual(r.returncode, 0, r.stderr)
                self.assertEqual(r.stdout.strip(), "", f"unexpected deny for: {cmd}")

    # ── required-allowed matrix (unchanged from round 1) ─────────────────────────────────────
    MUST_ALLOW = [
        "git reset --soft HEAD~1",
        "git status",
        "git log",
        "echo git reset --hard x",
        "echo git reset --hard x",  # explicit dup: prose false positive named by the reviewer
        "rm -rf ./target",
        "git clean -n",
    ]

    def test_prose_false_positive_and_required_allow_matrix(self):
        for cmd in self.MUST_ALLOW:
            with self.subTest(cmd=cmd):
                r = run_hook(cmd, self.proj, {"CLAUDE_MODEL": "claude-haiku-4-5"})
                self.assertEqual(r.returncode, 0, r.stderr)
                self.assertEqual(r.stdout.strip(), "", f"unexpected deny for: {cmd}")
                self.assertEqual(r.stderr.strip(), "", f"unexpected stderr for: {cmd}")

    def test_prose_exemption_void_when_piped_into_xargs(self):
        # The SAME prose head loses its exemption once piped into xargs/bash -c.
        r = run_hook(
            'echo "git reset --hard" | xargs -I{} bash -c \'{}\'',
            self.proj, {"CLAUDE_MODEL": "claude-haiku-4-5"},
        )
        out = json.loads(r.stdout)
        self.assertEqual(out["hookSpecificOutput"]["permissionDecision"], "deny")

    def test_interpreter_alias_and_command_substitution_residuals_documented(self):
        # Residual (KNOWN, documented in the report, not required by the reviewer's matrix): an
        # interpreter reached through a shell ALIAS, or dynamic code built from string
        # concatenation rather than a literal git/rm word in the -c argument, is NOT detected —
        # the interpreter check is a literal \bgit\b|\brm\b word-boundary scan of the -c string,
        # not a Python/Perl/Ruby/Node parser. This test documents the residual by asserting the
        # CURRENT (accepted) behavior rather than silently leaving it unasserted.
        r = run_hook(
            'python3 -c "cmd = \'g\' + \'it reset --hard\'; import os; os.system(cmd)"',
            self.proj, {"CLAUDE_MODEL": "claude-haiku-4-5"},
        )
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(
            r.stdout.strip(), "",
            "residual: string-concatenation-built destructive commands inside an interpreter "
            "are NOT detected (no git/rm literal word in the -c argument) — documented gap, not "
            "a regression",
        )

    def test_commands_with_no_git_or_rm_never_touch_the_registry(self):
        empty = Path(tempfile.mkdtemp())
        self.addCleanup(lambda: shutil.rmtree(empty, ignore_errors=True))
        r = run_hook("echo hello", empty, {"CLAUDE_MODEL": "claude-haiku-4-5"})
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(r.stdout.strip(), "")
        self.assertEqual(r.stderr.strip(), "")

    # ── session key: CLAUDE_CODE_SESSION_ID is the REAL var for this harness ────────────────
    def test_claude_code_session_id_resolves_a_sidecar_claim(self):
        session = "cc-sess-1"
        _write_project(
            self.proj,
            FIXTURE_POLICY,
            actor_lines=[_actor_claim_line("agent:implementer@claude-fable-5-1", session)],
        )
        r = run_hook("git reset --hard abc", self.proj, {"CLAUDE_CODE_SESSION_ID": session})
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(r.stdout.strip(), "", "CLAUDE_CODE_SESSION_ID must resolve the claim")

    def test_claude_session_id_still_works_as_a_fallback(self):
        session = "legacy-sess-1"
        _write_project(
            self.proj,
            FIXTURE_POLICY,
            actor_lines=[_actor_claim_line("agent:implementer@claude-fable-5-1", session)],
        )
        r = run_hook("git reset --hard abc", self.proj, {"CLAUDE_SESSION_ID": session})
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(r.stdout.strip(), "")

    def test_claude_code_session_id_takes_priority_over_claude_session_id(self):
        # A claim under CLAUDE_CODE_SESSION_ID must be found even when CLAUDE_SESSION_ID is ALSO
        # set to something with no (or a lower) claim.
        real_session = "cc-priority-sess"
        _write_project(
            self.proj,
            FIXTURE_POLICY,
            actor_lines=[_actor_claim_line("agent:implementer@claude-fable-5-1", real_session)],
        )
        r = run_hook(
            "git reset --hard abc", self.proj,
            {"CLAUDE_CODE_SESSION_ID": real_session, "CLAUDE_SESSION_ID": "no-such-session"},
        )
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(r.stdout.strip(), "")

    # ── tier resolution (round 1, still asserted) ────────────────────────────────────────────
    def test_haiku_below_floor_denied(self):
        r = run_hook("git reset --hard abc", self.proj, {"CLAUDE_MODEL": "claude-haiku-4-5"})
        self.assertEqual(r.returncode, 0, r.stderr)
        reason = _deny_reason(r)
        self.assertIn("destructive-git-requires-tier", reason)
        self.assertIn("check with the team first", reason)

    def test_remedy_carries_no_self_claim_hint(self):
        r = run_hook("git reset --hard abc", self.proj, {"CLAUDE_MODEL": "claude-haiku-4-5"})
        reason = _deny_reason(r)
        self.assertNotIn("epr actor claim", reason)

    def test_opus_at_floor_allowed(self):
        r = run_hook("git reset --hard abc", self.proj, {"CLAUDE_MODEL": "claude-opus-5"})
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(r.stdout.strip(), "")

    def test_unknown_tier_denied(self):
        r = run_hook(
            "git reset --hard abc", self.proj, {"CLAUDE_SESSION_ID": "session-with-no-claim"}
        )
        self.assertEqual(r.returncode, 0, r.stderr)
        reason = _deny_reason(r)
        self.assertIn("unknown", reason)

    def test_anthropic_model_env_is_never_consulted(self):
        r = run_hook("git reset --hard abc", self.proj, {"ANTHROPIC_MODEL": "claude-opus-5"})
        self.assertEqual(r.returncode, 0, r.stderr)
        reason = _deny_reason(r)
        self.assertIn("unknown", reason)

    # ── contentHash pin verification (round 1, still asserted) ──────────────────────────────
    def test_tampered_row_denies_every_git_rm_candidate(self):
        tampered = FIXTURE_POLICY.replace(
            "rm-force-recursive-targets: [\".\", \"./\", \"..\", \"*\", \"~\", \"/\"]",
            "rm-force-recursive-targets: [\".\"]",
        )
        _write_project(self.proj, tampered, actor_lines=[])
        for cmd in ("git reset --hard abc", "git status"):
            with self.subTest(cmd=cmd):
                r = run_hook(cmd, self.proj, {"CLAUDE_MODEL": "claude-opus-5"})
                self.assertEqual(r.returncode, 0, r.stderr)
                self.assertIn("fails its pin", _deny_reason(r))

    # ── malformed row: honest skip ────────────────────────────────────────────────────────────
    def test_malformed_row_skips_loudly(self):
        _write_project(self.proj, MALFORMED_POLICY, actor_lines=[])
        r = run_hook("git reset --hard abc", self.proj, {"CLAUDE_MODEL": "claude-haiku-4-5"})
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(r.stdout.strip(), "")
        self.assertTrue(r.stderr.strip().startswith("capability-tier-gate: skipped —"), r.stderr)

    # ── internal error: spoken, never silent ─────────────────────────────────────────────────
    def test_internal_error_is_spoken_not_silently_swallowed(self):
        env = dict(os.environ)
        env["CLAUDE_PROJECT_DIR"] = str(self.proj)
        for k in ("CLAUDE_MODEL", "ANTHROPIC_MODEL", "CLAUDE_SESSION_ID", "CLAUDE_CODE_SESSION_ID"):
            env.pop(k, None)
        env["CLAUDE_MODEL"] = "claude-haiku-4-5"
        r = subprocess.run(
            [sys.executable, str(HOOK)], input="{not valid json", capture_output=True, text=True,
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

    def test_real_repo_registry_round2_matrix(self):
        for cmd in ("eval \"git reset --hard\"", "xargs git reset --hard",
                    "git push origin +main", "git rm -rf .", "git checkout --orphan x"):
            with self.subTest(cmd=cmd):
                r = run_hook(cmd, REPO, {"CLAUDE_MODEL": "claude-haiku-4-5"})
                self.assertEqual(r.returncode, 0, r.stderr)
                out = json.loads(r.stdout)
                self.assertEqual(out["hookSpecificOutput"]["permissionDecision"], "deny", cmd)

    def test_real_repo_registry_relaxed_matrix(self):
        for cmd in ("git restore --staged file.txt", "git checkout -- file.txt",
                    "git branch --delete oldbranch", "git rebase --abort"):
            with self.subTest(cmd=cmd):
                r = run_hook(cmd, REPO, {"CLAUDE_MODEL": "claude-haiku-4-5"})
                self.assertEqual(r.returncode, 0, r.stderr)
                self.assertEqual(r.stdout.strip(), "", cmd)


if __name__ == "__main__":
    unittest.main()
