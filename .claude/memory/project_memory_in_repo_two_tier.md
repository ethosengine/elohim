---
name: Memory in repo — two-tier storage (project / personal)
description: Project memory lives in <repo>/.claude/memory/ (team-shareable, git-tracked, PVC-recoverable). The personal slot at .claude-config/projects/<slug>/memory/ is a symlink to the primary, kept as the harness-conventional path. Migrated 2026-05-13.
type: project
---

Project memory must be **team-shareable and recoverable from git** — a corrupted PVC should not erase the architectural insights, design rationale, and lessons-learned that serve the project's goals. Memory in service of the project goals belongs in the repository.

**Two-tier storage** (as of 2026-05-13):

| Tier | Path | Tracked? | Lifetime |
|---|---|---|---|
| **Project** (primary) | `/projects/elohim/.claude/memory/` | git-tracked in this repo | as long as the repo exists |
| **Personal** (harness slot) | `/projects/.claude-config/projects/-projects-elohim/memory/` | symlink → primary | survives only with `.claude-config/` |

**Mechanism**: the personal slot is a symlink to the primary. Harness writes go through the symlink and land in the repo. Both paths resolve to the same files. The hook + memory-review explicitly prefer the primary, falling back to the personal slot if the primary doesn't exist (e.g., fresh clone before symlink restoration).

**Recovery on fresh PVC**:
1. Clone the repo — `.claude/memory/` is restored from git, ~165 entries intact
2. Recreate the symlink (one command):
   ```bash
   ln -s /projects/elohim/.claude/memory /projects/.claude-config/projects/-projects-elohim/memory
   ```
3. Memory layer is fully functional again

**Why this matters:**
- PVC corruption / dev-environment loss does not erase project knowledge
- Team members get architectural context by cloning the repo
- Auto-memory's writes flow into git automatically (any new entries get tracked)
- The harness convention (`.claude-config/`) still works via the symlink — no change needed in harness behavior

**How to apply:**
- Future memory entries (project_*, feedback_*, reference_*) belong in this location
- User-profile observations (user_*) could optionally stay personal at `.claude-config/` if they shouldn't be team-visible — break the symlink for that file by writing directly under .claude-config/. (We have ~0 such entries today; the corpus is overwhelmingly project knowledge.)
- Operational artifacts (dated reports under `.claude/memory-kit/<date>/`, archive under `.claude/archive/<date>/`) are tracked as project history — they show the corpus's evolution and serve future historian work
- Do NOT put routing rules or always-loaded conventions in MEMORY.md — those still belong in CLAUDE.md per the Pawel Huryn article's discipline
