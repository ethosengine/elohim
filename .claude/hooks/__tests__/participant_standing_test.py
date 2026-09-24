#!/usr/bin/env python3
"""The participant-standing SessionStart line (post-station-4 sprint, Lane P task P4).

Rulings under test (genesis/docs/superpowers/plans/
2026-09-25-post-station-4-participant-classifier-search-attention-sprint.md):

  R-P6  standing is per DEVICE: `epr actor current --json` carries `standing` when the session
        registered no claim — the hook renders it as one line.
  R-P9  the elohim witness the human: witnessing is a deliberate act with a basis. The hook
        NEVER runs `witness` or `claim`; when the device is unwitnessed it names the command an
        agent who knows who is present would run.
  X2    honest floors: no binary is `(unknown — …)`, never a guess, and never a non-zero exit.

The `epr` here is a stub (the drift_observation_test.py pattern): every invocation is appended
to $STUB_LOG as one JSON argv line, and `actor current` answers with $STUB_CURRENT (a JSON
object whose `session` is echoed from argv) or, for a session listed in $STUB_CLAIMED, with a
claim present and `standing: null` — the real binary's contract (actor.rs `current_on_device`).

Run: python3 -m unittest discover -s .claude/hooks/__tests__ -p 'participant_standing_test.py'
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
HOOK = HOOKS / "participant-standing.py"

STUB = '''#!/usr/bin/env python3
import json, os, sys
argv = sys.argv[1:]
with open(os.environ["STUB_LOG"], "a") as fh:
    fh.write(json.dumps(argv) + "\\n")
if argv[:2] == ["actor", "current"]:
    session = argv[argv.index("--session") + 1] if "--session" in argv else None
    claimed = json.loads(os.environ.get("STUB_CLAIMED", "{}"))
    if session in claimed:
        print(json.dumps({"session": session, "claim": {
            "claimed": claimed[session], "session": session,
            "claimedAt": "2026-09-24T10:00:00Z", "definitionCid": None,
            "recordCid": "bafyreistubclaim"}, "standing": None}))
        sys.exit(0)
    body = json.loads(os.environ.get("STUB_CURRENT", '{"claim": null, "standing": null}'))
    body["session"] = session
    print(json.dumps(body))
    sys.exit(0)
sys.exit(2)
'''

DEVICE = "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK"

STANDING = {
    "claim": None,
    "standing": {
        "subject": "human:matthew",
        "handle": "matthew",
        "recordCid": "bafyreistubwitness",
        "witnessedBy": "agent:orchestrator@claude-fable-5-1",
        "claimedAt": "2026-09-24T16:45:00Z",
        "device": DEVICE,
    },
}

UNWITNESSED = {"claim": None, "standing": None}


class ParticipantStandingCase(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix="participant-standing-"))
        self.addCleanup(shutil.rmtree, self.tmp, True)
        self.project = self.tmp / "project"
        hooks = self.project / ".claude" / "hooks"
        hooks.mkdir(parents=True)
        # The fixture's `_observation` resolves no gate-target binary, so the ONLY `epr` the
        # hook can find is the one a test puts on $EPR_BIN (the real gate target on this host
        # would otherwise answer `binary_absent`).
        obs = (HOOKS / "_observation.py").read_text()
        self.assertIn('_GATE_TARGET_BIN = "', obs)
        obs = obs.replace(
            obs[obs.index('_GATE_TARGET_BIN = "'):].split("\n", 1)[0],
            '_GATE_TARGET_BIN = "/nonexistent/participant-standing-test/epr"',
        )
        (hooks / "_observation.py").write_text(obs)
        self.log = self.tmp / "stub.log"
        self.stub = self.tmp / "bin" / "epr"
        self.stub.parent.mkdir()
        # An absolute interpreter: $PATH is emptied so no real `epr` can be found on it.
        self.stub.write_text(STUB.replace("#!/usr/bin/env python3", f"#!{sys.executable}", 1))
        self.stub.chmod(0o755)
        self.empty_path = self.tmp / "empty-path"
        self.empty_path.mkdir()

    def run_hook(self, payload, *, stub=True, current=None, claimed=None, **envextra):
        env = {k: v for k, v in os.environ.items()
               if k not in ("EPR_BIN", "CLAUDE_CODE_SESSION_ID", "CLAUDE_SESSION_ID")}
        env.update({
            "CLAUDE_PROJECT_DIR": str(self.project),
            "STUB_LOG": str(self.log),
            "STUB_CURRENT": json.dumps(current if current is not None else UNWITNESSED),
            "STUB_CLAIMED": json.dumps(claimed or {}),
            "PATH": str(self.empty_path),
        })
        if stub:
            env["EPR_BIN"] = str(self.stub)
        env.update(envextra)
        return subprocess.run(
            [sys.executable, str(HOOK)],
            input=payload if isinstance(payload, str) else json.dumps(payload),
            capture_output=True, text=True, env=env, timeout=30,
        )

    def line(self, result) -> str:
        self.assertEqual(result.returncode, 0, result.stderr)
        out = json.loads(result.stdout)
        ctx = out["hookSpecificOutput"]["additionalContext"]
        self.assertEqual(out["hookSpecificOutput"]["hookEventName"], "SessionStart")
        self.assertEqual(len(ctx.splitlines()), 1, ctx)
        return ctx

    def calls(self) -> list[list[str]]:
        if not self.log.exists():
            return []
        return [json.loads(l) for l in self.log.read_text().splitlines() if l.strip()]

    # ── the five named cases ─────────────────────────────────────────────────────────────────
    def test_prints_standing_line_from_current_json(self):
        r = self.run_hook({"session_id": "sess-a"}, current=STANDING)
        self.assertEqual(
            self.line(r),
            "participant: human:matthew (did:key:z…ta2doK, standing; witnessed by "
            "agent:orchestrator@claude-fable-5-1 2026-09-24)",
        )

    def test_prints_unwitnessed_with_the_witness_command_when_standing_null(self):
        r = self.run_hook({"session_id": "sess-b"}, current=UNWITNESSED)
        self.assertEqual(
            self.line(r),
            "participant: (unwitnessed) — when you know who is present: epr actor witness "
            "--subject human:<handle> --as <your agent ref> --session $CLAUDE_CODE_SESSION_ID "
            "--basis \"<what you know>\"",
        )

    def test_never_invokes_witness_or_claim(self):
        for current in (STANDING, UNWITNESSED):
            self.line(self.run_hook({"session_id": "sess-c"}, current=current))
        self.line(self.run_hook({"session_id": "sess-d"},
                                claimed={"sess-d": "agent:implementer@claude-opus-5-5"}))
        calls = self.calls()
        self.assertTrue(calls, "the hook must read `epr actor current`")
        for argv in calls:
            self.assertEqual(argv[:2], ["actor", "current"], argv)
            self.assertNotIn("witness", argv)
            self.assertNotIn("claim", argv)
            self.assertNotIn("contest", argv)

    def test_binary_absent_is_unknown_exit_zero(self):
        r = self.run_hook({"session_id": "sess-e"}, stub=False)
        self.assertEqual(self.line(r), "participant: (unknown — epr binary not found)")
        self.assertEqual(self.calls(), [])

    def test_session_id_from_stdin_beats_env(self):
        self.line(self.run_hook({"session_id": "from-stdin"},
                                CLAUDE_CODE_SESSION_ID="from-env"))
        argv = self.calls()[0]
        self.assertEqual(argv[argv.index("--session") + 1], "from-stdin")

    # ── the edges the five imply ─────────────────────────────────────────────────────────────
    def test_env_session_id_when_stdin_has_none(self):
        self.line(self.run_hook("", CLAUDE_CODE_SESSION_ID="from-env"))
        argv = self.calls()[0]
        self.assertEqual(argv[argv.index("--session") + 1], "from-env")

    def test_self_claimed_standing_says_so(self):
        own = json.loads(json.dumps(STANDING))
        own["standing"]["witnessedBy"] = None
        r = self.run_hook({"session_id": "sess-f"}, current=own)
        self.assertEqual(
            self.line(r),
            "participant: human:matthew (did:key:z…ta2doK, standing; claimed by themselves "
            "2026-09-24)",
        )

    def test_an_agent_session_claim_does_not_hide_the_device_standing(self):
        # `current` returns `standing: null` beside ANY session claim; an agent claim says who
        # the agent is, not who the human is, so the device is read again with a session no
        # one claims — still a read, never a claim.
        r = self.run_hook({"session_id": "sess-g"}, current=STANDING,
                          claimed={"sess-g": "agent:implementer@claude-opus-5-5"})
        self.assertIn("participant: human:matthew (did:key:z…ta2doK, standing;", self.line(r))
        self.assertEqual(len(self.calls()), 2)

    def test_a_human_session_claim_is_the_participant(self):
        r = self.run_hook({"session_id": "sess-h"}, claimed={"sess-h": "human:matthew"})
        self.assertEqual(self.line(r),
                         "participant: human:matthew (session claim 2026-09-24)")

    def test_a_failing_binary_is_unknown_not_unwitnessed(self):
        broken = self.tmp / "bin" / "broken-epr"
        broken.write_text("#!/bin/sh\necho boom >&2\nexit 2\n")
        broken.chmod(0o755)
        r = self.run_hook({"session_id": "sess-i"}, EPR_BIN=str(broken))
        self.assertTrue(self.line(r).startswith("participant: (unknown — "), r.stdout)

    def test_a_binary_without_the_standing_read_is_unknown(self):
        # An `epr` older than 71eac9dc8 answers `current --json` without a `standing` key.
        r = self.run_hook({"session_id": "sess-j"}, current={"claim": None})
        line = self.line(r)
        self.assertTrue(line.startswith("participant: (unknown — "), line)
        self.assertIn("standing", line)


if __name__ == "__main__":
    unittest.main()
