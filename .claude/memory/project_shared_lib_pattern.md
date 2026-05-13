---
name: Shared lib pattern — _lib/ for .claude/* Python tools
description: Pure-stdlib helpers at .claude/scripts/_lib/ for scripts and hooks. Bootstrap-by-walk-up imports work from any depth. Discipline: only extract when 3+ callers share the same pattern. New scripts use it from the start; older scripts migrate when touched.
type: project
---

`.claude/scripts/_lib/` is a Python package of pure-stdlib helpers shared by scripts (under `.claude/scripts/*/`) and hooks (under `.claude/hooks/`). Created 2026-05-13 alongside the `claude-md-review` ceremony.

**Current modules**:
- `_lib.paths` — `repo_root_from_file(__file__)` (robust replacement for `Path(__file__).resolve().parents[N]`), `reports_root()`, `reports_dir_for_today()`, `memory_dir()`
- `_lib.frontmatter` — minimal YAML-frontmatter parser for memory entries (no PyYAML dependency)
- `_lib.store` — best-effort JSON load/save with safe defaults (used by accumulator hooks where filesystem errors must not crash the tool call)

**Bootstrap pattern** (6 lines, works from any script or hook):
```python
from pathlib import Path
import sys
_here = Path(__file__).resolve()
for _ in range(8):
    if (_here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(_here / ".claude" / "scripts"))
        break
    _here = _here.parent
from _lib import paths, store  # noqa: E402
```

**Why:** The `Path(__file__).resolve().parents[N]` pattern was in 8+ files and the index `N` varied between 2 and 3 depending on script depth — error-prone. The bootstrap walks up looking for `.claude/scripts/_lib`, working regardless of where the script lives.

**How to apply:**
- New scripts at `.claude/scripts/*/` and hooks at `.claude/hooks/` use `_lib` from the start
- Older scripts (cleanup-scan, path-update-scan, dedupe-memory-scan, sprint-distill, plan-status, skill-audit, memory-review) migrate when touched for other reasons — not as a dedicated refactor sprint
- Resist scope creep in `_lib` itself: only extract when 3+ callers share the same pattern. Two-caller patterns stay inline.
- Pure-stdlib only — no PyYAML, no requests, no third-party deps. If a helper would require a dep, it doesn't belong in `_lib`
