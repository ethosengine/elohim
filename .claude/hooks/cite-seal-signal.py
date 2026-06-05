#!/usr/bin/env python3
"""
Cite-seal advisory (deterministic born-linked enforcement).

PostToolUse hook (matcher: Edit|Write). When a Write/Edit lands on a `.md` that is a CITE-GRAPH
MEMBER (doc-roots + gospel CLAUDE.mds — membership answered by _lib.managed_surfaces, the single
edit-time registry; hardcoding doc-roots here is what let the 2026-06-05 gospel episode through)
and it carries UN-SEALED cite debt, it nudges the agent to run `cite-gen --seal <doc>` — so a new
spec/plan/memory/gospel enters the graph content-addressed instead of depending on anyone
remembering the ceremony. This is the postHook half of the discipline; managed-surface-context.py
is the PRE half (discipline injected before the edit); the ceremony POST-steps (brainstorm/plan)
and end-of-sprint decompose are the proactive halves.

Un-sealed debt = either:
  - a legacy path-cite whose target is a doc-root .md that HAS an `id:` (a CITE-FORMAT-CANDIDATE —
    it could be a `slug | desc | fingerprint` envelope but is still a bare path), or
  - the doc has `cites:` but no `id:` of its own (can't be a stable cite target until sealed).

Non-blocking + cheap: no slug-index build (path-cites resolve directly against repo root; one stat +
small read per cite). Absent/odd frontmatter → silent no-op. Mirrors memory-coherence-signal.py's
trust-compute discipline: the hot path stays light, judgment (the actual seal) is the agent's.

Hook Type: PostToolUse   Matcher: Edit|Write
"""
from __future__ import annotations

import json
import os
import sys
from pathlib import Path

_here = Path(__file__).resolve()
for _ in range(8):
    if (_here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(_here / ".claude" / "scripts"))
        break
    _here = _here.parent
from _lib import cite_graph as cg  # noqa: E402
from _lib import frontmatter as fm  # noqa: E402
from _lib import managed_surfaces as ms  # noqa: E402


def _unsealed_debt(doc: Path, repo: Path) -> int:
    """Count un-sealed cite issues on this doc (legacy doc-cites-with-id, or cites-without-own-id)."""
    f = fm.parse_file(doc)
    cites = cg.parse_cites(f)
    if not cites:
        return 0
    debt = 0
    has_id = bool(f.get("id"))
    for c in cites:
        if not c.get("legacy_path"):
            continue
        ref = c["ref"]
        if not ref.endswith(".md"):
            continue  # code/external path-cite — legitimately stays legacy
        tgt = (repo / ref) if not Path(ref).is_absolute() else Path(ref)
        try:
            if tgt.is_file() and fm.parse_file(tgt).get("id"):
                debt += 1  # CITE-FORMAT-CANDIDATE: a path-cite to an id-bearing doc
        except OSError:
            continue
    # has cites but no id: of its own → can't be cited stably until sealed
    if not has_id:
        debt += 1
    return debt


def main() -> int:
    try:
        data = json.load(sys.stdin)
    except (json.JSONDecodeError, ValueError):
        return 0
    edited = (data.get("tool_input", {}) or {}).get("file_path") or ""
    pd = os.environ.get("CLAUDE_PROJECT_DIR")
    if not edited or not pd or not edited.endswith(".md"):
        return 0
    repo = Path(pd).resolve()
    try:
        ep = Path(edited)
        rel = str(ep.relative_to(repo)) if ep.is_absolute() else str(ep)
    except (ValueError, OSError):
        rel = edited
    try:
        if not ms.in_cite_graph(rel, repo):
            return 0
    except Exception:
        return 0
    doc = repo / rel
    if not doc.is_file():
        return 0
    try:
        debt = _unsealed_debt(doc, repo)
    except Exception:
        return 0
    if debt <= 0:
        return 0
    msg = (f"[cite-seal] {rel} has {debt} un-sealed cite issue(s) (legacy path-cite to an id-bearing doc, "
           f"or no id: yet). Make it born-linked: `python3 .claude/scripts/memory-kit/cite-gen.py --seal {rel}` "
           f"(assigns id, converts path-cites → slug|desc|fingerprint envelopes, verifies). Then author "
           f"relationship descriptions for any title-default cites with cite-describe.py.")
    print(json.dumps({"hookSpecificOutput": {"hookEventName": "PostToolUse", "additionalContext": msg}}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
