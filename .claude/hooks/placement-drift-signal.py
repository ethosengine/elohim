#!/usr/bin/env python3
"""
Placement-Drift Signal Accumulator  (decompose-due tripwire)

PostToolUse hook (matcher: Edit|Write). Fast accumulator — when an edit lands a
TERMINAL status (`landed | superseded | abandoned`, with a `landed_commit:` for
the landed case) into a doc that still lives plan-shaped in an ACTIVE home
(genesis/docs/superpowers/{specs,plans}, genesis/docs/plans), that is PLACEMENT
DRIFT: the doc should DECOMPOSE to zero residue, not sit in the live tree. The
hook QUEUES the doc into a `decompose-due` accumulator. It never moves or mutates
the doc — mutations are operator-gated (decompose-self / cleanup-apply). All
judgment is deferred to a script run at ceremony / SessionStart.

This is the BACK fire point's tripwire from the Spec/Plan Compaction Loop
(genesis/docs/superpowers/specs/2026-06-02-spec-plan-compaction-loop-design.md
§5.1 / §10.1). It mirrors claude-md-drift-signal.py + memory-coherence-signal.py:
single-digit-ms cheap path, _lib bootstrap, best-effort, fail-open, never blocks.

The decompose-due count surfaces at SessionStart through the existing budget
headline (placement-audit.py --headline) as "N plans past-due to decompose".

Storage: .claude/memory-kit/placement-drift.json (schema_version: 1)

Hook Type: PostToolUse
Matcher: Edit|Write
"""

from __future__ import annotations

# The intervenor's removal condition (Meadows' shifting-the-burden trap;
# counted by _lib/intervenor_census.py). A condition, never a date.
RETIRE_WHEN = (
    "when decompose-to-zero-residue runs as part of landing a terminal status rather than as a "
    "later sweep — the accumulator exists because the decompose lags the landing, which is the "
    "same generation-outruns-absorption shape the doc-corpus stock measures. It retires when "
    "that stock holds in dynamic equilibrium (emission/absorption <= 1.0, turnover bounded) for "
    "a quarter."
)

import json
import os
import re
import sys
import time
from pathlib import Path

# Bootstrap: locate .claude/scripts/_lib by walking up
_here = Path(__file__).resolve()
for _ in range(8):
    if (_here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(_here / ".claude" / "scripts"))
        break
    _here = _here.parent
from _lib import store as _store  # noqa: E402

# ACTIVE homes (repo-relative dir prefixes) — must mirror placement-audit.py's
# ACTIVE_HOMES surfaces. A terminal-status doc here is plan-shaped residue that
# should have dissolved.
ACTIVE_HOME_PREFIXES = (
    "genesis/docs/superpowers/specs/",
    "genesis/docs/superpowers/plans/",
    "genesis/docs/plans/",
)

# Terminal status words. The spec asks one question (§10.1): does this ACTIVE-home
# doc carry a terminal status (landed | superseded | abandoned) yet still live
# plan-shaped? We mirror placement-audit.py's DEAD_WORDS (superseded family) and
# the landed family so the hook's classification matches the audit's verdict.
DEAD_WORDS = {"superseded", "abandoned", "cancelled", "canceled", "deprecated", "retired"}
LANDED_WORDS = {"landed", "stable", "done", "complete", "completed", "shipped", "accepted",
                "latest-stable"}
TERMINAL_WORDS = DEAD_WORDS | LANDED_WORDS

# Frontmatter `status:` and a markdown `**Status:**` fallback (mirrors placement-audit.py).
FM_STATUS_RE = re.compile(r"^status:\s*(.+?)\s*$", re.M | re.I)
MD_STATUS_RE = re.compile(r"^\*\*Status:\*\*\s*(.+?)\s*$", re.M)

DEFAULT_THRESHOLD = 1  # any past-due doc is worth surfacing


def repo_root_from_env() -> Path | None:
    pd = os.environ.get("CLAUDE_PROJECT_DIR")
    return Path(pd).resolve() if pd else None


def drift_store_path(repo_root: Path) -> Path:
    return repo_root / ".claude" / "memory-kit" / "placement-drift.json"


def in_active_home(rel: str) -> bool:
    r = rel.replace("\\", "/").lstrip("./")
    return any(r.startswith(p) for p in ACTIVE_HOME_PREFIXES)


def first_word(raw: str) -> str:
    parts = (raw or "").strip().lower().split()
    if not parts:
        return ""
    return parts[0].strip(":*\"'").strip()


def terminal_status_of(text: str) -> str | None:
    """Return the terminal status word if the doc carries one (frontmatter wins),
    else None. Reads `status:` frontmatter, falling back to `**Status:**`."""
    raw = ""
    m = FM_STATUS_RE.search(text)
    if m:
        raw = m.group(1)
    if not raw:
        md = MD_STATUS_RE.search(text)
        if md:
            raw = md.group(1)
    w = first_word(raw)
    return w if w in TERMINAL_WORDS else None


def main() -> int:
    try:
        data = json.load(sys.stdin)
    except (json.JSONDecodeError, ValueError):
        return 0

    edited = (data.get("tool_input", {}) or {}).get("file_path") or ""
    if not edited:
        return 0
    if not edited.endswith(".md"):
        return 0

    repo = repo_root_from_env()
    if not repo:
        return 0

    ep = Path(edited)
    if not ep.is_absolute():
        ep = repo / ep
    try:
        rel = str(ep.resolve().relative_to(repo))
    except (ValueError, OSError):
        return 0

    if not in_active_home(rel):
        return 0
    if Path(rel).name in ("INDEX.md", "CLAUDE.md", "README.md"):
        return 0

    try:
        text = ep.read_text(errors="replace")
    except OSError:
        return 0

    status = terminal_status_of(text)

    store_path = drift_store_path(repo)
    now_iso = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())

    def _mutate(data) -> dict:
        store = data if isinstance(data, dict) else {
            "schema_version": 1, "threshold": DEFAULT_THRESHOLD, "due": {}}
        store.setdefault("schema_version", 1)
        store.setdefault("threshold", DEFAULT_THRESHOLD)
        store.setdefault("due", {})
        if status is None:
            # Status was edited back to non-terminal (e.g. re-opened) → clear any queued
            # entry so the headline does not over-report. Cheap self-healing path.
            store["due"].pop(rel, None)
            return store
        entry = store["due"].get(rel) or {"hits": 0, "status": status, "first_signal_at": now_iso}
        entry["hits"] = entry.get("hits", 0) + 1
        entry["status"] = status
        entry["last_signal_at"] = now_iso
        entry.setdefault("first_signal_at", now_iso)
        store["due"][rel] = entry
        return store

    # Serialize the read-modify-write so concurrent sessions don't lose each other's updates.
    _store.locked_update(
        store_path, _mutate,
        default={"schema_version": 1, "threshold": DEFAULT_THRESHOLD, "due": {}},
    )
    return 0  # hooks are best-effort; never block the tool call


if __name__ == "__main__":
    sys.exit(main())
