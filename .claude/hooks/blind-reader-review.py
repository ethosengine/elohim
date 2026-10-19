#!/usr/bin/env python3
"""Post-write adapter for directory-local blind-reader review rules.

Each .epr-meta manifest decides which authored files need a blind read and
selects a review profile. This hook only makes that obligation visible after a
successful Edit/Write; it never pretends to perform or certify the semantic
review itself.

Hook Type: PostToolUse
Matcher: Edit|Write
"""

from __future__ import annotations

import json
import os
import sys
from pathlib import Path


AGENT_ID = "blind-reader"


def _repo_root() -> Path | None:
    project_dir = os.environ.get("CLAUDE_PROJECT_DIR")
    return Path(project_dir).resolve() if project_dir else None


def _target_path(file_path: str, repo: Path) -> Path:
    target = Path(file_path)
    if not target.is_absolute():
        target = repo / target
    return target.resolve()


def _matching_reviews(target: Path, repo: Path) -> list[tuple[str, str, dict]]:
    try:
        target.relative_to(repo)
    except ValueError:
        return []

    scripts_dir = repo / ".claude" / "scripts"
    if str(scripts_dir) not in sys.path:
        sys.path.insert(0, str(scripts_dir))

    try:
        from _lib.epr_meta import _matches_when, resolve_write

        content = target.read_text(errors="replace")
        write = {
            "path": str(target),
            "content": content,
            "is_new": False,
            "is_new_subdir": False,
        }
        result = resolve_write(target, write, repo)
    except (ImportError, OSError):
        return []

    reviews: list[tuple[str, str, dict]] = []
    rules = result.get("merged", {}).get("rules", {})
    for rule_id, rule in rules.items():
        if not isinstance(rule, dict):
            continue
        route = rule.get("route-to", {})
        if route.get("dest") != AGENT_ID or not _matches_when(rule.get("when", {}), write):
            continue
        parameters = rule.get("parameters", {})
        profile = parameters.get("review-profile", "general")
        reviews.append((rule_id, profile, parameters.get("variance") or {}))
    return reviews


def _display(target: Path, repo: Path):
    try:
        return target.relative_to(repo)
    except ValueError:
        return target


def _context(target: Path, repo: Path, reviews: list[tuple[str, str, dict]]) -> str:
    display_path = _display(target, repo)
    rules = ", ".join(f"`{rule_id}`" for rule_id, _, _ in reviews)
    profiles = ", ".join(f"`{profile}`" for _, profile, _ in reviews)
    return (
        f"[BLIND-READER REVIEW] `{display_path}` carries .epr-meta rule(s) {rules}. "
        f"After this authoring pass is complete, dispatch a NEW fresh-context `{AGENT_ID}` with "
        f"only this document path and review profile(s) {profiles}—no conversation, plan, diff, "
        "incident history, sibling documents, glossary, or author explanation. Revise from the "
        "full interpretability/coherence findings and repeat with another blind reader until "
        "READY, or name findings the operator explicitly defers. Defer dispatch until the "
        "completed authoring pass; do not spawn one per edit."
    )


def _floor_context(target: Path, repo: Path, held: list[tuple[str, str, dict]]) -> str:
    """The obligation at the rule's declared floor, for a write a covenant clause holds."""
    display_path = _display(target, repo)
    lines = []
    for rule_id, profile, check in held:
        lines.append(
            f"[BLIND-READER REVIEW · COVENANT] `{display_path}`: rule `{rule_id}` is held at its "
            f"variance floor under covenant clause {check.get('clause')} (granted by "
            f"{check.get('party') or 'the steward of record'} for this session and these paths). "
            f"Owed: {check.get('floor')}. The read is still a NEW fresh-context `{AGENT_ID}` with "
            f"only this path and profile `{profile}`. Be careful: the floor trusts this session's "
            "declared purpose, so keep implementation detail out of the story. Record the read's "
            "per-class counts and the clause id where the work reports."
        )
    return "\n".join(lines)


def _owed_suffix(check: dict, in_subagent: bool) -> str:
    """What to say about a variable rule whose floor is NOT held for this write."""
    if check.get("status") == "unavailable":
        return f" (Covenant unreadable: {check.get('reason')}. The full rule applies.)"
    if in_subagent:
        return ""
    return (" This rule declares a variance floor. If this session's declared purpose is authoring "
            "these stories, offer the steward of record that floor as ONE question (`epr covenant "
            "offer` lists it) and record their answer; never take the floor yourself.")


def _witness_floor(repo: Path, target: Path, rule_id: str, check: dict,
                   agent_type: str | None, agent_id: str | None) -> None:
    """A lowered obligation is counted, not felt: one ledger row per write held at a floor."""
    try:
        from _lib.epr_meta import witness

        # COVENANT_WITNESS_ROOT exists for the hook's tests, so fixtures never land in the
        # tracked ledger; in a session it is unset and rows go to this repository's ledger.
        ledger_root = Path(os.environ.get("COVENANT_WITNESS_ROOT") or repo)
        witness(ledger_root, runtime="claude-code", gate="blind-reader-review",
                subject=str(_display(target, repo)), decision="permit", cls="inject",
                rule_id=rule_id,
                checks=[f"variance: {check.get('clause')}",
                        f"party: {check.get('party')}",
                        f"agent: {agent_type or 'main-session'}"
                        + (f" {agent_id}" if agent_id else "")])
    except Exception:  # noqa: BLE001 — witnessing must never break the hook
        pass


def main() -> int:
    try:
        payload = json.load(sys.stdin)
    except (json.JSONDecodeError, ValueError):
        return 0

    file_path = (payload.get("tool_input", {}) or {}).get("file_path", "")
    repo = _repo_root()
    if not file_path or repo is None:
        return 0

    target = _target_path(file_path, repo)
    reviews = _matching_reviews(target, repo)
    if not reviews:
        return 0

    # A rule that declares a variance asks this session's covenant (`epr covenant check`, keyed by
    # the session id a subagent inherits) whether the steward of record granted its floor here.
    session = payload.get("session_id") or os.environ.get("CLAUDE_CODE_SESSION_ID")
    agent_id = payload.get("agent_id")
    agent_type = payload.get("agent_type")
    held: list[tuple[str, str, dict]] = []
    owed: list[tuple[str, str, dict]] = []
    suffixes: list[str] = []
    for rule_id, profile, variance in reviews:
        if not variance:
            owed.append((rule_id, profile, variance))
            continue
        from _lib import epr_client

        check = epr_client.covenant(repo, "check", session, "--rule", rule_id,
                                    "--path", str(_display(target, repo)))
        if check.get("status") == "held":
            held.append((rule_id, profile, check))
            _witness_floor(repo, target, rule_id, check, agent_type, agent_id)
        else:
            owed.append((rule_id, profile, variance))
            suffixes.append(_owed_suffix(check, in_subagent=bool(agent_id)))

    parts = []
    if owed:
        parts.append(_context(target, repo, owed) + "".join(dict.fromkeys(suffixes)))
    if held:
        parts.append(_floor_context(target, repo, held))
    print(
        json.dumps(
            {
                "hookSpecificOutput": {
                    "hookEventName": "PostToolUse",
                    "additionalContext": "\n".join(parts),
                }
            }
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
