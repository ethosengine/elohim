#!/usr/bin/env python3
"""
Participant Standing — one SessionStart line naming the human this device stands for.

Hook Type: SessionStart (synchronous)

Post-station-4 sprint, Lane P task P4 (genesis/docs/superpowers/plans/
2026-09-25-post-station-4-participant-classifier-search-attention-sprint.md, rulings R-P6, R-P9):

  R-P6  A human's standing is per DEVICE: the latest signed human record whose signer is in the
        handle's tracked roster (`.eprfs/status/participants/<handle>.jsonl`) applies to every
        session on the device. `epr actor current --session <id> --json` carries it as
        `standing` whenever the session registered no claim of its own.
  R-P9  The elohim witness the human. Witnessing is a deliberate act, once per device, by an
        agent who knows who is present, with a basis it stands behind. So this hook only READS:
        it never runs `witness` or `claim` (nor `contest`), and when the device is unwitnessed
        it names the command such an agent would run — it does not run it for them.

The line, one of:

    participant: human:<h> (did:key:z…<last 6>, standing; witnessed by agent:<…> <date>)
    participant: human:<h> (did:key:z…<last 6>, standing; claimed by themselves <date>)
    participant: human:<h> (session claim <date>)
    participant: (unwitnessed) — when you know who is present: epr actor witness …
    participant: (unknown — <why>)

An agent claim on the session says who the AGENT is, not who the human is, and `current` returns
`standing: null` beside any claim; the device is then read once more under a session label no
one claims (still a read — `current` never writes, and never mints a key).

Emission: the `hookSpecificOutput.additionalContext` JSON wrapper, the documented landing path
for a synchronous SessionStart hook (see load-project-context.py). Fail-open: every path exits
0; an absent or failing binary is `(unknown — …)`, never a guess at `(unwitnessed)`.

Budget: each `epr` read is capped at _READ_TIMEOUT and there are at most two, inside the
registered 2 s.
"""

# The intervenor's removal condition (counted by _lib/intervenor_census.py). A condition, never
# a date.
RETIRE_WHEN = (
    "when the bootstrapping head (`epr flow memory recall open --purpose bootstrap`, rendered by "
    "load-project-context.py) carries the device's standing participant itself — at which point "
    "this line is a second rendering of the same read and the hook is deleted"
)

import json
import os
import subprocess
import sys

_READ_TIMEOUT = 0.8  # seconds per `epr actor current`; at most two reads

# A session label no one claims: the device read beside an agent's own session claim.
_DEVICE_PROBE_SESSION = "participant-standing:device-read"

WITNESS_HINT = (
    "participant: (unwitnessed) — when you know who is present: epr actor witness "
    "--subject human:<handle> --as <your agent ref> --session $CLAUDE_CODE_SESSION_ID "
    "--basis \"<what you know>\""
)


def _observation_module(project_dir: str):
    """The hooks' shared binary resolver, imported by path (load-project-context.py's idiom)."""
    for base in (os.path.join(project_dir, ".claude", "hooks"),
                 os.path.dirname(os.path.abspath(__file__))):
        if base not in sys.path:
            sys.path.insert(0, base)
    try:
        import _observation
        return _observation
    except Exception:
        return None


def _short_did(did: str) -> str:
    return f"did:key:z…{did[-6:]}" if did.startswith("did:key:z") and len(did) > 6 else did


def _date(stamp) -> str:
    return str(stamp or "")[:10] or "undated"


def _current(binary: str, project_dir: str, session: str):
    """`epr actor current --json` as a dict, or an error string."""
    try:
        r = subprocess.run(
            [binary, "actor", "current", "--session", session, "--json", "--root", project_dir],
            capture_output=True, text=True, timeout=_READ_TIMEOUT, cwd=project_dir,
        )
    except subprocess.TimeoutExpired:
        return f"epr actor current exceeded {_READ_TIMEOUT}s"
    except OSError as e:
        return f"epr actor current did not run ({e.strerror or e})"
    if r.returncode != 0:
        tail = (r.stderr or r.stdout or "").strip().splitlines()
        return f"epr actor current exited {r.returncode}" + (f": {tail[-1][:120]}" if tail else "")
    try:
        body = json.loads(r.stdout)
    except (json.JSONDecodeError, ValueError):
        return "epr actor current --json returned no JSON"
    return body if isinstance(body, dict) else "epr actor current --json returned no object"


def _standing_line(body: dict):
    """The line for a `current` body with no session claim, or None to fall through."""
    if "standing" not in body:
        return (f"participant: (unknown — this epr predates the device standing read; "
                f"rebuild it at HEAD)")
    standing = body.get("standing")
    if not isinstance(standing, dict):
        return WITNESS_HINT
    subject = standing.get("subject") or f"human:{standing.get('handle', '?')}"
    device = _short_did(str(standing.get("device") or ""))
    on = _date(standing.get("claimedAt"))
    witness = standing.get("witnessedBy")
    by = f"witnessed by {witness} {on}" if witness else f"claimed by themselves {on}"
    return f"participant: {subject} ({device}, standing; {by})"


def participant_line(project_dir: str, session: str) -> str:
    obs = _observation_module(project_dir)
    binary = obs.resolve_bin() if obs else None
    if not binary:
        return "participant: (unknown — epr binary not found)"
    body = _current(binary, project_dir, session)
    if isinstance(body, str):
        return f"participant: (unknown — {body})"
    claim = body.get("claim")
    if isinstance(claim, dict) and claim.get("claimed"):
        claimed = str(claim["claimed"])
        if claimed.startswith("human:"):
            return f"participant: {claimed} (session claim {_date(claim.get('claimedAt'))})"
        # An agent's own claim hides the device standing; read the device on its own.
        body = _current(binary, project_dir, _DEVICE_PROBE_SESSION)
        if isinstance(body, str):
            return f"participant: (unknown — {body})"
    return _standing_line(body)


def main() -> None:
    try:
        raw = sys.stdin.read()
        payload = json.loads(raw) if raw.strip() else {}
        if not isinstance(payload, dict):
            payload = {}
    except Exception:
        payload = {}
    project_dir = (os.environ.get("CLAUDE_PROJECT_DIR") or payload.get("cwd") or os.getcwd())
    session = (str(payload.get("session_id") or "")
               or os.environ.get("CLAUDE_CODE_SESSION_ID", "")
               or _DEVICE_PROBE_SESSION)
    try:
        line = participant_line(project_dir, session)
    except Exception as e:  # fail-open, but spoken
        line = f"participant: (unknown — {type(e).__name__})"
    print(json.dumps({
        "hookSpecificOutput": {"hookEventName": "SessionStart", "additionalContext": line}
    }))


if __name__ == "__main__":
    main()
    sys.exit(0)
