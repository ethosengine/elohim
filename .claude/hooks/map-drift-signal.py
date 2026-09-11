#!/usr/bin/env python3
"""
Map-Currency-Drift Signal Accumulator  (MAP-stale tripwire)

PostToolUse hook (matcher: Edit|Write). Fast accumulator — when an ARCHITECTURE
SEED under genesis/docs/content/elohim-protocol/architecture/*.md (EXCLUDING the
two map artifacts INDEX.md and MAP.md) is created or changed, that seed may have
moved the territory the developer's WALK describes. The hook increments a
`map-currency-drift` accumulator: it signals "MAP.md may be stale vs the seeds."
It never moves or mutates the doc — the MAP refresh is operator/cartographer-gated
(the LEGIBILITY/PATH discipline's ceremony). All judgment is deferred to the
script that reads the accumulator (placement-audit.py --headline currency line).

This is the LEGIBILITY/PATH discipline's in-flight tripwire — the standing,
continuously-maintained twin of the BACK fire point's decompose tripwire. It
MIRRORS placement-drift-signal.py exactly: single-digit-ms cheap path, _lib
bootstrap, best-effort, fail-open, never blocks the tool call.

Self-healing: when MAP.md ITSELF is edited (the walk just got refreshed), the
accumulator is reset to zero — the seeds and the walk are back in sync as far as
this cheap signal can tell. The currency line then reads "0 seed(s) changed".

The drift count surfaces at SessionStart through the existing budget headline
(placement-audit.py --headline) as a "path:" currency line:
    "path: N seed(s) changed since MAP update | roadmap: refreshed <date>"

Storage: ONE thing — a fold via `epr flow note --kind observation --measure
map-currency-drift@1`, routed to `map-currency-drift-reset@1` when MAP.md itself is edited
(a bulk clear). The private JSON accumulator under `.claude/memory-kit/` was deleted with the
kit at station six round (b) (2026-09-11); the accumulated count is DERIVED from the folds by
map-currency-drift-ceiling@1's `derive: distinct-subjects-since-reset`.
The JSON is not a fallback: cleanup-pressure.py counts its `changed` collection, so the kit
still produces the accumulated number the SessionStart bridge folds. Station six deletes it.

Hook Type: PostToolUse
Matcher: Edit|Write
"""

from __future__ import annotations

# The intervenor's removal condition (Meadows' shifting-the-burden trap;
# counted by _lib/intervenor_census.py). A condition, never a date.
RETIRE_WHEN = (
    "when the MAP is PROJECTED from the architecture seeds rather than hand-maintained beside "
    "them — a derived map cannot go stale relative to its sources, so a staleness tripwire over "
    "it has no subject."
)

import json
import os
import sys
from pathlib import Path

# Bootstrap: locate .claude/scripts/_lib by walking up
_here = Path(__file__).resolve()
for _ in range(8):
    if (_here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(_here / ".claude" / "scripts"))
        break
    _here = _here.parent
sys.path.insert(0, str(Path(__file__).resolve().parent))
import _observation as _obs  # noqa: E402  (structured-observation emitter; fail-open, never blocks)

# The CANONICAL architecture surface — the seeds the MAP walks over. Repo-relative
# dir prefix. A *.md created/changed here (other than the two map artifacts) is a
# potential currency drift between the seeds (territory) and MAP.md (the walk).
# Must mirror placement-audit.py's SURFACES["CANONICAL"].
ARCHITECTURE_PREFIX = "genesis/docs/content/elohim-protocol/architecture/"

# The two MAP artifacts themselves are NOT seeds — they ARE the map. Editing them
# is the refresh, not the drift. INDEX.md = the graph; MAP.md = the walk.
MAP_ARTIFACTS = {"INDEX.md", "MAP.md"}
# The walk-defining artifact whose edit RESETS the accumulator (the walk is now current).
WALK_ARTIFACT = "MAP.md"

# The threshold is DECLARED, not kept here: map-currency-drift-ceiling@1
# in .claude/epr-meta/measures.yaml carries `hard: 1` — any seed changed since the last MAP refresh is worth surfacing.


def repo_root_from_env() -> Path | None:
    pd = os.environ.get("CLAUDE_PROJECT_DIR")
    return Path(pd).resolve() if pd else None


def in_architecture_surface(rel: str) -> bool:
    r = rel.replace("\\", "/").lstrip("./")
    # Direct children of the architecture dir only — subdirs (applications/, horizons/)
    # are proof galleries / horizon notes, not the architecture seeds the MAP walks.
    if not r.startswith(ARCHITECTURE_PREFIX):
        return False
    tail = r[len(ARCHITECTURE_PREFIX):]
    return "/" not in tail  # no further path separator → a top-level seed


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

    if not in_architecture_surface(rel):
        return 0

    name = Path(rel).name

    # INDEX.md is graph maintenance, not a seed — it neither resets nor accumulates. Skip it
    # before touching the store (no read-modify-write needed, so no lock needed).
    if name in MAP_ARTIFACTS and name != WALK_ARTIFACT:
        return 0

    # The bound lives in .claude/epr-meta (map-currency-drift@1). `1` = a seed moved the
    # territory the walk describes; `0` on MAP.md = the walk was refreshed, which is a BULK
    # CLEAR of everything accumulated since the last walk. The zero is not sent as
    # a zero: `_observation._BULK_CLEAR_ON_ZERO` routes it to `map-currency-drift-reset@1` on
    # subject `.`, because a per-subject zero on MAP.md would leave every other accumulated seed
    # counted forever. The routing lives in ONE place, the emitter, so every hook that clears a
    # collection expresses it the same way; this hook just says what happened.
    _obs.emit("map-currency-drift@1", rel, 0 if name == WALK_ARTIFACT else 1,
              reason=("map currency: MAP.md walk refreshed" if name == WALK_ARTIFACT
                      else "map currency: architecture seed changed since the MAP walk"),
              env={"artifact": "walk" if name == WALK_ARTIFACT else "seed"},
              root=str(repo))

    return 0  # hooks are best-effort; never block the tool call


if __name__ == "__main__":
    sys.exit(main())
