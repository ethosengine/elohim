#!/usr/bin/env python3
"""
Covenant Subagent Stamp — carries the session covenant into every agent the session spawns.

Hook Types: SubagentStart (cannot block; its additionalContext lands in the SUBAGENT's context)
            PreToolUse:Bash (refuses a subagent's own attempt to record a covenant clause)

The steward of record answers the session's covenant once, in the main conversation
(participant-standing.py asks; `epr covenant record` keeps the answers). A spawned agent never saw
that dialog, and AskUserQuestion is unavailable to it, so without this stamp it would either work
under the full rules unaware of a granted floor or — worse — infer a floor nobody granted. The
stamp names what is held, where, under which clause, and that the agent can neither grant nor
extend one. Hooks firing on the subagent's own writes still decide per write (`epr covenant
check`, keyed by the parent session id the subagent inherits); this is the in-flight memory of it.

The PreToolUse leg is the same boundary from the other side: a tool call carrying `agent_id` is
a subagent's, and a subagent never heard the operator, so `epr covenant record` or a direct
`brit-build-ref covenant put` from it is refused. This is a guard against honest drift (a string
match), not against a determined forger — rung 2's human co-signature is what closes that.

`blind-reader` is skipped: its isolation contract admits only a path and a profile, and it only
reads. Nothing held, or a covenant that cannot be read, stamps nothing: the full rules apply and
are already stated where they live.
"""

# The intervenor's removal condition (counted by _lib/intervenor_census.py). A condition, never
# a date.
RETIRE_WHEN = (
    "when spawned agents receive the session covenant from the runtime's own session context "
    "(a covenant-aware agent launch, or a notarized governance commitment the agent reads itself)"
)

import json
import os
import re
import sys
from pathlib import Path

RECORDING = re.compile(r"\bcovenant\s+(record|put)\b")

SKIP_AGENT_TYPES = {"blind-reader"}


sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
from _lib.session import resolve as resolve_session_identity

def stamp(show: dict) -> str | None:
    """The stamp text for an `epr covenant show` body, or None when nothing is held."""
    variances = [v for v in show.get("variances") or [] if isinstance(v, dict)]
    steward = show.get("steward")
    purpose = (show.get("purpose") or {}).get("purpose") if isinstance(show.get("purpose"), dict) else None
    if not variances and not steward:
        return None
    lines = ["[SESSION COVENANT] This session's steward of record answered its covenant before you "
             "were spawned; it binds your work too."]
    if steward:
        lines.append(f"  steward of record: {steward}")
    if purpose:
        lines.append(f"  declared purpose: {purpose}")
    for v in variances:
        paths = ", ".join(v.get("paths") or []) or "?"
        lines.append(f"  rule `{v.get('rule')}` is held at its floor within {paths} "
                     f"(clause {v.get('clause')}): {v.get('floor')}")
    lines.append("  Outside those paths every rule applies in full. You cannot grant, extend or "
                 "record a covenant clause, and you never ask the operator; if a rule's floor seems "
                 "warranted where none is held, say so in your report. Where a floor is held, be "
                 "careful: it trusts this session's declared purpose.")
    return "\n".join(lines)


def refuse_subagent_recording(payload: dict) -> str | None:
    """The refusal reason when a subagent's Bash call would record a clause, else None."""
    if not payload.get("agent_id") or payload.get("tool_name") != "Bash":
        return None
    command = (payload.get("tool_input") or {}).get("command") or ""
    if not RECORDING.search(command):
        return None
    return ("A covenant clause records what the steward of record said in the main conversation; "
            "a spawned agent never heard them, so it cannot record, grant or extend one. Report "
            "what you would ask instead, and the main session will ask the operator.")


def main() -> int:
    try:
        raw = sys.stdin.read()
        payload = json.loads(raw) if raw.strip() else {}
        if not isinstance(payload, dict):
            payload = {}
    except Exception:
        payload = {}
    if payload.get("hook_event_name") == "PreToolUse":
        reason = refuse_subagent_recording(payload)
        if reason:
            print(json.dumps({"hookSpecificOutput": {"hookEventName": "PreToolUse",
                                                     "permissionDecision": "deny",
                                                     "permissionDecisionReason": reason}}))
        return 0
    if payload.get("agent_type") in SKIP_AGENT_TYPES:
        return 0
    session = resolve_session_identity()["session"]
    project = Path(os.environ.get("CLAUDE_PROJECT_DIR") or payload.get("cwd") or os.getcwd())
    scripts = project / ".claude" / "scripts"
    if str(scripts) not in sys.path:
        sys.path.insert(0, str(scripts))
    try:
        from _lib import epr_client

        show = epr_client.covenant(project, "show", session)
    except Exception:
        return 0
    if show.get("status") == "unavailable":
        return 0
    text = stamp(show)
    if text:
        print(json.dumps({"hookSpecificOutput": {"hookEventName": "SubagentStart",
                                                 "additionalContext": text}}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
