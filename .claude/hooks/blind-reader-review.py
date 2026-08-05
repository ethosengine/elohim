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


def _matching_reviews(target: Path, repo: Path) -> list[tuple[str, str]]:
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

    reviews: list[tuple[str, str]] = []
    rules = result.get("merged", {}).get("rules", {})
    for rule_id, rule in rules.items():
        if not isinstance(rule, dict):
            continue
        route = rule.get("route-to", {})
        if route.get("dest") != AGENT_ID or not _matches_when(rule.get("when", {}), write):
            continue
        parameters = rule.get("parameters", {})
        profile = parameters.get("review-profile", "general")
        reviews.append((rule_id, profile))
    return reviews


def _context(target: Path, repo: Path, reviews: list[tuple[str, str]]) -> str:
    try:
        display_path = target.relative_to(repo)
    except ValueError:
        display_path = target
    rules = ", ".join(f"`{rule_id}`" for rule_id, _ in reviews)
    profiles = ", ".join(f"`{profile}`" for _, profile in reviews)
    return (
        f"[BLIND-READER REVIEW] `{display_path}` carries .epr-meta rule(s) {rules}. "
        f"After this authoring pass is complete, dispatch a NEW fresh-context `{AGENT_ID}` with "
        f"only this document path and review profile(s) {profiles}—no conversation, plan, diff, "
        "incident history, sibling documents, glossary, or author explanation. Revise from the "
        "full interpretability/coherence findings and repeat with another blind reader until "
        "READY, or name findings the operator explicitly defers. Defer dispatch until the "
        "completed authoring pass; do not spawn one per edit."
    )


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

    print(
        json.dumps(
            {
                "hookSpecificOutput": {
                    "hookEventName": "PostToolUse",
                    "additionalContext": _context(target, repo, reviews),
                }
            }
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
