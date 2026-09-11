#!/usr/bin/env python3
"""
Capability Tier Gate — destructive git needs a declared tier or a team check.

Hook Type: PreToolUse   Matcher: Bash

Why (operator, 2026-09-11): a Haiku subagent of another session ran `git reset --hard` on the
shared `dev` checkout — three commits left the branch pointer and ~27 tracked files' uncommitted
work across several lanes was wiped; recovery took two sessions an hour (see
.claude/memory/feedback_tiered_agent_capabilities_destructive_git.md). Haiku- and Sonnet-tier
agents must not hold `git reset`-level capability without checking with the team first. The tier
vocabulary is the one the reader lens already uses (`agent:<role>@<model>`) — a declared
capability table, evaluated at the edge, printed with its reason.

The table lives in ONE place: `.claude/epr-meta/policies.yaml` row
`destructive-git-requires-tier@1` (patterns, tier-order, tier-floor, unknown-tier, remedy). This
hook is a thin reader of that declared table, never a second copy of it — see the
`capability-tier-gate-owns-destructive-git` rule in `.claude/hooks/.epr-meta`.

Tier resolution order (fail-closed): `CLAUDE_MODEL` or `ANTHROPIC_MODEL` env -> the actor
sidecar's latest claim (`.eprfs/status/actors.jsonl`) for `CLAUDE_SESSION_ID` -> `unknown`.
`unknown-tier: deny` in the row means an unclaimed actor is treated as BELOW the floor, never
assumed safe.

Deny shape copies `cargo-disk-guard.py`'s exact convention: one `hookSpecificOutput` JSON object
on stdout (`permissionDecision: deny` + `permissionDecisionReason`), process exit 0 — the harness
reads the JSON, not the exit code, for a structured PreToolUse deny.

Fail-open by contract, but never silently: a malformed/missing policy row prints
`capability-tier-gate: skipped — <reason>` to stderr and exits 0 (every other hook in this tree
fails open silently; this one says so, because "why didn't the gate fire" must be answerable).
A non-destructive command (or one where the cheap `git`/`rm` pre-filter never fires) passes
without touching the actor sidecar at all — the sidecar read is paid only when a destructive
pattern actually matches, never on every Bash command.
"""

# The intervenor's removal condition (counted by _lib/intervenor_census.py). A condition, never
# a date.
RETIRE_WHEN = (
    "when the harness enforces agent-tier command capability natively — a first-class permission "
    "scope keyed on the acting model, not a repo-side pattern table — at which point this hook "
    "is a redundant second enforcement of a guarantee the platform already holds. Until then an "
    "unscoped git-reset-class command is unrecoverable, cross-session data loss on a shared "
    "checkout, and that risk does not retire on a quiet quarter."
)

import json
import os
import re
import sys

PROJECT_DIR = os.environ.get("CLAUDE_PROJECT_DIR", "/projects/elohim")
POLICY_FILE = os.path.join(PROJECT_DIR, ".claude", "epr-meta", "policies.yaml")
ACTORS_FILE = os.path.join(PROJECT_DIR, ".eprfs", "status", "actors.jsonl")

POLICY_ID = "destructive-git-requires-tier"
POLICY_VERSION = 1
POLICY_REF = f"{POLICY_ID}@{POLICY_VERSION}"

# Cheap pre-filter: only a Bash command that mentions `git` or `rm` can possibly match a
# destructive-git pattern, so everything else returns before touching policies.yaml at all
# (mirrors cargo-disk-guard.py's `"cargo" not in command` early bailout).
_PRE_FILTER = re.compile(r"git|rm")

_WS_RE = re.compile(r"\s+")


def _normalize(command: str) -> str:
    """Collapse whitespace runs to one space, strip ends."""
    return _WS_RE.sub(" ", command or "").strip()


def _pattern_regex(pattern: str) -> "re.Pattern[str]":
    """One compiled matcher for one declared pattern, matched as a substring of the
    whitespace-normalised command, ANY position.

    A pattern ending in a bare `.` is a path-argument pattern (`git checkout .`, `git restore .`,
    `rm -rf .`) and needs a word boundary immediately after the dot: `rm -rf .` must match
    `rm -rf .` and `rm -rf ./`, but never `rm -rf ./target` — the dot may be followed by nothing,
    whitespace, or exactly one `/` with nothing after it, never by another path segment.
    """
    if pattern.endswith("."):
        return re.compile(re.escape(pattern) + r"/?(?=$|\s)")
    return re.compile(re.escape(pattern))


def _matches(command: str, patterns: list) -> "str | None":
    norm = _normalize(command)
    for p in patterns:
        if isinstance(p, str) and p and _pattern_regex(p).search(norm):
            return p
    return None


def _load_policy_row():
    """(`{patterns, tier_order, tier_floor, unknown_tier, remedy}`, None) on success, or
    (None, reason) — never raises. Reads the row directly (not the full `.epr-meta` compose-gate
    machinery in `_lib.epr_meta.load_policies`, which validates fields this Bash-command
    predicate does not carry, e.g. a file-write `scope`) — this hook IS the consumer, so it owns
    its own minimal, honest validation."""
    try:
        import yaml
    except Exception as e:  # pragma: no cover - PyYAML is vendored in this workspace
        return None, f"PyYAML unavailable ({e!r})"
    try:
        with open(POLICY_FILE) as f:
            data = yaml.safe_load(f) or {}
    except Exception as e:
        return None, f"cannot read/parse {POLICY_FILE}: {e!r}"
    if not isinstance(data, dict) or data.get("epr-meta-policies-version") != 1:
        return None, f"{POLICY_FILE} missing/invalid `epr-meta-policies-version`"
    row = None
    for candidate in data.get("policies") or []:
        if (
            isinstance(candidate, dict)
            and candidate.get("id") == POLICY_ID
            and candidate.get("version") == POLICY_VERSION
            and candidate.get("status") != "superseded"
        ):
            row = candidate
            break
    if row is None:
        return None, f"no active row `{POLICY_REF}` in {POLICY_FILE}"
    if row.get("class") != "deny":
        return None, f"policy `{POLICY_REF}` is not `class: deny` (got {row.get('class')!r})"
    params = row.get("parameters")
    if not isinstance(params, dict):
        return None, f"policy `{POLICY_REF}` missing `parameters` block"
    patterns = params.get("patterns")
    tier_order = params.get("tier-order")
    tier_floor = params.get("tier-floor")
    if not isinstance(patterns, list) or not patterns:
        return None, f"policy `{POLICY_REF}` `parameters.patterns` missing or empty"
    if not isinstance(tier_order, list) or not tier_order:
        return None, f"policy `{POLICY_REF}` `parameters.tier-order` missing or empty"
    if not isinstance(tier_floor, str) or tier_floor not in tier_order:
        return None, (
            f"policy `{POLICY_REF}` `parameters.tier-floor` missing or not present in "
            f"`parameters.tier-order`"
        )
    return {
        "patterns": patterns,
        "tier_order": tier_order,
        "tier_floor": tier_floor,
        "unknown_tier": params.get("unknown-tier", "deny"),
        "remedy": params.get("remedy", ""),
    }, None


def _latest_claim(session_id: str) -> "str | None":
    """The `claimed` string (`agent:<role>@<model>`) of the LAST claim record in the actor
    sidecar for `session_id`, append-order (claims stack; latest wins) — mirrors
    `ActorStore::current_for` (elohim/epr-rea/src/actor.rs). Missing/unreadable file or no claim
    for this session -> None (honest absence, never an error)."""
    try:
        with open(ACTORS_FILE) as f:
            lines = f.readlines()
    except Exception:
        return None
    latest = None
    for line in lines:
        line = line.strip()
        if not line:
            continue
        try:
            obj = json.loads(line)
        except Exception:
            continue
        record = obj.get("record") if isinstance(obj, dict) else None
        if not isinstance(record, dict) or record.get("kind") != "claim":
            continue
        if record.get("session") != session_id:
            continue
        claimed = record.get("claimed")
        if isinstance(claimed, str) and claimed:
            latest = claimed
    return latest


def _resolve_tier(session_id: str) -> str:
    """`CLAUDE_MODEL`/`ANTHROPIC_MODEL` env -> the actor sidecar's latest claim for
    `session_id` -> `unknown`."""
    env_tier = os.environ.get("CLAUDE_MODEL") or os.environ.get("ANTHROPIC_MODEL")
    if env_tier and env_tier.strip():
        return env_tier.strip()
    if session_id:
        claimed = _latest_claim(session_id)
        if claimed:
            _, _, model = claimed.partition("@")
            if model:
                return model
    return "unknown"


def deny(reason: str):
    print(json.dumps({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "deny",
            "permissionDecisionReason": reason,
        }
    }))
    sys.exit(0)


def skip(reason: str):
    print(f"capability-tier-gate: skipped — {reason}", file=sys.stderr)
    sys.exit(0)


def main():
    data = json.load(sys.stdin)
    if data.get("tool_name") != "Bash":
        return
    command = (data.get("tool_input") or {}).get("command", "")
    if not command or not _PRE_FILTER.search(command):
        return

    row, err = _load_policy_row()
    if err:
        skip(err)
        return  # unreachable (skip() exits); kept for readability under test/import

    matched = _matches(command, row["patterns"])
    if not matched:
        return

    session_id = os.environ.get("CLAUDE_SESSION_ID", "")
    tier = _resolve_tier(session_id)

    tier_order = row["tier_order"]
    floor = row["tier_floor"]
    floor_idx = tier_order.index(floor)
    tier_idx = tier_order.index(tier) if tier in tier_order else None

    if tier_idx is not None and tier_idx >= floor_idx:
        return  # at/above the declared floor: allowed, silently

    # Below the floor, OR an unresolved/unrecognised tier — `unknown-tier: deny` is fail-closed
    # by design: an unclaimed or unrecognised actor is treated as below the floor, never assumed
    # safe.
    deny(
        f"DESTRUCTIVE GIT ({POLICY_REF}): command matches declared pattern `{matched}`. "
        f"Resolved tier: {tier} (floor: {floor}; tier-order: {tier_order}). {row['remedy']}"
    )


if __name__ == "__main__":
    try:
        main()
        sys.exit(0)
    except Exception:
        # Fail-open: a guard bug must never block development.
        sys.exit(0)
